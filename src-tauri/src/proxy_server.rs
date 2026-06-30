//! actix-web 高性能瓦片代理服务器。
//!
//! 替代原有的手写 TCP 服务器，提供：
//! - 多 worker 线程池，处理 CesiumJS 的 30-80 个并发瓦片请求
//! - L1 (moka 内存) + L2 (文件系统) 两级缓存
//! - 缓存未命中时通过 TileDownloader 从 CDN 下载
//! - /health 健康检查端点
//! - 静态文件服务 (cesium, assets, terrain 等非瓦片资源)
//! - 所有响应包含缓存头
//!
//! 缓存实例由调用方（lib.rs）创建并通过 Arc 共享，
//! 确保 proxy_server 和 Tauri 管理命令操作同一组缓存。

use actix_cors::Cors;
use actix_web::{get, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::l1_cache::L1Cache;
use crate::l2_cache::L2Cache;
use crate::tile_downloader::TileDownloader;

/// 服务器共享状态 — 缓存实例由外部传入。
struct AppState {
    l1: Arc<L1Cache>,
    l2: Arc<L2Cache>,
    downloader: Arc<TileDownloader>,
    data_dir: PathBuf,
}

/// 启动 actix-web 代理服务器。
/// 在独立 std::thread 中运行，自带 tokio runtime。
/// 缓存实例由调用方创建并通过 Arc 共享。
pub fn start_proxy_server(
    data_dir: PathBuf,
    l1: Arc<L1Cache>,
    l2: Arc<L2Cache>,
    downloader: Arc<TileDownloader>,
    shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("actix-proxy-server".into())
        .spawn(move || {
            let rt = actix_rt::System::new();
            rt.block_on(async move {
                let app_state = web::Data::new(AppState {
                    l1: Arc::clone(&l1),
                    l2: Arc::clone(&l2),
                    downloader: Arc::clone(&downloader),
                    data_dir: data_dir.clone(),
                });

                let workers = std::thread::available_parallelism()
                    .map(|n| n.get().min(8))
                    .unwrap_or(4);

                let server = match HttpServer::new(move || {
                    App::new()
                        .app_data(app_state.clone())
                        // ponytail: 全局 CORS。前端在 dev 模式 origin 是 http://127.0.0.1:1430，
                        // 打包后是 http://tauri.localhost，都与代理 127.0.0.1:21337 跨域。
                        // permissive 对所有源放行——代理只读本地缓存瓦片，无凭证、无敏感数据。
                        .wrap(Cors::permissive())
                        .wrap(middleware::Logger::default())
                        .service(health)
                        .service(serve_tile)
                        .service(serve_pmtiles)
                        .service(serve_static)
                })
                .workers(workers)
                .bind("127.0.0.1:21337")
                {
                    Ok(s) => s,
                    Err(e) => {
                        crate::logger::log(
                            "proxy_server",
                            "ERROR",
                            &format!("绑定 21337 失败: {e}"),
                        );
                        crate::logger::log("proxy_server", "ERROR", "端口被占用，代理服务器未启动");
                        return;
                    }
                }
                .run();

                crate::logger::log(
                    "proxy_server",
                    "INFO",
                    &format!(
                        "actix-web 服务器已启动 (127.0.0.1:21337, {} workers)",
                        workers
                    ),
                );

                // 优雅关闭 — 轮询 shutdown 标志
                let handle = server.handle();
                let shutdown_clone = shutdown.clone();
                tokio::spawn(async move {
                    loop {
                        if shutdown_clone.load(Ordering::Relaxed) {
                            crate::logger::log(
                                "proxy_server",
                                "INFO",
                                "收到关闭信号，停止服务器...",
                            );
                            handle.stop(true).await;
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                });

                let _ = server.await;
                crate::logger::log("proxy_server", "INFO", "actix-web 服务器已停止");
            });
        })
        .unwrap_or_else(|e| {
            crate::logger::log("proxy_server", "ERROR", &format!("无法创建代理线程: {e}"));
            // ponytail: return a dummy handle; caller logs the error
            std::thread::spawn(|| {})
        })
}

/// 健康检查端点。
#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache"))
        .body("OK")
}

/// 路径遍历核心检查：canonical_file 必须落在 canonical_root 内。
/// serve_static / serve_pmtiles 共用此判定，避免两处独立 starts_with
/// 各自演化导致一处改漏（安全检查重复是隐患）。
/// 调用方各自负责 canonicalize root/file 与失败时的定制响应。
fn is_within_root(canonical_root: &std::path::Path, canonical_file: &std::path::Path) -> bool {
    canonical_file.starts_with(canonical_root)
}

/// 安全检查：file_path 必须在 root 内（canonicalize + starts_with，防路径遍历）。
/// 成功返回 canonicalize 后的绝对路径；失败返回 500/403/404 响应。
/// serve_static 用此便捷封装；serve_pmtiles 因需定制 NotFound+log，
/// 直接用 is_within_root 保留自身响应语义。
fn validate_within_root(
    root: &std::path::Path,
    file_path: &std::path::Path,
) -> Result<PathBuf, HttpResponse> {
    let canonical_root = root
        .canonicalize()
        .map_err(|_| HttpResponse::InternalServerError().body("500"))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| HttpResponse::NotFound().body("404 Not Found"))?;
    if !is_within_root(&canonical_root, &canonical_file) {
        return Err(HttpResponse::Forbidden().body("403 Forbidden"));
    }
    Ok(canonical_file)
}

/// 瓦片代理端点 — 支持 china-tiles 路径。
/// 路径格式: /china-tiles/{z}/{x}/{y}.{ext}
#[get("/china-tiles/{z}/{x}/{y}.{ext}")]
async fn serve_tile(
    path: actix_web::web::Path<(u32, u32, u32, String)>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let (z, x, y, ext) = path.into_inner();
    let key = format!("{}/{}/{}.{}", z, x, y, ext);

    // L1 查询
    if let Some(data) = state.l1.get(&key) {
        return tile_response(data, &ext);
    }

    // L2 查询 — 无锁并发读，多 worker 直接命中磁盘缓存
    if let Some(data) = state.l2.get(&key) {
        state.l1.put(&key, data.clone());
        return tile_response(data, &ext);
    }

    // 下载
    if let Some(data) = state.downloader.download(z, x, y).await {
        let _ = state.l2.put(&key, &data);
        state.l1.put(&key, data.clone());
        return tile_response(data, &ext);
    }

    // 所有源失败 — 返回 404
    HttpResponse::NotFound()
        .insert_header(("Cache-Control", "no-cache"))
        .body("tile unavailable")
}

/// PMTiles 文件端点 — 支持 HTTP Range 请求。
/// 路径: /pmtiles/basemap.pmtiles
/// PMTiles 协议靠 Range 随机读取 136GB 文件，不整文件加载。
#[get("/pmtiles/basemap.pmtiles")]
async fn serve_pmtiles(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let file_path = state.data_dir.join("basemap.pmtiles");

    // 安全：必须在 data_dir 内
    let canonical_root = match state.data_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return HttpResponse::InternalServerError().body("500"),
    };
    let canonical_file = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            crate::logger::log(
                "proxy_server",
                "WARN",
                "basemap.pmtiles 缺失，2D 矢量地图不可用",
            );
            return HttpResponse::NotFound()
                .insert_header(("Cache-Control", "no-cache"))
                .body("pmtiles unavailable");
        }
    };
    if !is_within_root(&canonical_root, &canonical_file) || !canonical_file.is_file() {
        return HttpResponse::Forbidden().body("403 Forbidden");
    }

    let metadata = match std::fs::metadata(&canonical_file) {
        Ok(m) => m,
        Err(_) => return HttpResponse::InternalServerError().body("500"),
    };
    let total = metadata.len();
    // Empty file: `total - 1` would wrap to u64::MAX and cause a huge alloc.
    if total == 0 {
        return HttpResponse::Ok()
            .insert_header(("Content-Length", "0"))
            .body(Vec::new());
    }

    use std::io::{Read, Seek, SeekFrom};
    let mut file = match std::fs::File::open(&canonical_file) {
        Ok(f) => f,
        Err(_) => return HttpResponse::InternalServerError().body("500"),
    };

    // 解析 Range: bytes=start-end
    if let Some(range_header) = req.headers().get("range").and_then(|v| v.to_str().ok()) {
        if let Some(spec) = range_header.strip_prefix("bytes=") {
            let parts: Vec<&str> = spec.splitn(2, '-').collect();
            if parts.len() == 2 {
                let start: u64 = parts[0].parse().unwrap_or(0);
                let end: u64 = if parts[1].is_empty() {
                    total - 1
                } else {
                    parts[1].parse().unwrap_or(total - 1).min(total - 1)
                };
                if start > end || start >= total {
                    return HttpResponse::RangeNotSatisfiable()
                        .insert_header(("Content-Range", format!("bytes */{total}")))
                        .body("416 Range Not Satisfiable");
                }
                let length = end - start + 1;
                if file.seek(SeekFrom::Start(start)).is_err() {
                    return HttpResponse::InternalServerError().body("500");
                }
                let mut buf = vec![0u8; length as usize];
                if file.read_exact(&mut buf).is_err() {
                    return HttpResponse::InternalServerError().body("500");
                }
                return HttpResponse::PartialContent()
                    .insert_header(("Content-Type", "application/octet-stream"))
                    .insert_header(("Content-Range", format!("bytes {start}-{end}/{total}")))
                    .insert_header(("Content-Length", length.to_string()))
                    .insert_header(("Accept-Ranges", "bytes"))
                    .insert_header(("Cache-Control", "public, max-age=86400, immutable"))
                    .body(buf);
            }
        }
    }

    // 无 Range 头：返回整文件（PMTiles 首次读 header 会发小 Range，但兜底整文件）
    let mut buf = Vec::new();
    if std::io::Read::read_to_end(&mut file, &mut buf).is_err() {
        return HttpResponse::InternalServerError().body("500");
    }
    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/octet-stream"))
        .insert_header(("Content-Length", total.to_string()))
        .insert_header(("Accept-Ranges", "bytes"))
        .insert_header(("Cache-Control", "public, max-age=86400, immutable"))
        .body(buf)
}

/// 静态文件服务 — cesium, assets, terrain, tiles 等。
/// 路径格式: /{prefix}/{rest:.*}
///
/// 安全措施：
/// - canonicalize + starts_with 防止路径遍历（validate_within_root）
/// - URL 前缀白名单 (cesium|assets|terrain|tiles|fonts|sprites) 防止访问非授权目录
/// - 显式拒绝 `..`/`~`/`\\` —— actix `{rest:.*}` 不拒绝这些段，必须在此检查
#[get("/{prefix:(cesium|assets|terrain|tiles|fonts|sprites)}/{rest:.*}")]
async fn serve_static(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let path = req.path();
    let path = path.trim_start_matches('/');

    // URL percent-decode（如 %20 → 空格），字体目录名含空格需解码才能匹配文件
    let path = match percent_encoding::percent_decode_str(path).decode_utf8() {
        Ok(p) => p,
        Err(_) => return HttpResponse::BadRequest().body("400 Bad Request"),
    };
    let path = path.as_ref();

    // 拒绝包含路径遍历字符的请求
    if path.contains("..") || path.contains('~') || path.contains('\\') {
        return HttpResponse::Forbidden().body("403 Forbidden");
    }

    let file_path = state.data_dir.join(path);

    // 安全检查：canonicalize + starts_with 防路径遍历
    let canonical_file = match validate_within_root(&state.data_dir, &file_path) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    if !canonical_file.is_file() {
        return HttpResponse::NotFound().body("404 Not Found");
    }

    match std::fs::read(&canonical_file) {
        Ok(content) => {
            let mime = guess_mime_for_static(&canonical_file);
            HttpResponse::Ok()
                .insert_header(("Content-Type", mime))
                .insert_header(("Cache-Control", "public, max-age=86400, immutable"))
                .body(content)
        }
        Err(_) => HttpResponse::InternalServerError().body("500"),
    }
}

// ── 辅助函数 ──

fn tile_response(data: Vec<u8>, ext: &str) -> HttpResponse {
    let mime = match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", mime))
        .insert_header(("Cache-Control", "public, max-age=86400, immutable"))
        .body(data)
}

/// 按扩展名猜测 MIME 类型。
fn guess_mime_for_static(path: &std::path::Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("js") | Some("mjs") => "application/javascript".into(),
        Some("css") => "text/css".into(),
        Some("html") | Some("htm") => "text/html".into(),
        Some("json") => "application/json".into(),
        Some("xml") => "application/xml".into(),
        Some("jpg") | Some("jpeg") => "image/jpeg".into(),
        Some("png") => "image/png".into(),
        Some("svg") => "image/svg+xml".into(),
        Some("gif") => "image/gif".into(),
        Some("webp") => "image/webp".into(),
        Some("wasm") => "application/wasm".into(),
        Some("woff") => "font/woff".into(),
        Some("woff2") => "font/woff2".into(),
        Some("ttf") => "font/ttf".into(),
        _ => "application/octet-stream".into(),
    }
}
