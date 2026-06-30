//! 多渠道资源预下载器。
//!
//! 启动时检查 ~/.aurora/ 下是否有 CesiumJS 库、影像瓦片、地形数据、纹理，
//! 若缺失则从多个 CDN 源依次尝试下载。
//!
//! 四类资源：
//!   1. CesiumJS 库文件（~30MB 核心）→ ~/.aurora/cesium/
//!   2. 影像瓦片（Natural Earth II z0-z2）→ ~/.aurora/tiles/
//!   3. 中国区域瓦片（高德卫星 z3-z10）→ ~/.aurora/china-tiles/
//!   4. 纹理资源（蓝色大理石等）     → ~/.aurora/assets/
//!
//! 地形数据暂不自动下载（quantized-mesh 生成复杂，后续支持）。
//! 下载到 .tmp 文件后 rename，保证原子性。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

// ══════════════════════════════════════════════════════════════════
// 资源清单
// ══════════════════════════════════════════════════════════════════

/// 纹理资源（地球贴图）。
const TEXTURE_ASSETS: &[&str] = &[
    "earth-blue-marble.jpg",
    "earth-topo-bathy.jpg",
    "earth-topology.png",
    "night-sky.png",
];

/// CesiumJS 下载策略：下载 npm tarball 并解压到 ~/.aurora/cesium/。
/// 不逐个文件下载——110+ Workers 文件名随版本变化，不可硬编码。
/// tarball 约 40MB，一次下载后解压即完成。
///
/// CesiumJS npm tarball CDN 源。
/// 国内镜像前置（npmmirror 阿里源，国内最快最稳），国外源作回退。
const CESIUM_TARBALL_MIRRORS: &[&str] = &[
    "https://registry.npmmirror.com/cesium/-/cesium-1.125.0.tgz",
    "https://cdn.jsdelivr.net/npm/cesium@1.125.0/cesium-1.125.0.tgz",
    "https://registry.npmjs.org/cesium/-/cesium-1.125.0.tgz",
];

/// Natural Earth II 影像瓦片（z0-z2 全球覆盖，约 21 个瓦片）。
/// 瓦片路径格式：z/x/y.jpg
const TILE_FILES: &[&str] = &[
    "tilemapresource.xml",
    "0/0/0.jpg",
    "1/0/0.jpg",
    "1/0/1.jpg",
    "1/1/0.jpg",
    "1/1/1.jpg",
    "2/0/0.jpg",
    "2/0/1.jpg",
    "2/0/2.jpg",
    "2/0/3.jpg",
    "2/1/0.jpg",
    "2/1/1.jpg",
    "2/1/2.jpg",
    "2/1/3.jpg",
    "2/2/0.jpg",
    "2/2/1.jpg",
    "2/2/2.jpg",
    "2/2/3.jpg",
    "2/3/0.jpg",
    "2/3/1.jpg",
    "2/3/2.jpg",
    "2/3/3.jpg",
];

// ══════════════════════════════════════════════════════════════════
// CDN 源
// ══════════════════════════════════════════════════════════════════

/// 纹理资源 CDN 源。
/// 国内可达源前置（jsdelivr 国内通常可达），国外源作回退。
const TEXTURE_MIRRORS: &[&str] = &[
    "https://cdn.jsdelivr.net/npm/three-globe/example/img/",
    "https://fastly.jsdelivr.net/npm/three-globe/example/img/",
    "https://unpkg.com/three-globe/example/img/",
    "https://raw.githubusercontent.com/vasturiano/three-globe/master/example/img/",
];

/// earth-topo-bathy.jpg 专用源（不在 three-globe 仓库中）。
/// 该图源自 Wikimedia Commons，国内无可靠镜像；保留为唯一源，
/// 失败时由纹理回退逻辑处理（不影响主地球渲染）。
const TOPO_BATHY_MIRRORS: &[&str] = &[
    "https://upload.wikimedia.org/wikipedia/commons/thumb/4/4d/Whole_world_-_land_and_oceans_12000.jpg/2048px-Whole_world_-_land_and_oceans_12000.jpg",
];

/// Natural Earth II 瓦片 CDN 源。
/// Cesium 官方提供的离线瓦片包。国内可达源（jsdelivr）前置。
const TILE_MIRRORS: &[&str] = &[
    "https://cdn.jsdelivr.net/npm/cesium@1.125.0/Build/Cesium/Assets/Textures/NaturalEarthII/",
    "https://fastly.jsdelivr.net/npm/cesium@1.125.0/Build/Cesium/Assets/Textures/NaturalEarthII/",
    "https://assets.cesium.com/natural-earth-ii/tiles/",
    "https://unpkg.com/cesium@1.125.0/Build/Cesium/Assets/Textures/NaturalEarthII/",
];

// ══════════════════════════════════════════════════════════════════
// 瓦片预下载批次配置
// ══════════════════════════════════════════════════════════════════

/// 预下载的卫星瓦片批次。全球统一 Esri 源，无高德叠加，无色调色差。
/// 中国区域已包含在全球范围内（不再单独下载）。
struct TileBatch {
    zoom_min: u32,
    zoom_max: u32,
    tiles_for_zoom: fn(u32) -> Vec<(u32, u32, u32)>,
    category: &'static str,
    display_name: &'static str,
    source: &'static str,
}

/// 全球底图瓦片缩放级别范围 (z3-z6)。
/// z6 约 2.5km/像素，看清大陆轮廓和国家。z7+ 走按需下载回退。
const GLOBAL_ZOOM_MIN: u32 = 3;
const GLOBAL_ZOOM_MAX: u32 = 6;

/// 全球 z3-z6 Esri 批次（统一源，含中国）。
fn global_batch() -> TileBatch {
    TileBatch {
        zoom_min: GLOBAL_ZOOM_MIN,
        zoom_max: GLOBAL_ZOOM_MAX,
        tiles_for_zoom: global_tiles_for_zoom,
        category: "global-tiles",
        display_name: "全球卫星影像",
        source: "arcgisonline.com",
    }
}

/// 在 data_dir/china-tiles/（历史目录名，复用端点+L2缓存）下载全球 Esri 批次。
fn spawn_tile_batch_downloader(tiles_dir: PathBuf, force: bool, batch: &'static str) {
    let b = match batch {
        "global" => global_batch(),
        _ => return,
    };
    if let Err(e) = std::thread::Builder::new()
        .name("global-tiles-downloader".into())
        .spawn(move || {
            let info = download_tile_batch_sync(&tiles_dir, force, &b);
            crate::logger::log(
                "asset",
                "INFO",
                &format!("瓦片后台下载完成: status={}", info.status),
            );
        })
    {
        // spawn 失败（资源耗尽等）不崩启动，但必须留痕便于排查
        crate::logger::log("asset", "ERROR", &format!("瓦片后台下载线程启动失败: {e}"));
    }
}

// ══════════════════════════════════════════════════════════════════
// 数据结构
// ══════════════════════════════════════════════════════════════════

/// 单个资源文件的状态。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetInfo {
    /// 文件名（相对路径，如 "Workers/cesiumWorkerBootstrapper.js"）
    pub name: String,
    /// 类别：texture / cesium / tiles / china-tiles / terrain
    pub category: String,
    /// 状态：cached / missing / ok / failed
    pub status: String,
    /// 本地文件大小（字节），缺失时为 0
    pub size: u64,
    /// 人类可读的大小（如 "1.4 MB"）
    pub size_human: String,
    /// 下载来源（CDN 域名），仅下载成功时有
    pub source: String,
    /// 错误信息（仅失败时）
    pub error: String,
}

/// 全部资源的状态报告。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetReport {
    /// assets 目录绝对路径
    pub assets_dir: String,
    /// 每个文件的状态
    pub assets: Vec<AssetInfo>,
    /// 是否全部就绪
    pub all_ready: bool,
}

// ── AssetInfo 工厂 — 收口 31 处字面构造 ──

/// 已缓存（本地存在）。source/error 为空。
fn cached_asset(name: &str, category: &str, size: u64) -> AssetInfo {
    AssetInfo {
        name: name.to_string(),
        category: category.into(),
        status: "cached".into(),
        size,
        size_human: crate::utils::human_size(size),
        source: String::new(),
        error: String::new(),
    }
}

/// 下载成功。size 为本次结果字节数，source 为 CDN 域名。
fn ok_asset(name: &str, category: &str, size: u64, source: &str) -> AssetInfo {
    AssetInfo {
        name: name.to_string(),
        category: category.into(),
        status: "ok".into(),
        size,
        size_human: crate::utils::human_size(size),
        source: source.to_string(),
        error: String::new(),
    }
}

/// 缺失（未下载）。size/source/error 全空。
fn missing_asset(name: &str, category: &str) -> AssetInfo {
    AssetInfo {
        name: name.to_string(),
        category: category.into(),
        status: "missing".into(),
        size: 0,
        size_human: String::new(),
        source: String::new(),
        error: String::new(),
    }
}

/// 下载失败。error 为最后一条错误。
fn failed_asset(name: &str, category: &str, error: &str) -> AssetInfo {
    AssetInfo {
        name: name.to_string(),
        category: category.into(),
        status: "failed".into(),
        size: 0,
        size_human: String::new(),
        source: String::new(),
        error: error.to_string(),
    }
}

// ══════════════════════════════════════════════════════════════════
// 辅助函数
// ══════════════════════════════════════════════════════════════════

/// 获取 ~/.aurora/ 根目录。
fn aurora_root() -> PathBuf {
    crate::data_dir::aurora_data_dir()
}

/// 检查文件是否存在且大小 > 0。
fn file_exists_and_nonempty(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(meta) => meta.len() > 0,
        Err(_) => false,
    }
}

/// 创建目录，失败时记录 WARN 日志（非致命）。
fn ensure_dir(path: &Path) {
    if let Err(e) = fs::create_dir_all(path) {
        crate::logger::log(
            "asset",
            "WARN",
            &format!("创建目录失败 (非致命): {} — {e}", path.display()),
        );
    }
}

// ══════════════════════════════════════════════════════════════════
// 公开 API
// ══════════════════════════════════════════════════════════════════

/// 查询所有资源文件的当前状态（不触发下载）。
pub fn asset_status() -> AssetReport {
    let root = aurora_root();
    let mut assets = Vec::new();
    let mut all_ready = true;

    // 纹理
    for name in TEXTURE_ASSETS {
        let target = root.join("assets").join(name);
        match fs::metadata(&target) {
            Ok(meta) if meta.len() > 0 => {
                assets.push(cached_asset(name, "texture", meta.len()));
            }
            _ => {
                all_ready = false;
                assets.push(missing_asset(name, "texture"));
            }
        }
    }

    // CesiumJS（只检查主入口文件，Worker 等随 tarball 一起解压）
    let cesium_entry = root.join("cesium").join("Cesium.js");
    match fs::metadata(&cesium_entry) {
        Ok(meta) if meta.len() > 0 => {
            let cesium_size = crate::utils::dir_size(&root.join("cesium"));
            assets.push(cached_asset("Cesium.js (库)", "cesium", cesium_size));
        }
        _ => {
            all_ready = false;
            assets.push(missing_asset("Cesium.js (库)", "cesium"));
        }
    }

    // 瓦片
    for name in TILE_FILES {
        let target = root.join("tiles").join(name);
        match fs::metadata(&target) {
            Ok(meta) if meta.len() > 0 => {
                assets.push(cached_asset(name, "tiles", meta.len()));
            }
            _ => {
                all_ready = false;
                assets.push(missing_asset(name, "tiles"));
            }
        }
    }

    // 全球卫星瓦片 (Esri z3-z6，统一源含中国)
    let tiles_dir = root.join("china-tiles");
    let batch = global_batch();
    let expected = batch_total(&batch);
    // 单次遍历同时取 (字节数, 文件数)——原先 file_count + dir_size 两次全目录递归
    let (size, existing) = if tiles_dir.exists() {
        crate::utils::dir_size_and_count(&tiles_dir)
    } else {
        (0, 0)
    };
    let batch_name = format!(
        "{} z{}-z{}",
        batch.display_name, batch.zoom_min, batch.zoom_max
    );
    if existing >= expected {
        assets.push(cached_asset(&batch_name, batch.category, size));
    } else if existing > 0 {
        all_ready = false;
        // ok 带 source + 条件 error（部分瓦片缺失）——保留字面，工厂 error 固定空不适用
        assets.push(AssetInfo {
            name: batch_name,
            category: batch.category.into(),
            status: "ok".into(),
            size,
            size_human: crate::utils::human_size(size),
            source: batch.source.into(),
            error: format!("{}/{} 瓦片", existing, expected),
        });
    } else {
        all_ready = false;
        assets.push(missing_asset(&batch_name, batch.category));
    }

    AssetReport {
        assets_dir: root.to_string_lossy().to_string(),
        assets,
        all_ready,
    }
}

/// 启动时确保核心资源就绪（一键启动）。
/// 快速资源（纹理 + CesiumJS + z0-z2 瓦片）同步下载，约 25MB 几秒搞定。
/// 全球卫星瓦片（Esri z3-z6）由 download_assets_async 后台下载，不在此重复触发，
/// 避免与 download_assets_async 并发写同一目录。
pub fn ensure_all_resources() {
    let root = aurora_root();

    // ── 纹理 ──
    let assets_dir = root.join("assets");
    ensure_dir(&assets_dir);
    for name in TEXTURE_ASSETS {
        let target = assets_dir.join(name);
        if file_exists_and_nonempty(&target) {
            crate::logger::log("asset", "INFO", &format!("✓ {} 已存在", name));
            continue;
        }
        let info = download_texture(name, &assets_dir, false);
        if info.status == "failed" {
            crate::logger::log("asset", "ERROR", &format!("✗ {} 下载失败", name));
        }
    }

    // ── CesiumJS ──
    let cesium_dir = root.join("cesium");
    let cesium_entry = cesium_dir.join("Cesium.js");
    if file_exists_and_nonempty(&cesium_entry) {
        let size = crate::utils::dir_size(&cesium_dir);
        crate::logger::log(
            "asset",
            "INFO",
            &format!("✓ CesiumJS 已存在 ({})", crate::utils::human_size(size)),
        );
    } else {
        crate::logger::log("asset", "INFO", "↓ CesiumJS 缺失，开始下载...");
        let info = download_cesium_tarball(&cesium_dir, false);
        if info.status == "failed" {
            crate::logger::log("asset", "ERROR", "✗ CesiumJS 下载失败");
        }
    }

    // ── 瓦片 (Natural Earth II z0-z2) ──
    let tiles_dir = root.join("tiles");
    ensure_dir(&tiles_dir);
    let tiles_missing = TILE_FILES
        .iter()
        .any(|name| !file_exists_and_nonempty(&tiles_dir.join(name)));
    if tiles_missing {
        crate::logger::log("asset", "INFO", "↓ 影像瓦片缺失，开始下载...");
        for name in TILE_FILES {
            let urls = tile_urls(name);
            let info = download_one(name, &tiles_dir, &urls, "tiles", false);
            if info.status == "failed" {
                crate::logger::log("asset", "WARN", &format!("✗ 瓦片 {} 下载失败", name));
            }
        }
    } else {
        crate::logger::log("asset", "INFO", "✓ 影像瓦片已存在");
    }
}

/// 补全缺失的全球瓦片（后台线程，不阻塞 UI）。
///
/// 用于"刷新地球"按钮：不清空已有瓦片，仅以 force=false 跑一次下载，
/// download_batch 会跳过已存在文件，只补缺失/失败的瓦片。
/// 立即返回，前端通过 get_asset_status 轮询进度。
pub fn refresh_tiles() {
    let tiles_dir = aurora_root().join("china-tiles");
    spawn_tile_batch_downloader(tiles_dir, false, "global");
}

/// 手动触发下载（UI 安全版）。
/// 快速资源（纹理 + CesiumJS + z0-z2 瓦片）同步下载后立即返回，
/// 中国瓦片在后台线程下载，不阻塞 UI。
/// 前端通过 get_asset_status 轮询进度。
pub fn download_assets_async(force: bool) -> AssetReport {
    let root = aurora_root();
    let mut assets = Vec::new();
    let mut all_ready = true;

    // ── 快速资源：纹理（~3MB，几秒）──
    let assets_dir = root.join("assets");
    ensure_dir(&assets_dir);
    for name in TEXTURE_ASSETS {
        let info = download_texture(name, &assets_dir, force);
        if info.status == "missing" || info.status == "failed" {
            all_ready = false;
        }
        assets.push(info);
    }

    // ── 快速资源：CesiumJS（~30MB，几十秒）──
    let cesium_dir = root.join("cesium");
    let cesium_entry = cesium_dir.join("Cesium.js");
    let need_cesium = force || !file_exists_and_nonempty(&cesium_entry);
    if need_cesium {
        ensure_dir(&cesium_dir);
        let info = download_cesium_tarball(&cesium_dir, force);
        if info.status == "missing" || info.status == "failed" {
            all_ready = false;
        }
        assets.push(info);
    } else {
        let cesium_size = crate::utils::dir_size(&cesium_dir);
        assets.push(cached_asset("Cesium.js (库)", "cesium", cesium_size));
    }

    // ── 快速资源：z0-z2 瓦片（~252KB，瞬间）──
    let tiles_dir = root.join("tiles");
    ensure_dir(&tiles_dir);
    for name in TILE_FILES {
        let urls = tile_urls(name);
        let info = download_one(name, &tiles_dir, &urls, "tiles", force);
        if info.status == "missing" || info.status == "failed" {
            all_ready = false;
        }
        assets.push(info);
    }

    // ── 全球瓦片（Esri z3-z6，含中国）：后台线程，不阻塞 ──
    // 全球统一 Esri 源，无高德叠加，无色调色差。中国区域已包含在全球内。
    let tiles_dir = root.join("china-tiles");
    let batch = global_batch();
    let global_expected = batch_total(&batch);
    let (global_size, global_existing) = if tiles_dir.exists() {
        crate::utils::dir_size_and_count(&tiles_dir)
    } else {
        (0, 0)
    };
    let global_done = !force && global_existing >= global_expected;

    if global_done {
        assets.push(cached_asset(
            &format!(
                "{} z{}-z{}",
                batch.display_name, batch.zoom_min, batch.zoom_max
            ),
            batch.category,
            global_size,
        ));
    } else {
        all_ready = false;
        assets.push(AssetInfo {
            name: format!(
                "{} z{}-z{}",
                batch.display_name, batch.zoom_min, batch.zoom_max
            ),
            category: batch.category.into(),
            status: "ok".into(), // "下载中"
            size: 0,
            size_human: format!("{}/{}", global_existing, global_expected),
            source: "后台下载中".into(),
            error: String::new(),
        });

        spawn_tile_batch_downloader(tiles_dir.clone(), force, "global");
    }

    AssetReport {
        assets_dir: root.to_string_lossy().to_string(),
        assets,
        all_ready,
    }
}

/// 为 z0-z2 瓦片构造完整下载 URL 列表（mirror + 文件名）。
fn tile_urls(name: &str) -> Vec<String> {
    TILE_MIRRORS
        .iter()
        .map(|m| format!("{}{}", m, name))
        .collect()
}

/// 为纹理文件构造完整下载 URL 列表。
/// earth-topo-bathy.jpg 不在 three-globe 仓库中，使用完整的专用 URL；
/// 其余纹理在 mirror 后追加文件名。
fn texture_urls(name: &str) -> Vec<String> {
    if name == "earth-topo-bathy.jpg" {
        TOPO_BATHY_MIRRORS.iter().map(|s| s.to_string()).collect()
    } else {
        TEXTURE_MIRRORS
            .iter()
            .map(|m| format!("{}{}", m, name))
            .collect()
    }
}

/// 下载纹理资源，earth-topo-bathy.jpg 使用专用源。
fn download_texture(name: &str, target_dir: &Path, force: bool) -> AssetInfo {
    let urls = texture_urls(name);
    download_one(name, target_dir, &urls, "texture", force)
}

/// 从 URL 提取 CDN 域名（去掉 https:// 前缀和路径），用于 AssetInfo.source。
fn source_from_url(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_end_matches('/')
        .split('/')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// 单文件多源下载。urls 为已构造的完整 URL 列表，按序尝试，成功即停。
/// 下载到 .tmp 后 rename 保证原子性。已缓存则跳过（force 时重下）。
fn download_one(
    name: &str,
    target_dir: &Path,
    urls: &[String],
    category: &str,
    force: bool,
) -> AssetInfo {
    let target = target_dir.join(name);
    if let Some(parent) = target.parent() {
        ensure_dir(parent);
    }
    if force && target.exists() {
        crate::logger::log("asset", "INFO", &format!("强制重新下载: {}", name));
        let _ = fs::remove_file(&target);
    }
    if file_exists_and_nonempty(&target) {
        let size = fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
        return cached_asset(name, category, size);
    }

    crate::logger::log("asset", "INFO", &format!("↓ 下载 {}...", name));
    let mut last_error = String::new();
    let mut success = None::<(u64, String)>;

    for (i, url) in urls.iter().enumerate() {
        crate::logger::log(
            "asset",
            "INFO",
            &format!("  源 {}/{}: {}", i + 1, urls.len(), url),
        );
        match download_file(url, &target) {
            Ok(size) => {
                crate::logger::log(
                    "asset",
                    "INFO",
                    &format!("✓ {} 下载完成 ({} bytes)", name, size),
                );
                success = Some((size, source_from_url(url)));
                break;
            }
            Err(e) => {
                last_error = format!("源{}: {}", i + 1, e);
                crate::logger::log("asset", "WARN", &format!("  失败: {e}"));
            }
        }
    }

    match success {
        Some((size, source)) => ok_asset(name, category, size, &source),
        None => {
            crate::logger::log("asset", "ERROR", &format!("✗ {} 所有源均失败", name));
            failed_asset(name, category, &last_error)
        }
    }
}

/// 从 URL 下载文件到目标路径。
/// 下载到 .tmp 后 rename，保证原子性。
fn download_file(url: &str, target: &Path) -> anyhow::Result<u64> {
    let tmp = target.with_extension("tmp");

    let agent = ureq::Agent::new_with_defaults();
    let response = agent.get(url).call()?;
    let mut reader = response.into_body().into_reader();

    let mut file = fs::File::create(&tmp)?;
    let written = std::io::copy(&mut reader, &mut file)?;
    file.flush()?;

    // 原子性 rename
    fs::rename(&tmp, target)?;

    Ok(written)
}

/// 下载 CesiumJS npm tarball 并解压到 cesium_dir。
/// tarball 内部路径为 package/Build/Cesium/，解压时去掉前缀。
fn download_cesium_tarball(cesium_dir: &Path, force: bool) -> AssetInfo {
    const NAME: &str = "Cesium.js (库)";
    const CATEGORY: &str = "cesium";
    let entry = cesium_dir.join("Cesium.js");

    if !force && file_exists_and_nonempty(&entry) {
        return cached_asset(NAME, CATEGORY, crate::utils::dir_size(cesium_dir));
    }

    // force 模式清空目录
    if force && cesium_dir.exists() {
        let _ = fs::remove_dir_all(cesium_dir);
    }
    ensure_dir(cesium_dir);

    crate::logger::log("asset", "INFO", "↓ 下载 CesiumJS tarball...");
    let mut last_error = String::new();
    let mut success_source = None::<String>;

    for (i, url) in CESIUM_TARBALL_MIRRORS.iter().enumerate() {
        crate::logger::log(
            "asset",
            "INFO",
            &format!("  源 {}/{}: {}", i + 1, CESIUM_TARBALL_MIRRORS.len(), url),
        );

        match download_and_extract_tarball(url, cesium_dir) {
            Ok(()) => {
                crate::logger::log("asset", "INFO", "✓ CesiumJS 解压完成");
                success_source = Some(source_from_url(url));
                break;
            }
            Err(e) => {
                last_error = format!("源{}: {}", i + 1, e);
                crate::logger::log("asset", "WARN", &format!("  失败: {e}"));
            }
        }
    }

    match success_source {
        Some(source) => ok_asset(NAME, CATEGORY, crate::utils::dir_size(cesium_dir), &source),
        None => {
            crate::logger::log("asset", "ERROR", "✗ CesiumJS 所有源均失败");
            failed_asset(NAME, CATEGORY, &last_error)
        }
    }
}

/// 下载 npm tarball (.tgz) 并解压到目标目录。
/// tarball 内部路径格式：package/Build/Cesium/...
/// 只提取 Build/Cesium/ 下的内容，去掉 package/Build/Cesium/ 前缀。
fn download_and_extract_tarball(url: &str, target_dir: &Path) -> anyhow::Result<()> {
    // 下载到临时文件
    let tmp_tgz = target_dir.join("_download.tgz");
    let response = ureq::get(url).call()?;
    let mut reader = response.into_body().into_reader();
    let mut file = fs::File::create(&tmp_tgz)?;
    std::io::copy(&mut reader, &mut file)?;
    file.flush()?;

    // 解压 .tgz (gzip + tar)
    let tgz_file = fs::File::open(&tmp_tgz)?;
    let gz_decoder = flate2::read::GzDecoder::new(tgz_file);
    let mut archive = tar::Archive::new(gz_decoder);

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?;

        // 只提取 Build/Cesium/ 下的文件
        // tarball 内路径：package/Build/Cesium/...
        let path_str = path.to_string_lossy();
        let cesium_prefix = "package/Build/Cesium/";

        if let Some(relative) = path_str.strip_prefix(cesium_prefix) {
            if relative.is_empty() {
                continue;
            }
            let target = target_dir.join(relative);
            if let Some(parent) = target.parent() {
                ensure_dir(parent);
            }
            let mut file = fs::File::create(&target)?;
            std::io::copy(&mut entry, &mut file)?;
        }
    }

    // 清理临时文件
    let _ = fs::remove_file(&tmp_tgz);

    Ok(())
}

// ══════════════════════════════════════════════════════════════════
// 瓦片批量下载（统一实现，供 china/global 复用）
// ══════════════════════════════════════════════════════════════════

/// 生成全球在指定缩放级别的所有瓦片坐标 (z, x, y)，含中国。
/// 全球统一 Esri 源，中国区域已包含（无高德叠加）。
fn global_tiles_for_zoom(zoom: u32) -> Vec<(u32, u32, u32)> {
    let max = 1u32 << zoom;
    let mut tiles = Vec::new();
    for x in 0..max {
        for y in 0..max {
            tiles.push((zoom, x, y));
        }
    }
    tiles
}

/// 计算批次 z_min..=z_max 的总瓦片数。
fn batch_total(batch: &TileBatch) -> u64 {
    let mut total: u64 = 0;
    for z in batch.zoom_min..=batch.zoom_max {
        total += (batch.tiles_for_zoom)(z).len() as u64;
    }
    total
}

/// 异步批量下载一个瓦片批次（16 并发，跳过已存在文件）。
/// 调用方需在 tokio runtime 上下文中调用。
async fn download_tile_batch_async(
    dir: &Path,
    force: bool,
    downloader: &crate::tile_downloader::TileDownloader,
    batch: &TileBatch,
) -> AssetInfo {
    ensure_dir(dir);

    let (size, existing_count) = crate::utils::dir_size_and_count(dir);
    let expected_count = batch_total(batch);

    let batch_name = format!(
        "{} z{}-z{}",
        batch.display_name, batch.zoom_min, batch.zoom_max
    );

    if !force && existing_count >= expected_count {
        crate::logger::log(
            "asset",
            "INFO",
            &format!(
                "✓ {} 已存在 ({} 个, {})",
                batch.display_name,
                existing_count,
                crate::utils::human_size(size)
            ),
        );
        return cached_asset(&batch_name, batch.category, size);
    }

    crate::logger::log(
        "asset",
        "INFO",
        &format!(
            "↓ 下载 {} (z{}-z{}, 约 {} 个瓦片, 16 并发)...",
            batch.display_name, batch.zoom_min, batch.zoom_max, expected_count
        ),
    );

    let mut tiles_to_download: Vec<(u32, u32, u32)> = Vec::new();
    let mut skipped: u64 = 0;

    for z in batch.zoom_min..=batch.zoom_max {
        let tiles = (batch.tiles_for_zoom)(z);
        crate::logger::log("asset", "INFO", &format!("  z{z}: {} 个瓦片", tiles.len()));

        for (zoom, x, y) in &tiles {
            let rel_path = format!("{zoom}/{x}/{y}.jpg");
            let target = dir.join(&rel_path);

            if !force && file_exists_and_nonempty(&target) {
                skipped += 1;
                continue;
            }

            tiles_to_download.push((*zoom, *x, *y));
        }
    }

    // 节奏优先：6 并发（原 16 对 Esri 太猛易限流），配合 download_batch 内
    // 的指数退避重试。首次预加载稍慢但稳。
    let (downloaded, failed, _) = downloader
        .download_batch(&tiles_to_download, dir, force, 6, 100)
        .await;

    crate::logger::log(
        "asset",
        "INFO",
        &format!(
            "{} 下载完成: 下载 {downloaded}, 跳过 {skipped}, 失败 {failed}",
            batch.display_name
        ),
    );

    let (size, current_count) = crate::utils::dir_size_and_count(dir);

    if current_count > 0 {
        // ok 带条件 error（部分失败）——保留字面，工厂 error 固定空不适用
        AssetInfo {
            name: batch_name,
            category: batch.category.into(),
            status: "ok".into(),
            size,
            size_human: crate::utils::human_size(size),
            source: batch.source.into(),
            error: if failed > 0 {
                format!("{failed} 个瓦片下载失败")
            } else {
                String::new()
            },
        }
    } else {
        crate::logger::log(
            "asset",
            "ERROR",
            &format!("✗ {} 下载全部失败", batch.display_name),
        );
        failed_asset(&batch_name, batch.category, "所有瓦片下载失败")
    }
}

/// 同步版本 — 内部创建 tokio runtime。
fn download_tile_batch_sync(dir: &Path, force: bool, batch: &TileBatch) -> AssetInfo {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime for tile batch download");

    let downloader = crate::tile_downloader::TileDownloader::new();
    rt.block_on(download_tile_batch_async(dir, force, &downloader, batch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_tiles_covers_all() {
        // z3 全球 8x8=64 张，含中国（全球统一 Esri）
        let tiles = global_tiles_for_zoom(3);
        assert_eq!(
            tiles.len(),
            64,
            "z3 应覆盖全部 64 张瓦片, got {}",
            tiles.len()
        );
    }

    #[test]
    fn test_global_tiles_z6_count_reasonable() {
        let tiles = global_tiles_for_zoom(6);
        // z6 全球 64x64=4096
        assert_eq!(tiles.len(), 4096, "z6 瓦片数异常: {}", tiles.len());
    }
}
