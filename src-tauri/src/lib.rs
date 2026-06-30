mod asset_fetcher;
mod commands;
mod data_dir;
mod data_sources;
mod l1_cache;
mod l2_cache;
mod logger;
mod proxy_server;
mod tile_downloader;
mod tile_sources;
mod utils;

use aurora::app::AuroraApp;
use commands::AppState;
use l1_cache::L1Cache;
use l2_cache::L2Cache;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tile_downloader::TileDownloader;

#[tauri::command]
fn show_window(window: tauri::Window) {
    logger::log("window", "INFO", "show_window 被调用");
    let _ = window.show();
}

#[tauri::command]
fn exit_app(window: tauri::Window) {
    logger::log("window", "INFO", "exit_app 被调用 — 关闭窗口");
    let _ = window.close();
}

/// 诊断命令：返回 WebView 的当前 URL
#[tauri::command]
fn diag_url(window: tauri::WebviewWindow) -> String {
    let url = window.url();
    logger::log(
        "diag",
        "INFO",
        &format!("diag_url 请求 — 当前 URL: {:?}", url),
    );
    format!("{:?}", url)
}

/// 诊断命令：接收前端日志写入 Rust 日志文件。
/// level 仅接受 INFO/WARN/ERROR，防止注入非法日志级别。
/// module 和 message 中的换行符被替换为空格，防止日志注入。
#[tauri::command]
fn frontend_log(level: String, module: String, message: String) {
    let validated_level = match level.as_str() {
        "INFO" | "WARN" | "ERROR" => level,
        _ => {
            logger::log(
                "frontend_log",
                "WARN",
                &format!("非法日志级别: {} — 降级为 WARN", sanitize_log_str(&level)),
            );
            "WARN".into()
        }
    };
    let safe_module = sanitize_log_str(&module);
    let safe_message = sanitize_log_str(&message);
    logger::log(
        &format!("js:{}", safe_module),
        &validated_level,
        &safe_message,
    );
}

/// 防止日志注入：将换行符和控制字符替换为可见占位符。
fn sanitize_log_str(s: &str) -> String {
    s.replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace(
            |c: char| c.is_control() && c != '\n' && c != '\r' && c != '\t',
            "?",
        )
}

/// 检查本地缓存纹理是否全部就绪。
/// 返回 assets 目录路径（若就绪）或空字符串（若缺失）。
#[tauri::command]
fn check_cached_assets() -> String {
    let report = asset_fetcher::asset_status();
    if report.all_ready {
        report.assets_dir
    } else {
        String::new()
    }
}

/// 查询所有资源文件的详细状态。
#[tauri::command]
fn get_asset_status() -> asset_fetcher::AssetReport {
    asset_fetcher::asset_status()
}

/// 手动触发资源下载。force=true 时强制重新下载已有文件。
/// 快速资源（纹理+CesiumJS+z0-z2瓦片）同步下载后立即返回，
/// 中国瓦片在后台线程下载，前端通过 get_asset_status 轮询进度。
#[tauri::command]
fn download_assets(force: bool) -> asset_fetcher::AssetReport {
    asset_fetcher::download_assets_async(force)
}

/// 返回本地资源服务器地址。
#[tauri::command]
fn get_resource_server_url() -> String {
    "http://localhost:21337".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── 1. 初始化日志 ──────────────────────────────────────────────
    let _log_path = logger::init().unwrap_or_else(|e| {
        eprintln!("[FATAL] 日志初始化失败: {e}");
        panic!("日志初始化失败: {e}");
    });

    // ── 2. 确定数据根目录 ──────────────────────────────────
    let aurora_dir = data_dir::aurora_data_dir();

    // ── 3. 创建共享缓存实例 ──────────────────────────────────
    let l1 = Arc::new(L1Cache::new(256 * 1024 * 1024)); // 256MB
    let l2 = Arc::new(L2Cache::new(
        aurora_dir.join("china-tiles"),
        50u64 * 1024 * 1024 * 1024, // 50GB
    ));
    let downloader = Arc::new(TileDownloader::new());

    // ── 4. 启动 actix-web 代理服务器 ──────────────────────────────
    logger::log(
        "init",
        "INFO",
        "启动 actix-web 代理服务器 (localhost:21337)...",
    );
    let server_shutdown = Arc::new(AtomicBool::new(false));
    let server_handle = proxy_server::start_proxy_server(
        aurora_dir.clone(),
        Arc::clone(&l1),
        Arc::clone(&l2),
        Arc::clone(&downloader),
        Arc::clone(&server_shutdown),
    );
    logger::log("init", "INFO", "actix-web 代理服务器已启动");

    // ── 4. 后台下载核心资源（纹理 + CesiumJS + z0-z2 瓦片，约 25MB）──────
    // 不阻塞启动——以存在文件可直接服务于 proxy_server。
    // 中国瓦片 z3-z10 (~575MB) 也在后台异步下载。
    logger::log("init", "INFO", "后台检查并下载核心地球资源...");
    std::thread::Builder::new()
        .name("asset-downloader".into())
        .spawn(|| {
            asset_fetcher::ensure_all_resources();
            crate::logger::log("init", "INFO", "核心资源检查完成，中国瓦片后台下载中");
        })
        .expect("failed to spawn asset-downloader thread");

    // ── 4b. 后台定时刷新公开数据源缓存（climate + ucdp）──────
    // 借鉴 worldmonitor seed 循环：定时预热缓存，首屏命令读缓存零等待。
    // 复用 server_shutdown 信号优雅退出。
    logger::log("init", "INFO", "启动公开数据源后台刷新线程...");
    data_sources::spawn_refresh_loop(Arc::clone(&l2), Arc::clone(&server_shutdown));

    // ── 5. 初始化 AuroraApp ────────────────────────────────────────
    // M1 数据持久化：DB 落盘到 aurora_data_dir/aurora.db，数据跨重启保留
    // （M1 Exit Criteria "数据导出"的前提 + CHARTER "不剥夺"底线）。
    // 持久化失败回落 in-memory，不阻塞启动。
    let db_path = aurora_dir.join("aurora.db");
    logger::log("init", "INFO", &format!("初始化 AuroraApp (DB: {})...", db_path.display()));
    let aurora_app = match AuroraApp::new(Some(&db_path)) {
        Ok(app) => {
            logger::log("init", "INFO", "AuroraApp 初始化完成 (持久化 DB)");
            app
        }
        Err(e) => {
            logger::log(
                "init",
                "WARN",
                &format!("持久化 DB 失败 {e}，回落 in-memory"),
            );
            match AuroraApp::new(None) {
                Ok(app) => {
                    logger::log("init", "WARN", "AuroraApp 初始化完成 (in-memory 回落)");
                    app
                }
                Err(e2) => {
                    logger::log("init", "ERROR", &format!("AuroraApp in-memory 也失败: {e2}"));
                    server_shutdown.store(true, Ordering::SeqCst);
                    let _ = server_handle.join();
                    eprintln!("Fatal: AuroraApp init failed: {e2}");
                    std::process::exit(1);
                }
            }
        }
    };

    // ── 6. 构建 Tauri 应用 ─────────────────────────────────────────
    logger::log("init", "INFO", "构建 Tauri Builder...");
    let builder = tauri::Builder::default()
        .manage(AppState {
            app: Mutex::new(aurora_app),
            l1: Arc::clone(&l1),
            l2: Arc::clone(&l2),
            downloader: Arc::clone(&downloader),
        })
        .invoke_handler(tauri::generate_handler![
            show_window,
            exit_app,
            diag_url,
            frontend_log,
            check_cached_assets,
            get_asset_status,
            download_assets,
            get_resource_server_url,
            commands::run_analysis_pipeline,
            commands::cache_stats,
            commands::set_cache_limit,
            commands::clear_cache,
            commands::refresh_tiles,
            commands::server_health,
            commands::prefetch_tiles,
            commands::get_monitoring_stations,
            commands::get_anchor_status,
            commands::get_geo_events,
            commands::export_user_data,
        ])
        .setup(|app| {
            logger::log("setup", "INFO", "Tauri setup 回调开始");

            // ponytail: Tauri config app.windows 已自动创建 main 窗口，获取而不是重建
            let _window = app
                .get_webview_window("main")
                .expect("main 窗口应由 Tauri 自动创建（见 tauri.conf.json app.windows）");
            logger::log("setup", "INFO", "获取到 main 窗口");

            #[cfg(debug_assertions)]
            {
                logger::log("setup", "INFO", "打开 DevTools (debug 模式)");
                _window.open_devtools();
            }

            logger::log("setup", "INFO", "Tauri setup 回调完成");
            Ok(())
        })
        .on_window_event(|window, event| match &event {
            tauri::WindowEvent::CloseRequested { .. } => {
                logger::log("window-event", "INFO", "CloseRequested — 销毁窗口");
                let _ = window.destroy();
            }
            tauri::WindowEvent::Destroyed => {
                logger::log(
                    "window-event",
                    "INFO",
                    &format!("Destroyed (label={})", window.label()),
                );
            }
            _ => {}
        });

    logger::log("init", "INFO", "Builder 构建完成，准备运行...");

    // ── 7. 运行 ────────────────────────────────────────────────────
    let run_result = builder.run(tauri::generate_context!());

    // Signal proxy server to shut down gracefully
    server_shutdown.store(true, Ordering::SeqCst);
    let _ = server_handle.join();

    match run_result {
        Ok(()) => {
            logger::log("run", "INFO", "Tauri 正常退出");
        }
        Err(ref e) => {
            logger::log("run", "ERROR", &format!("Tauri 运行错误: {e}"));
        }
    }

    logger::log("run", "INFO", "══════════════════════════════════════════");
    logger::log("run", "INFO", "Aurora Desktop 已退出");
}
