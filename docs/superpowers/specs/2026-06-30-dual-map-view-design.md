# Aurora 双地图视图 — Cesium 3D 全球底图 + MapLibre 2D PMTiles 矢量底图

**日期**: 2026-06-30
**状态**: 设计中，待用户审核
**目标**: 解决"程序启动后中国之外空白"问题；新增 2D 矢量地图视图，复用 worldmonitor 的 PMTiles 离线底图栈

---

## 0. 背景与问题

### 0.1 现状
- **3D 地球**（`Earth.tsx`）：CesiumJS。底图是 OSM 在线瓦片（`tile.openstreetmap.org`），中国区域叠加高德卫星（走代理 `china-tiles/` 预下载，z3-z10，约 575MB）。
- **空白根因**：中国之外的底图靠运行时在线拉取 OSM。无网/慢/被墙时，3D 地球中国之外一片空白。中国有预下载所以正常。
- **代理服务器**（`proxy_server.rs`）：actix-web，`127.0.0.1:21337`。`serve_tile` 端点读 `china-tiles/{z}/{x}/{y}.jpg`，未命中时 `downloader.download()` 回退下载——`select_sources()` 对中国返回高德、对全球返回 **Esri World Imagery**（`bbox: None`，全球覆盖）。L1(moka 256MB) + L2(文件 50GB) 两级缓存。
- **数据目录**：`aurora_data_dir()`（`data_dir.rs`），优先级 `AURORA_DATA_DIR` 环境变量 > 项目根 `aurora-data/` > exe 同目录。dev 下即 `E:\trit-core\aurora-data\`。

### 0.2 可用资源
已从 worldmonitor 剪切到 `E:\trit-core\aurora-data\`：
- `basemap.pmtiles`（136GB）— Protomaps 矢量瓦片单文件存档，OSM 派生，z0-z15，图层 `earth/boundaries/buildings/landcover/landuse/places/...`，MVT 格式，gzip 压缩。
- 待搬：`fonts/`（13MB，Noto Sans 系列，PBF 字体）、`sprites/v4/`（112KB，light/dark + @2x）。来源 `E:\worldmonitor\public\fonts` 和 `E:\worldmonitor\public\sprites`。

### 0.3 worldmonitor 验证过的栈
worldmonitor 用 **MapLibre GL JS + `pmtiles` npm 包（`Protocol`）+ `@protomaps/basemaps`（`layers()` 运行时生成样式）** 渲染 PMTiles，前端靠 HTTP Range 请求随机读取本地文件。样式不是静态 JSON，是 `@protomaps/basemaps` 按 flavor（light/dark）动态生成。**worldmonitor 是 2D 渲染，不是 Cesium 3D。**

---

## 1. 总体架构

三个独立、松耦合的改动：

```
┌─ A. Cesium 3D 地球底图修复（解决空白）──────────────────┐
│  底图: OSM 在线 → Esri 卫星预下载（复用 china-tiles 端点）   │
│  叠加: 中国高德卫星（不变，高分辨率补充）                    │
│  结果: 全球 z3-z6 启动即有底图，不空白                       │
└──────────────────────────────────────────────────────────┘
┌─ B. 2D MapLibre 矢量面板（新增视图）────────────────────┐
│  数据: aurora-data/basemap.pmtiles（136GB）                 │
│  渲染: maplibre-gl + pmtiles 协议 + @protomaps/basemaps     │
│  服务: actix 代理新增 Range 端点 + 静态 fonts/sprites       │
│  结果: 离线矢量地图，z0-z15，有地名/图层                     │
└──────────────────────────────────────────────────────────┘
┌─ C. 文件搬运（已部分完成）──────────────────────────────┐
│  basemap.pmtiles → aurora-data/  （✓ 已剪切）               │
│  fonts/ + sprites/ → aurora-data/  （待搬）                 │
└──────────────────────────────────────────────────────────┘
```

**设计决策**：
- 2D 面板与 3D 地球是**独立视图，不强制同步相机**。worldmonitor 都没解决 MapLibre+Cesium 相机同步的滑动错位问题，不重造。用户在 UI 切换或并列。
- A 和 B 互不依赖，可独立实施、独立验证。A 优先（解决你最初的空白诉求）。
- 136GB 文件**手动剪切**，代码只检测存在性，不自动移动。

---

## 2. 改动 A — Cesium 3D 底图换 Esri 预下载

### 2.1 关键发现：端点已支持全球
现有 `/china-tiles/{z}/{x}/{y}.jpg` 端点的下载回退走 `select_sources()`：中国→高德，全球→Esri。所以**这个端点已经能服务全球 Esri 瓦片**，只是按需下载、不预下载。预下载只需把全球 z3-z6 的瓦片提前写入 `china-tiles/` 目录，端点和缓存逻辑完全复用。

### 2.2 后端：新增全球瓦片预下载
**文件**：`src-tauri/src/asset_fetcher.rs`

仿 `download_china_tiles_async` 新增 `download_global_tiles_async`：
- 遍历 z3-z6 全球瓦片坐标 `(z, x, y)`。
- **排除中国 bbox**（lat 15-55, lng 70-140），避免与高德重复存储。用 `tile_sources::select_sources` 判断：若第一个源是"高德卫星"则跳过。
- 用 `downloader.download_batch(&tiles, china_dir, false, 16, 100)` 写入**同一个 `china-tiles/` 目录**（复用端点 + L2 缓存 key `china-tiles/{z}/{x}/{y}.jpg`）。
- 跳过已存在文件（`download_batch` 已实现）。

**常量**（新增）：
```rust
const GLOBAL_ZOOM_MIN: u32 = 3;
const GLOBAL_ZOOM_MAX: u32 = 6;  // 约 4-5 万张瓦片，减去中国部分，几百 MB
```

**全球瓦片坐标生成**：
```rust
fn global_tiles_for_zoom(zoom: u32) -> Vec<(u32, u32, u32)> {
    let max = 1u32 << zoom;
    let mut tiles = Vec::new();
    for x in 0..max {
        for y in 0..max {
            // 排除中国区域（高德已覆盖）
            if tile_sources::select_sources(zoom, x, y)
                .first()
                .map(|s| s.name == "高德卫星")
                .unwrap_or(false) { continue; }
            tiles.push((zoom, x, y));
        }
    }
    tiles
}
```

**接入启动流**（`ensure_all_resources`，line 393 附近）：在中国瓦片后台下载之后，再起一个后台线程下载全球瓦片：
```rust
let global_dir = root.join("china-tiles");  // 同目录
std::thread::Builder::new()
    .name("global-tiles-downloader".into())
    .spawn(move || {
        let info = download_global_tiles(&global_dir, false);
        // log
    })
```

**AssetInfo category**：新增 `"global-tiles"`，前端 `get_asset_status` 报告里区分。

### 2.3 前端：Cesium 底图换 Esri
**文件**：`ui/src/Earth.tsx`（line 251-254）

```typescript
// 旧: OSM 在线
// const osmProvider = new Cesium.OpenStreetMapImageryProvider({ url: 'https://tile.openstreetmap.org/', maximumLevel: 18 });

// 新: Esri 卫星走代理（复用 china-tiles 端点，全球 z3-z6 预下载，更高 zoom 按需下载）
const baseProvider = new Cesium.UrlTemplateImageryProvider({
  url: `${RESOURCE_SERVER}/china-tiles/{z}/{x}/{y}.jpg`,
  tilingScheme: new Cesium.WebMercatorTilingScheme(),
  minimumLevel: 3,
  maximumLevel: 18,  // z7+ 按需下载回退 Esri
});
```

中国高德叠加层（line 277）**保留**——中国区域高德 z3-z10 已预下载，作为高分辨率覆盖在 Esri 之上。两者同一端点，`select_sources` 自动按区域选源。

### 2.4 验证
- `cargo test` 现有瓦片测试不破。
- 新增测试：`global_tiles_for_zoom` 排除中国区域、z6 瓦片数合理。
- 手动：启动后 3D 地球中国之外不再空白（z3-z6 预下载完成前靠按需下载，完成后秒开）。

---

## 3. 改动 B — 2D MapLibre PMTiles 面板

### 3.1 后端：Range 端点 + 静态资源
**文件**：`src-tauri/src/proxy_server.rs`

现有 `serve_static` 整文件读取、无 Range。PMTiles 协议靠 HTTP Range 随机读取。新增 Range 感知端点：

```rust
/// PMTiles 文件 — 支持 HTTP Range 请求。
/// 路径: /pmtiles/basemap.pmtiles
#[get("/pmtiles/basemap.pmtiles")]
async fn serve_pmtiles(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let file_path = state.data_dir.join("basemap.pmtiles");
    // 安全: canonicalize + starts_with(data_dir)
    // 解析 Range: bytes=start-end 头
    // 命中: 206 Partial Content + Content-Range + 读取 [start, end]
    // 无 Range 头: 200 + 整文件（PMTiles 首次会读 header）
    // 文件缺失: 404 + 日志提示用户手动放置
}
```

实现要点：
- `req.headers().get("range")` 解析 `bytes=a-b`。
- `std::fs::File::seek + read_exact` 偏移读取，不整文件加载（136GB）。
- 返回 `206` + `Content-Range: bytes a-b/total` + `Accept-Ranges: bytes`。
- 文件不存在时返回 404，日志明确提示"basemap.pmtiles 缺失，2D 矢量地图不可用"。

**fonts/sprites 静态服务**：扩展 `serve_static` 的前缀白名单：
```rust
// 旧: /{prefix:(cesium|assets|terrain|tiles)}/{rest:.*}
// 新: /{prefix:(cesium|assets|terrain|tiles|fonts|sprites)}/{rest:.*}
```
fonts/sprites 放 `aurora-data/fonts/` 和 `aurora-data/sprites/`，代理直接静态服务。

### 3.2 前端：MapPanel 组件
**新文件**：`ui/src/MapPanel.tsx`

照搬 worldmonitor `basemap-styles.ts` 模式，懒加载：
```typescript
import { useEffect, useRef } from 'react';
const RESOURCE_SERVER = 'http://localhost:21337';

let registered = false;
async function registerPMTilesProtocol() {
  if (registered) return;
  const { Protocol } = await import('pmtiles');
  const maplibregl = (await import('maplibre-gl')).default;
  const protocol = new Protocol();
  maplibregl.addProtocol('pmtiles', protocol.tile);
  registered = true;
}

async function buildPMTilesStyle(flavor: 'light' | 'dark') {
  const { layers, namedFlavor } = await import('@protomaps/basemaps');
  const spriteName = flavor === 'light' ? 'light' : 'dark';
  return {
    version: 8,
    glyphs: `${RESOURCE_SERVER}/fonts/{fontstack}/{range}.pbf`,
    sprite: `${RESOURCE_SERVER}/sprites/v4/${spriteName}`,
    sources: {
      basemap: {
        type: 'vector',
        url: `pmtiles://${RESOURCE_SERVER}/pmtiles/basemap.pmtiles`,
        maxzoom: 15,
      },
    },
    layers: layers('basemap', namedFlavor(flavor), { lang: 'en' }),
  };
}

export default function MapPanel({ flavor = 'light' }: { flavor?: 'light' | 'dark' }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let map: any;
    (async () => {
      await registerPMTilesProtocol();
      const maplibregl = (await import('maplibre-gl')).default;
      const style = await buildPMTilesStyle(flavor);
      map = new maplibregl.Map({ container: ref.current!, style, center: [116, 36], zoom: 3 });
    })();
    return () => map?.remove();
  }, [flavor]);
  return <div ref={ref} style={{ width: '100%', height: '100%' }} />;
}
```

**依赖**（`ui/package.json`）：
```json
"maplibre-gl": "^4.x",
"pmtiles": "^4.4.0",
"@protomaps/basemaps": "^5.7.1"
```
三者都动态 import，不进主 bundle（参考 worldmonitor `vite.config.ts` 的 manualChunks）。

### 3.3 UI 集成
**文件**：`ui/src/App.tsx`

加 2D/3D 视图切换（原则，具体布局实施时定）：
- 一个 tab/按钮切换 `Earth`（3D）与 `MapPanel`（2D）。
- 或并列分屏。设计阶段定原则：默认 3D，2D 可选打开。
- `MapPanel` 仅在 PMTiles 文件就绪时可用；缺失时禁用切换按钮 + 提示。

### 3.4 文件搬运（剩余）
手动剪切到 `E:\trit-core\aurora-data\`：
- `E:\worldmonitor\public\fonts` → `aurora-data/fonts`（13MB）
- `E:\worldmonitor\public/sprites` → `aurora-data/sprites`（112KB）

（basemap.pmtiles 已搬完。）

### 3.5 验证
- 后端：`/pmtiles/basemap.pmtiles` Range 请求返回 206 + 正确字节（用 curl 测 `Range: bytes=0-127`，应返回 header 127 字节）。
- 前端：2D 面板加载后显示矢量地图，有地名、缩放清晰。
- 缺失场景：删 pmtiles 文件，端点返回 404，前端禁用 2D 切换。

---

## 4. 实施顺序

| Phase | 内容 | 依赖 |
|-------|------|------|
| **P1** | 改动 A（3D 底图换 Esri 预下载） | 无 — 解决核心空白诉求，优先 |
| **P2** | 搬运 fonts/sprites | 无 |
| **P3** | 改动 B 后端（Range 端点 + 静态白名单） | P2 |
| **P4** | 改动 B 前端（MapPanel + 依赖 + UI 切换） | P3 |

P1 可独立交付。P2-P4 是 2D 面板，整体交付。

---

## 5. 风险与未决

1. **Esri 全球瓦片下载耗时**：z3-z6 约 4-5 万张，16 并发，首次可能数十分钟。后台进行不阻塞启动；期间靠按需下载回退。可调 `GLOBAL_ZOOM_MAX`。
2. **Esri 访问性**：国内访问 `server.arcgisonline.com` 可能慢/不稳。现有 `select_sources` 已配置，失败时该瓦片空白。若普遍失败需加备用源——本次不做，观察到再说。
3. **PMTiles Range 端点性能**：每次瓦片请求触发多次 Range 读取（root dir → leaf dir → tile data）。actix 同步读取 + 文件 seek，单文件 136GB 无问题（OS page cache 会热 root/leaf dir）。若高 zoom 频繁请求卡顿，再考虑内存缓存目录——本次不做。
4. **@protomaps/basemaps 样式图层匹配**：worldmonitor 的 PMTiles 图层（earth/boundaries/buildings/...）是 Protomaps 标准图层，`@protomaps/basemaps` 的 `layers()` 专为它设计，应匹配。若个别图层不显示，调样式——实施时验证。
5. **2D/3D 不同步**：明确不同步，避免复杂度。若后续要同步，单独立项。
6. **磁盘空间**：aurora-data 已有 136GB pmtiles + 575MB 中国瓦片；P1 再加几百 MB Esri。确认 E 盘空间。

---

## 6. 不做的事（YAGNI）

- 不写 Rust PMTiles 读取器（前端 pmtiles 协议已够，后端只做 Range 透传）。
- 不在后端栅格化 MVT（Cesium 3D 用 Esri 栅格，不碰矢量）。
- 不做 2D/3D 相机同步。
- 不加 tileserver-gl sidecar（2D 走前端矢量渲染，3D 走 Esri，无需 sidecar）。
- 不为 Esri 加备用源（观察到失败再加）。
