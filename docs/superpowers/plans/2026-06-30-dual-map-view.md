# 双地图视图 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让程序启动后中国之外的 3D 地球不再空白（Esri 卫星预下载），并新增一个 2D MapLibre 矢量地图视图复用本地 136GB PMTiles 离线底图。

**Architecture:** 两条独立改动。A：Cesium 3D 底图从 OSM 在线换成走代理的 Esri 卫星，后端新增 `download_global_tiles` 预下载 z3-z6 全球瓦片（排除中国）写入现有 `china-tiles/` 目录复用端点。B：新增 MapLibre 2D 面板，actix 代理加 HTTP Range 端点透传 PMTiles 文件 + 扩展静态白名单服务 fonts/sprites，前端照搬 worldmonitor 的 `pmtiles` 协议 + `@protomaps/basemaps` 栈。

**Tech Stack:** Rust（actix-web, reqwest, tokio）/ TypeScript（React 18, Cesium 1.142, maplibre-gl, pmtiles, @protomaps/basemaps）/ PMTiles v3 / Tauri 2

## Global Constraints

- `#![forbid(unsafe_code)]` — 两个 crate 都强制，不得引入 unsafe。
- 数据目录由 `crate::data_dir::aurora_data_dir()` 解析（`AURORA_DATA_DIR` 环境变量 > 项目根 `aurora-data/` > exe 同目录）。所有文件路径基于此。
- `aurora-data/` 已在 `.gitignore`，136GB PMTiles 不会被提交。
- 瓦片端点和 L2 缓存 key 固定为 `china-tiles/{z}/{x}/{y}.jpg`，全球 Esri 瓦片复用此路径。
- `select_sources(z,x,y)` 对中国返回高德优先、对全球返回 Esri（`tile_sources.rs`）。
- 前端代理基址 `http://localhost:21337`（`RESOURCE_SERVER`）。
- 测试用 `assert_float_eq!` 宏做 f64 比较；`cargo test --workspace --all-features -- --test-threads=2`。
- `cargo fmt -- --check` 和 `cargo clippy --workspace --all-targets --all-features -- -D warnings` 必须通过。
- 文件 `basemap.pmtiles` 已在 `aurora-data/`；`fonts/` 和 `sprites/` 已复制到 `aurora-data/`。

---

## File Structure

**改动 A（3D 底图）：**
- Modify: `src-tauri/src/asset_fetcher.rs` — 新增 `GLOBAL_ZOOM_MIN/MAX` 常量、`global_tiles_for_zoom`、`global_total_tile_count`、`download_global_tiles_async`、`download_global_tiles`；在 `download_assets_async` 接入全球瓦片报告 + 后台下载。
- Modify: `ui/src/Earth.tsx` — OSM provider 换成走代理的 Esri `UrlTemplateImageryProvider`。
- Modify: `src-tauri/src/tile_sources.rs` — 测试全球瓦片排除中国区域。

**改动 B（2D 面板）：**
- Modify: `src-tauri/src/proxy_server.rs` — 新增 `serve_pmtiles` Range 端点；扩展 `serve_static` 白名单加 `fonts|sprites`。
- Create: `ui/src/MapPanel.tsx` — MapLibre 2D 面板组件。
- Modify: `ui/src/App.tsx` — 加 2D/3D 视图切换。
- Modify: `ui/package.json` — 加 `maplibre-gl`、`pmtiles`、`@protomaps/basemaps` 依赖。
- Modify: `ui/vite.config.ts` — manualChunks 把 maplibre/pmtiles/protomaps 拆独立 chunk。
- Create: `ui/src/test/MapPanel.test.tsx` — MapPanel 单测。

---

## Task 1: global_tiles_for_zoom — 全球瓦片坐标生成（排除中国）

**Files:**
- Modify: `src-tauri/src/asset_fetcher.rs`（在 `china_tiles_for_zoom` 之后，约 line 935）
- Test: `src-tauri/src/asset_fetcher.rs`（`#[cfg(test)] mod tests` 末尾）

**Interfaces:**
- Consumes: `crate::tile_sources::select_sources(z: u32, x: u32, y: u32) -> Vec<&'static TileSource>`（已存在）；TileSource.name 字段。
- Produces: `fn global_tiles_for_zoom(zoom: u32) -> Vec<(u32, u32, u32)>` — 全球 z 级瓦片坐标，排除中国 bbox（第一个源为"高德卫星"的瓦片）。

- [ ] **Step 1: 加常量**

在 `asset_fetcher.rs` line 116（`CHINA_ZOOM_MAX` 之后）加：

```rust
/// 全球底图瓦片缩放级别范围 (z3-z6)。
/// z6 约 2.5km/像素，看清大陆轮廓和国家。z7+ 走按需下载回退。
/// 排除中国区域（高德已覆盖），避免重复存储。
const GLOBAL_ZOOM_MIN: u32 = 3;
const GLOBAL_ZOOM_MAX: u32 = 6;
```

- [ ] **Step 2: 写失败测试**

在 `asset_fetcher.rs` 的 `#[cfg(test)] mod tests` 末尾加：

```rust
    #[test]
    fn test_global_tiles_excludes_china() {
        // z3 全球 8x8=64 张；中国 bbox (lat15-55,lng70-140) 约覆盖 2-3 张
        let tiles = super::global_tiles_for_zoom(3);
        assert!(tiles.len() < 64, "z3 全球应排除中国瓦片, got {}", tiles.len());
        // 排除的瓦片：任一被高德覆盖的坐标不应出现
        for (z, x, y) in &tiles {
            let sources = crate::tile_sources::select_sources(*z, *x, *y);
            let is_china = sources.first().map(|s| s.name == "高德卫星").unwrap_or(false);
            assert!(!is_china, "z{z} ({x},{y}) 是中国区域，不应出现在全球瓦片列表");
        }
    }

    #[test]
    fn test_global_tiles_z6_count_reasonable() {
        let tiles = super::global_tiles_for_zoom(6);
        // z6 全球 64x64=4096，减去中国约 100 张，应 < 4100
        assert!(tiles.len() > 3000 && tiles.len() < 4100, "z6 瓦片数异常: {}", tiles.len());
    }
```

- [ ] **Step 3: 运行测试确认失败**

Run: `cargo test --package src-tauri --lib global_tiles -- --test-threads=2`
Expected: 编译失败，`global_tiles_for_zoom` 未定义。

- [ ] **Step 4: 实现 global_tiles_for_zoom**

在 `china_tiles_for_zoom` 函数之后（line 935 之后）加：

```rust
/// 生成全球在指定缩放级别的所有瓦片坐标 (z, x, y)，排除中国区域。
/// 中国区域由高德卫星单独覆盖，全球列表跳过 select_sources 首选源为"高德卫星"的瓦片。
fn global_tiles_for_zoom(zoom: u32) -> Vec<(u32, u32, u32)> {
    let max = 1u32 << zoom;
    let mut tiles = Vec::new();
    for x in 0..max {
        for y in 0..max {
            // 排除中国区域（高德已覆盖）
            let is_china = crate::tile_sources::select_sources(zoom, x, y)
                .first()
                .map(|s| s.name == "高德卫星")
                .unwrap_or(false);
            if is_china {
                continue;
            }
            tiles.push((zoom, x, y));
        }
    }
    tiles
}

/// 计算全球 z3-z6 的总瓦片数（排除中国）。
fn global_total_tile_count() -> u64 {
    let mut total: u64 = 0;
    for z in GLOBAL_ZOOM_MIN..=GLOBAL_ZOOM_MAX {
        total += global_tiles_for_zoom(z).len() as u64;
    }
    total
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --package src-tauri --lib global_tiles -- --test-threads=2`
Expected: 两个测试 PASS。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/asset_fetcher.rs
git commit -m "feat(asset): global_tiles_for_zoom 全球瓦片坐标生成(排除中国)"
```

---

## Task 2: download_global_tiles_async — 全球瓦片批量下载

**Files:**
- Modify: `src-tauri/src/asset_fetcher.rs`（在 `download_china_tiles` 之后，约 line 1067）

**Interfaces:**
- Consumes: `TileDownloader::download_batch(&self, tiles: &[(u32,u32,u32)], output_dir: &Path, force: bool, concurrency: usize, progress_interval: u64) -> (u64, u64, u64)`（返回 `(downloaded, failed, skipped)`）；`crate::utils::{dir_size, dir_file_count, human_size}`；`ensure_dir`；`file_exists_and_nonempty`。
- Produces: `fn download_global_tiles(china_dir: &Path, force: bool) -> AssetInfo`（同步包装，内部建 tokio runtime）。

- [ ] **Step 1: 写 download_global_tiles_async**

在 `download_china_tiles` 函数之后（line 1067 之后）加。结构完全镜像 `download_china_tiles_async`，只换常量和坐标来源：

```rust
/// 下载全球卫星影像瓦片（Esri z3-z6，排除中国）。
/// 瓦片存入 china_dir（复用 china-tiles 端点 + L2 缓存）。
/// 调用方需在 tokio runtime 上下文中调用。
async fn download_global_tiles_async(
    china_dir: &Path,
    force: bool,
    downloader: &crate::tile_downloader::TileDownloader,
) -> AssetInfo {
    ensure_dir(china_dir);

    let existing_count = crate::utils::dir_file_count(china_dir);
    let expected_count = global_total_tile_count();

    if !force && existing_count >= expected_count {
        let size = crate::utils::dir_size(china_dir);
        crate::logger::log(
            "asset",
            "INFO",
            &format!(
                "✓ 全球瓦片已存在 ({} 个, {})",
                existing_count,
                crate::utils::human_size(size)
            ),
        );
        return AssetInfo {
            name: format!("全球卫星影像 z{GLOBAL_ZOOM_MIN}-z{GLOBAL_ZOOM_MAX}"),
            category: "global-tiles".into(),
            status: "cached".into(),
            size,
            size_human: crate::utils::human_size(size),
            source: String::new(),
            error: String::new(),
        };
    }

    crate::logger::log(
        "asset",
        "INFO",
        &format!(
            "↓ 下载全球卫星影像 (z{GLOBAL_ZOOM_MIN}-z{GLOBAL_ZOOM_MAX}, 约 {} 个瓦片, 16 并发)...",
            expected_count
        ),
    );

    let mut tiles_to_download: Vec<(u32, u32, u32)> = Vec::new();
    let mut skipped: u64 = 0;

    for z in GLOBAL_ZOOM_MIN..=GLOBAL_ZOOM_MAX {
        let tiles = global_tiles_for_zoom(z);
        crate::logger::log("asset", "INFO", &format!("  z{z}: {} 个瓦片", tiles.len()));

        for (zoom, x, y) in &tiles {
            let rel_path = format!("{zoom}/{x}/{y}.jpg");
            let target = china_dir.join(&rel_path);

            if !force && file_exists_and_nonempty(&target) {
                skipped += 1;
                continue;
            }

            tiles_to_download.push((*zoom, *x, *y));
        }
    }

    let (downloaded, failed, _) = downloader
        .download_batch(&tiles_to_download, china_dir, force, 16, 100)
        .await;

    crate::logger::log(
        "asset",
        "INFO",
        &format!("全球瓦片下载完成: 下载 {downloaded}, 跳过 {skipped}, 失败 {failed}"),
    );

    let size = crate::utils::dir_size(china_dir);
    let current_count = crate::utils::dir_file_count(china_dir);

    if current_count > 0 {
        AssetInfo {
            name: format!("全球卫星影像 z{GLOBAL_ZOOM_MIN}-z{GLOBAL_ZOOM_MAX}"),
            category: "global-tiles".into(),
            status: "ok".into(),
            size,
            size_human: crate::utils::human_size(size),
            source: "arcgisonline.com".into(),
            error: if failed > 0 {
                format!("{failed} 个瓦片下载失败")
            } else {
                String::new()
            },
        }
    } else {
        crate::logger::log("asset", "ERROR", "✗ 全球瓦片下载全部失败");
        AssetInfo {
            name: format!("全球卫星影像 z{GLOBAL_ZOOM_MIN}-z{GLOBAL_ZOOM_MAX}"),
            category: "global-tiles".into(),
            status: "failed".into(),
            size: 0,
            size_human: String::new(),
            source: String::new(),
            error: "所有瓦片下载失败".into(),
        }
    }
}

/// 同步版本 — 内部创建 tokio runtime。
fn download_global_tiles(china_dir: &Path, force: bool) -> AssetInfo {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime for global tiles download");

    let downloader = crate::tile_downloader::TileDownloader::new();
    rt.block_on(download_global_tiles_async(china_dir, force, &downloader))
}
```

- [ ] **Step 2: 编译确认**

Run: `cargo build --package src-tauri`
Expected: 编译成功（函数已定义但未调用，会有 `dead_code` 警告——Task 3 会接入，可忽略；若 clippy `-D warnings` 在 CI 报 dead_code，Task 3 接入后消失）。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/asset_fetcher.rs
git commit -m "feat(asset): download_global_tiles 全球 Esri 卫星瓦片预下载"
```

---

## Task 3: 接入启动流 — download_assets_async 报告 + 后台下载

**Files:**
- Modify: `src-tauri/src/asset_fetcher.rs`（`download_assets_async`，line 514-560 中国瓦片块之后）

**Interfaces:**
- Consumes: Task 2 的 `download_global_tiles`；`global_total_tile_count`。
- Produces: `AssetReport.assets` 含 `category: "global-tiles"` 条目，前端 `get_asset_status` 可见。

- [ ] **Step 1: 在 download_assets_async 加全球瓦片报告 + 后台下载**

在 `download_assets_async` 中，中国瓦片块（line 560 `}` 结束）之后、`AssetReport { ... }` 返回之前（line 562 之前）插入：

```rust
    // ── 全球瓦片：后台线程，不阻塞 ──
    // 复用 china_dir（同一端点 + L2 缓存 key china-tiles/{z}/{x}/{y}.jpg）
    let global_expected = global_total_tile_count();
    // 全球瓦片混在 china_dir 里，无法单独计数，用 expected 做就绪判断的粗略近似：
    // 若 china_dir 文件数 >= 中国+全球 expected 之和，视为就绪。
    let global_done = !force
        && china_existing >= china_expected + global_expected;

    if global_done {
        assets.push(AssetInfo {
            name: format!("全球卫星影像 z{GLOBAL_ZOOM_MIN}-z{GLOBAL_ZOOM_MAX}"),
            category: "global-tiles".into(),
            status: "cached".into(),
            size: 0,
            size_human: String::new(),
            source: String::new(),
            error: String::new(),
        });
    } else {
        all_ready = false;
        assets.push(AssetInfo {
            name: format!("全球卫星影像 z{GLOBAL_ZOOM_MIN}-z{GLOBAL_ZOOM_MAX}"),
            category: "global-tiles".into(),
            status: "ok".into(), // "下载中"
            size: 0,
            size_human: format!("{}/{}", china_existing, china_expected + global_expected),
            source: "后台下载中".into(),
            error: String::new(),
        });

        let global_dir_bg = china_dir.clone();
        std::thread::Builder::new()
            .name("global-tiles-downloader".into())
            .spawn(move || {
                let info = download_global_tiles(&global_dir_bg, force);
                crate::logger::log(
                    "asset",
                    "INFO",
                    &format!("全球瓦片后台下载完成: status={}", info.status),
                );
            })
            .expect("failed to spawn global-tiles-downloader thread");
    }
```

- [ ] **Step 2: 编译确认**

Run: `cargo build --package src-tauri`
Expected: 编译成功，无 dead_code 警告（`download_global_tiles` 现已被调用）。

- [ ] **Step 3: 运行现有测试不破**

Run: `cargo test --workspace --all-features -- --test-threads=2`
Expected: 全部 PASS（含 Task 1 的 global_tiles 测试）。

- [ ] **Step 4: fmt + clippy**

Run: `cargo fmt -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: 通过。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/asset_fetcher.rs
git commit -m "feat(asset): 启动流接入全球瓦片后台下载 + 状态报告"
```

---

## Task 4: 前端 Cesium 底图换 Esri

**Files:**
- Modify: `ui/src/Earth.tsx`（line 251-254，`initCesiumJS` 内）

**Interfaces:**
- Consumes: `RESOURCE_SERVER`（`http://localhost:21337`，line 15）。
- Produces: 3D 地球底图走代理 china-tiles 端点（全球 Esri + 中国高德自动选源）。

- [ ] **Step 1: 替换 OSM provider 为 Esri 代理 provider**

在 `Earth.tsx` 找到（line 251-254）：

```typescript
    const osmProvider = new Cesium.OpenStreetMapImageryProvider({
      url: 'https://tile.openstreetmap.org/',
      maximumLevel: 18,
    });
```

替换为：

```typescript
    // 全球底图走代理 china-tiles 端点：select_sources 自动按区域选源
    // （中国→高德卫星，全球→Esri World Imagery）。z3-z6 后端预下载，z7+ 按需下载回退。
    const baseProvider = new Cesium.UrlTemplateImageryProvider({
      url: `${RESOURCE_SERVER}/china-tiles/{z}/{x}/{y}.jpg`,
      tilingScheme: new Cesium.WebMercatorTilingScheme(),
      minimumLevel: 3,
      maximumLevel: 18,
    });
```

- [ ] **Step 2: 更新 Viewer 构造里的引用**

在 `Earth.tsx` 找到（line 257）：

```typescript
      baseLayer: new Cesium.ImageryLayer(osmProvider),
```

替换为：

```typescript
      baseLayer: new Cesium.ImageryLayer(baseProvider),
```

- [ ] **Step 3: 运行前端测试**

Run: `cd ui && npm test`
Expected: 现有 Earth 测试通过（`buildTextureUrls` 等不涉及 base provider，应不受影响）。

- [ ] **Step 4: 类型检查 + 构建**

Run: `cd ui && npm run build`
Expected: `tsc && vite build` 成功。

- [ ] **Step 5: Commit**

```bash
git add ui/src/Earth.tsx
git commit -m "feat(earth): 3D 底图换 Esri 卫星走代理,解决中国之外空白"
```

---

## Task 5: 后端 PMTiles Range 端点

**Files:**
- Modify: `src-tauri/src/proxy_server.rs`（新增 `serve_pmtiles` handler + 注册到 App）

**Interfaces:**
- Consumes: `AppState.data_dir`（`PathBuf`）；`std::fs::File` seek/read。
- Produces: `GET /pmtiles/basemap.pmtiles` 支持 Range，返回 206/200/404。

- [ ] **Step 1: 写 serve_pmtiles handler**

在 `proxy_server.rs` 的 `serve_tile` 之后（line 172 之后）加：

```rust
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
    if !canonical_file.starts_with(&canonical_root) || !canonical_file.is_file() {
        return HttpResponse::Forbidden().body("403 Forbidden");
    }

    let metadata = match std::fs::metadata(&canonical_file) {
        Ok(m) => m,
        Err(_) => return HttpResponse::InternalServerError().body("500"),
    };
    let total = metadata.len();

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
```

- [ ] **Step 2: 注册到 App**

在 `proxy_server.rs` 找到（line 67-69）：

```rust
                        .service(health)
                        .service(serve_tile)
                        .service(serve_static)
```

改为：

```rust
                        .service(health)
                        .service(serve_tile)
                        .service(serve_pmtiles)
                        .service(serve_static)
```

- [ ] **Step 3: 编译确认**

Run: `cargo build --package src-tauri`
Expected: 编译成功。

- [ ] **Step 4: fmt + clippy**

Run: `cargo fmt -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: 通过。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/proxy_server.rs
git commit -m "feat(proxy): PMTiles Range 端点 /pmtiles/basemap.pmtiles"
```

---

## Task 6: 扩展静态白名单 — fonts/sprites

**Files:**
- Modify: `src-tauri/src/proxy_server.rs`（`serve_static` 路由，line 181）

**Interfaces:**
- Consumes: `AppState.data_dir`；`aurora-data/fonts/`、`aurora-data/sprites/` 已存在。
- Produces: `GET /fonts/...`、`GET /sprites/...` 静态服务。

- [ ] **Step 1: 扩展白名单正则**

在 `proxy_server.rs` 找到（line 181）：

```rust
#[get("/{prefix:(cesium|assets|terrain|tiles)}/{rest:.*}")]
```

改为：

```rust
#[get("/{prefix:(cesium|assets|terrain|tiles|fonts|sprites)}/{rest:.*}")]
```

- [ ] **Step 2: 更新 doc comment**

找到 `serve_static` 上方注释（line 179）：

```rust
/// - URL 前缀白名单 (cesium|assets|terrain|tiles) 防止访问非授权目录
```

改为：

```rust
/// - URL 前缀白名单 (cesium|assets|terrain|tiles|fonts|sprites) 防止访问非授权目录
```

- [ ] **Step 3: 编译 + clippy**

Run: `cargo build --package src-tauri && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: 通过。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/proxy_server.rs
git commit -m "feat(proxy): 静态白名单加 fonts/sprites"
```

---

## Task 7: 前端依赖 + MapPanel 组件

**Files:**
- Modify: `ui/package.json`
- Create: `ui/src/MapPanel.tsx`
- Create: `ui/src/test/MapPanel.test.tsx`

**Interfaces:**
- Consumes: `RESOURCE_SERVER = 'http://localhost:21337'`；`aurora-data/basemap.pmtiles`、`fonts/`、`sprites/v4/{light,dark}`。
- Produces: `default export MapPanel({ flavor?: 'light' | 'dark' })` — 挂载 MapLibre 2D 地图。

- [ ] **Step 1: 安装依赖**

Run: `cd ui && npm install maplibre-gl@^4.7.1 pmtiles@^4.4.0 @protomaps/basemaps@^5.7.2`
Expected: 三个包写入 `package.json` dependencies。

- [ ] **Step 2: 写 MapPanel 组件**

创建 `ui/src/MapPanel.tsx`：

```typescript
// ui/src/MapPanel.tsx — 2D 矢量地图面板 (MapLibre + PMTiles)
// 复用 worldmonitor 的 pmtiles 协议 + @protomaps/basemaps 栈。
// maplibre/pmtiles/protomaps 动态 import，不进主 bundle。
import { useEffect, useRef } from 'react';
import diag from './utils/diag';

const RESOURCE_SERVER = 'http://localhost:21337';

let registered = false;
let registerPromise: Promise<void> | null = null;

async function registerPMTilesProtocol(): Promise<void> {
  if (registered) return;
  registerPromise ??= (async () => {
    const { Protocol } = await import('pmtiles');
    const maplibregl = (await import('maplibre-gl')).default;
    const protocol = new Protocol();
    maplibregl.addProtocol('pmtiles', protocol.tile);
    registered = true;
  })();
  await registerPromise;
}

async function buildPMTilesStyle(flavor: 'light' | 'dark') {
  const { layers, namedFlavor } = await import('@protomaps/basemaps');
  const spriteName = flavor === 'light' ? 'light' : 'dark';
  return {
    version: 8 as const,
    glyphs: `${RESOURCE_SERVER}/fonts/{fontstack}/{range}.pbf`,
    sprite: `${RESOURCE_SERVER}/sprites/v4/${spriteName}`,
    sources: {
      basemap: {
        type: 'vector' as const,
        url: `pmtiles://${RESOURCE_SERVER}/pmtiles/basemap.pmtiles`,
        maxzoom: 15,
      },
    },
    layers: layers('basemap', namedFlavor(flavor), { lang: 'en' }),
  };
}

interface Props {
  flavor?: 'light' | 'dark';
}

export default function MapPanel({ flavor = 'light' }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let map: any = null;
    let cancelled = false;

    (async () => {
      try {
        await registerPMTilesProtocol();
        if (cancelled || !containerRef.current) return;
        const maplibregl = (await import('maplibre-gl')).default;
        const style = await buildPMTilesStyle(flavor);
        if (cancelled || !containerRef.current) return;
        map = new maplibregl.Map({
          container: containerRef.current,
          style,
          center: [116, 36],
          zoom: 3,
        });
        diag('MapPanel', 'INFO', '2D 矢量地图已加载');
      } catch (e) {
        diag('MapPanel', 'WARN', `2D 地图加载失败: ${e}`);
      }
    })();

    return () => {
      cancelled = true;
      map?.remove();
    };
  }, [flavor]);

  return <div ref={containerRef} style={{ width: '100%', height: '100%' }} />;
}
```

- [ ] **Step 3: 写单测**

创建 `ui/src/test/MapPanel.test.tsx`：

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/react';

// mock 动态 import 的 maplibre/pmtiles/protomaps，避免 jsdom 跑 WebGL
vi.mock('maplibre-gl', () => ({
  default: {
    addProtocol: vi.fn(),
    Map: vi.fn().mockImplementation(() => ({ remove: vi.fn() })),
  },
}));
vi.mock('pmtiles', () => ({ Protocol: vi.fn().mockImplementation(() => ({ tile: vi.fn() })) }));
vi.mock('@protomaps/basemaps', () => ({
  layers: vi.fn().mockReturnValue([]),
  namedFlavor: vi.fn().mockReturnValue('light'),
}));

import MapPanel from '../MapPanel';

describe('MapPanel', () => {
  it('渲染容器 div', async () => {
    const { container } = render(<MapPanel flavor="light" />);
    const div = container.querySelector('div');
    expect(div).not.toBeNull();
    expect(div!.style.width).toBe('100%');
  });
});
```

- [ ] **Step 4: 运行测试**

Run: `cd ui && npm test`
Expected: MapPanel 测试 PASS，现有测试不破。

- [ ] **Step 5: 类型检查 + 构建**

Run: `cd ui && npm run build`
Expected: 成功（maplibre/pmtiles/protomaps 被动态 import，打进独立 chunk）。

- [ ] **Step 6: Commit**

```bash
git add ui/package.json ui/package-lock.json ui/src/MapPanel.tsx ui/src/test/MapPanel.test.tsx
git commit -m "feat(ui): MapPanel 2D 矢量地图组件 (MapLibre + PMTiles)"
```

---

## Task 8: vite manualChunks 拆分重依赖

**Files:**
- Modify: `ui/vite.config.ts`

**Interfaces:**
- Consumes: maplibre-gl / pmtiles / @protomaps/basemaps（Task 7 装的）。
- Produces: 这三个包进独立 chunk，不污染主 bundle。

- [ ] **Step 1: 加 manualChunks**

在 `ui/vite.config.ts` 的 `build` 对象里加 `rollupOptions`。找到：

```typescript
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
```

改为：

```typescript
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        manualChunks: {
          // ponytail: maplibre/pmtiles/protomaps 是 2D 面板重依赖，动态 import，
          // 单独拆 chunk 避免主 bundle 膨胀，且只在打开 2D 面板时加载。
          maplibre: ['maplibre-gl'],
          pmtiles: ['pmtiles', '@protomaps/basemaps'],
        },
      },
    },
  },
```

- [ ] **Step 2: 构建确认**

Run: `cd ui && npm run build`
Expected: 构建成功，dist/assets 下有 `maplibre-*.js` 和 `pmtiles-*.js` chunk。

- [ ] **Step 3: Commit**

```bash
git add ui/vite.config.ts
git commit -m "build(ui): manualChunks 拆分 maplibre/pmtiles"
```

---

## Task 9: App.tsx 2D/3D 视图切换

**Files:**
- Modify: `ui/src/App.tsx`

**Interfaces:**
- Consumes: `MapPanel`（Task 7）；`Earth`（现有）；`isTauriEnvironment`。
- Produces: 顶栏一个 2D/3D 切换，默认 3D。

- [ ] **Step 1: 加视图状态 + 懒加载 MapPanel**

在 `ui/src/App.tsx` 顶部 import 区（line 6 `import Earth from './Earth';` 之后）加：

```typescript
import { lazy, Suspense } from 'react';
const MapPanel = lazy(() => import('./MapPanel'));
```

（若 `lazy`/`Suspense` 已从 react 导入则合并，否则加入现有 `import { useState, ... } from 'react';`。）

- [ ] **Step 2: 加视图状态**

在组件内（`DEFAULT_PIPELINE_REQUEST` 等状态声明附近，约 line 50-90 区间）加：

```typescript
  const [view2D, setView2D] = useState(false);
```

- [ ] **Step 3: 加切换按钮 + 渲染 MapPanel**

找到（line 162-165）：

```typescript
      {/* Globe backdrop fills the content area */}
      <div className="aur-globe-area aur-globe-area--full">
        <Earth resumeDelayMs={resumeDelayMs} rotationSpeed={rotationSpeed} />
      </div>
```

替换为：

```typescript
      {/* 2D/3D 视图切换按钮 */}
      <button
        className="aur-view-toggle"
        onClick={() => setView2D(v => !v)}
        style={{ position: 'fixed', top: 60, right: 16, zIndex: 50 }}
      >
        {view2D ? '3D 地球' : '2D 地图'}
      </button>

      {/* Globe / Map backdrop fills the content area */}
      <div className="aur-globe-area aur-globe-area--full">
        {view2D ? (
          <Suspense fallback={<div style={{ color: '#fff' }}>加载 2D 地图…</div>}>
            <MapPanel flavor="light" />
          </Suspense>
        ) : (
          <Earth resumeDelayMs={resumeDelayMs} rotationSpeed={rotationSpeed} />
        )}
      </div>
```

- [ ] **Step 4: 运行测试**

Run: `cd ui && npm test`
Expected: 现有测试不破（App 测试若有快照需更新）。

- [ ] **Step 5: 类型检查 + 构建**

Run: `cd ui && npm run build`
Expected: 成功。

- [ ] **Step 6: Commit**

```bash
git add ui/src/App.tsx
git commit -m "feat(ui): 2D/3D 视图切换"
```

---

## Task 10: 端到端验证

**Files:** 无（验证任务）

- [ ] **Step 1: 全量测试**

Run: `cargo test --workspace --all-features -- --test-threads=2 && cd ui && npm test`
Expected: 全绿。

- [ ] **Step 2: fmt + clippy**

Run: `cargo fmt -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: 通过。

- [ ] **Step 3: 手动启动验证**

Run: `cargo tauri dev`（或项目既有的启动方式）

验证清单：
- 3D 地球启动后中国之外有 Esri 卫星底图（z3-z6 预下载完成后秒开；下载中靠按需回退，可能短暂空白但会填充）。
- 中国区域高德卫星叠加正常。
- 后台日志有 "全球瓦片后台下载完成: status=ok"。
- 点 "2D 地图" 切换，MapPanel 加载矢量地图，有地名、可缩放。
- 切回 "3D 地球" 正常。
- `curl -r 0-127 http://localhost:21337/pmtiles/basemap.pmtiles -o /tmp/hdr.bin` 返回 128 字节、HTTP 206。

- [ ] **Step 4: 缺失场景验证**

临时改名 `aurora-data/basemap.pmtiles` → `.bak`，重启，点 2D 地图：
Expected: 2D 面板加载失败日志，不崩溃；恢复文件名后正常。验证完恢复。

- [ ] **Step 5: Commit（若有验证中修的小问题）**

```bash
git add -A
git commit -m "fix: 端到端验证修复" || echo "无修复，跳过"
```

---

## Self-Review

**1. Spec coverage:**
- 改动 A（3D 底图 Esri 预下载）：Task 1（坐标生成）→ 2（下载函数）→ 3（启动流接入）→ 4（前端换 provider）。✓
- 改动 B 后端（Range 端点 + 静态白名单）：Task 5 → 6。✓
- 改动 B 前端（MapPanel + 依赖 + UI 切换）：Task 7 → 8 → 9。✓
- 文件搬运：basemap.pmtiles 已搬；fonts/sprites 已复制（Global Constraints 注明）。✓
- 端到端验证：Task 10。✓

**2. Placeholder scan:** 无 TBD/TODO。"appropriate error handling" 未出现——错误处理都在代码里具体写了。✓

**3. Type consistency:**
- `global_tiles_for_zoom(zoom: u32) -> Vec<(u32,u32,u32)>` — Task 1 定义，Task 2 调用，一致。✓
- `download_global_tiles(china_dir: &Path, force: bool) -> AssetInfo` — Task 2 定义，Task 3 调用，一致。✓
- `download_batch` 返回 `(downloaded, failed, skipped)` — Task 2 解构 `(downloaded, failed, _)`，与 tile_downloader.rs:120 一致。✓
- `MapPanel({ flavor })` — Task 7 定义，Task 9 用 `<MapPanel flavor="light" />`，一致。✓
- `serve_pmtiles` — Task 5 定义并注册，路由 `/pmtiles/basemap.pmtiles` 与 Task 7 的 `pmtiles://${RESOURCE_SERVER}/pmtiles/basemap.pmtiles` 一致。✓
- 静态白名单 `fonts|sprites` — Task 6 加，Task 7 用 `/fonts/` `/sprites/v4/`，一致。✓

无问题。
