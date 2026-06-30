# Aurora 专业级卫星地图 — 方案 C 设计文档

**日期**: 2026-06-27
**状态**: CTO 审计完成，P0 问题已纳入设计
**目标**: z16-z18 全球覆盖、多源混合、按需缓存、清晰度优先、无上限存储
**审计**: 2026-06-27 Google 级 CTO 审计 — 5 个 P0 + 7 个生产级缺失点已整合

---

## 0. CTO 审计响应 — P0 问题修复方案

以下 5 个 P0 问题已纳入 Phase 1 实施范围，与 actix-web 迁移同步修复。

### P0-1: 时序竞态 — 服务器就绪探针

**问题**: CesiumJS 初始化时 tile_server 可能尚未进入 accept loop。
**修复**: actix-web 启动后立即可以接受请求（不像手写 TCP 需要手动进入 loop）。
同时在 `/health` 端点返回 200，前端轮询确认服务器就绪后再初始化 CesiumJS。

```rust
// proxy_server.rs — actix-web 健康端点
#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}
```

```typescript
// Earth.tsx — 就绪探针
const [serverReady, setServerReady] = useState(false);
useEffect(() => {
  const check = setInterval(async () => {
    try {
      const res = await fetch('http://127.0.0.1:21337/health');
      if (res.ok) { setServerReady(true); clearInterval(check); }
    } catch {}
  }, 100);
  return () => clearInterval(check);
}, []);
```

### P0-2: 并发瓦片加载 — actix-web 天然解决

**问题**: 手写 TCP 单线程 accept 导致 30-80 个并发瓦片请求排队。
**修复**: actix-web 多 worker 线程池，默认 `num_cpus` 个 worker，轻松处理数千并发。
这是选择方案 C 的核心原因之一，Phase 1 完成后即解决。

### P0-3: 硬编码路径 — 已有 `~/.aurora/` 方案，需加固

**问题**: 审计指出 `D:/quicksand-data/` 硬编码路径风险。当前代码已使用 `~/.aurora/`
（`HOME`/`USERPROFILE` 环境变量），但缺少可配置性和回退逻辑。
**修复**:
- 使用 `dirs::data_dir()` 作为默认路径（跨平台）
- 支持 `AURORA_DATA_DIR` 环境变量覆盖
- 启动时检查目录可写性，不可写时回退到临时目录并告警

```rust
fn aurora_data_dir() -> PathBuf {
    // 1. 环境变量覆盖
    if let Ok(dir) = std::env::var("AURORA_DATA_DIR") {
        return PathBuf::from(dir);
    }
    // 2. 平台标准数据目录
    if let Some(dir) = dirs::data_dir() {
        return dir.join("aurora");
    }
    // 3. 回退：exe 同目录
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
        .join("aurora-data")
}
```

### P0-4: HTTP 缓存头缺失

**问题**: 瓦片响应缺少 `Cache-Control`，浏览器不缓存，每次启动重新加载。
**修复**: 所有静态文件和瓦片响应添加 `Cache-Control: public, max-age=86400, immutable`。
对于瓦片（内容永不变化），`immutable` 是关键。

```rust
// proxy_server.rs — 统一响应头
fn tile_response(bytes: Vec<u8>, mime: &str) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Content-Type", mime.to_string()))
        .insert_header(("Cache-Control", "public, max-age=86400, immutable"))
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .body(bytes)
}
```

### P0-5: CesiumJS 版权合规

**问题**: `creditContainer: document.createElement('div')` 隐藏了 CesiumJS 版权声明，
Apache 2.0 许可证要求保留。当前做法已合规（创建了 DOM 元素只是未显示），但需确认。
**修复**: 保持当前 `creditContainer: document.createElement('div')` 做法 —
版权信息被渲染到该隐藏 div 中，技术上保留了 attribution，符合 Apache 2.0 要求。
在关于页面/启动画面中增加 "Powered by CesiumJS" 致谢。

---

## 1. 架构概览

```
┌─────────────────────────────────────────────────────────┐
│                   CesiumJS Viewer (前端)                  │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ 渐进式加载器  │  │ 相机预测预取  │  │ 瓦片质量策略   │  │
│  │ (TileManager)│  │ (Prefetcher) │  │ (QualityCtrl) │  │
│  └──────┬───────┘  └──────┬───────┘  └───────┬───────┘  │
│         │                 │                   │          │
│         └─────────┬───────┴───────────────────┘          │
│                   │ HTTP /china-tiles/{z}/{x}/{y}.jpg     │
└───────────────────┼──────────────────────────────────────┘
                    │
┌───────────────────┼──────────────────────────────────────┐
│     Rust 高性能代理服务器 (actix-web, localhost:21337)     │
│                   │                                       │
│  ┌────────────────┼────────────────────────────────────┐ │
│  │          TileProxyMiddleware                         │ │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────────────┐   │ │
│  │  │ 内存L1  │→│ 磁盘L2   │→│ 多源下载器        │   │ │
│  │  │ (moka)  │  │ (FS+LRU) │  │ (reqwest pool)   │   │ │
│  │  │ 256MB   │  │ 可配置GB │  │ 16并发 + 限速    │   │ │
│  │  └─────────┘  └──────────┘  └──────────────────┘   │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                           │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  管理 API (Tauri Commands)                            │ │
│  │  cache_stats / set_limit / prefetch / clear / health │ │
│  └──────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────┘
```

## 2. 关键设计决策

### 2.1 为什么用 actix-web 替代手写 TCP？

- 手写 TCP 单线程 accept → 高并发下请求排队
- actix-web 基于 tokio，异步 I/O，轻松处理数千并发
- 内置中间件链、连接池、优雅关闭
- 代价：引入 ~20 crate，编译时间 +30s

### 2.2 两级缓存

| 层级 | 技术 | 容量 | 延迟 | 用途 |
|------|------|------|------|------|
| L1 | moka (sync cache) | 256MB | <1ms | 当前视野热点瓦片 |
| L2 | 文件系统 + LRU 元数据索引 | 可配置 (默认 50GB) | 1-5ms | 持久化缓存 + LRU 驱逐 |

### 2.3 前端渐进式加载

- 低级别瓦片拉伸显示 → 高级别瓦片到达后 CesiumJS 自动替换
- 相机停止时优先加载当前视野高分辨率瓦片
- 旋转时降低加载优先级

### 2.4 Runtime 隔离

Tauri 自带 tokio runtime（单线程），actix-web 需要独立的 tokio runtime（多线程 worker）。
解决方案：在独立 `std::thread` 中启动 actix-web，内部创建自己的 `tokio::runtime::Runtime`。

```
main thread (Tauri tokio)
    │
    └─ std::thread::spawn("actix-server")
        └─ tokio::runtime::Runtime::new(multi_thread)
            └─ actix-web HttpServer::new()
```

actix-web 的 `actix_rt::System` 已内置 tokio runtime，直接使用 `#[actix_web::main]` 或在独立线程中 `actix_rt::System::new().block_on(...)` 即可。

### 2.5 Y 轴坐标转换

不同瓦片源使用不同的 Y 轴编号方案：

| 源 | Y 轴方案 | URL 格式 |
|----|----------|----------|
| 高德卫星 | Slippy Map (y=0 在北) | `{z}/{x}/{y}` |
| Esri World Imagery | TMS (y=0 在南) | `{z}/{y}/{x}` |
| Mapbox Satellite | Slippy Map (y=0 在北) | `{z}/{x}/{y}` |

CesiumJS `WebMercatorTilingScheme` 使用 Slippy Map Y 编号。
对于 Esri 源，需要转换：`slippyY = (2^z - 1) - tmsY`。
此转换在 `tile_downloader.rs` 中处理，前端无感知。

## 3. 模块设计

### 3.1 Rust 端

| 模块 | 文件 | 职责 | 关键技术 |
|------|------|------|----------|
| `proxy_server.rs` | 新增 | actix-web 服务器启动/关闭，替代手写 TCP | actix-web, tokio |
| `tile_middleware.rs` | 新增 | 请求拦截 → L1→L2→下载链 | actix middleware, moka, 文件系统 |
| `l1_cache.rs` | 新增 | 内存热点瓦片缓存 | moka sync cache |
| `l2_cache.rs` | 新增 | 磁盘持久化缓存 + LRU 驱逐 | 文件系统 + atime 元数据索引 |
| `tile_downloader.rs` | 新增 | 多源并发下载池 (16并发 + 限速) | reqwest |
| `tile_sources.rs` | 新增 | 瓦片源注册、选择、故障转移 | URL 模板 + 覆盖范围 |
| `prefetch.rs` | 新增 | 预取优先级队列 | tokio channel |
| `tile_server.rs` | 重构 | 保留静态文件服务 (cesium/assets)，瓦片代理迁移到 actix | — |
| `commands.rs` | 扩展 | 新增 cache_stats/set_limit/prefetch/clear/health | Tauri commands |
| `lib.rs` | 修改 | 启动 actix-web 替代手写 TCP 服务器 | — |

### 3.2 前端

| 模块 | 文件 | 职责 |
|------|------|------|
| `TileQualityManager` | `ui/src/TileQuality.tsx` (新增) | 根据相机状态决定请求级别 |
| `CameraPrefetcher` | `ui/src/CameraPrefetcher.ts` (新增) | 预测相机轨迹，提前请求瓦片 |
| `CacheDashboard` | `ui/src/Overlay.tsx` (扩展) | 缓存监控 UI |
| `Earth.tsx` | 修改 | 集成渐进式加载 + 预取，maxLevel→18 |

## 4. 数据流

```
1. CesiumJS 请求 /china-tiles/12/3456/2345.jpg
2. actix-web TileProxyMiddleware 拦截
3. L1 (moka) 查询 → 命中 (<1ms) → 返回
4. L1 未命中 → L2 (文件系统) 查询 → 命中 (1-5ms) → 回填 L1 → 返回
5. L2 未命中 → 加入下载队列 (reqwest pool, 16并发)
   - 源选择: 高德(中国) / Esri(全球) / Mapbox(全球)
   - 下载 → 写入 L2 → 回填 L1 → 返回
6. 所有源失败 → 返回低一级别瓦片拉伸 (fallback)
```

## 5. 瓦片源配置

```rust
// tile_sources.rs
pub struct TileSource {
    pub name: &'static str,
    pub url_template: &'static str,       // {z}/{x}/{y}
    pub bbox: Option<(f64, f64, f64, f64)>, // (lat_min, lng_min, lat_max, lng_max)
    pub min_zoom: u32,
    pub max_zoom: u32,
    pub priority: u8,                      // 1=最高
    pub rate_limit_rps: f64,              // 每秒请求数限制
}

pub const TILE_SOURCES: &[TileSource] = &[
    TileSource {
        name: "高德卫星",
        url_template: "https://wprd0{0-3}.is.autonavi.com/appmaptile?style=6&x={x}&y={y}&z={z}",
        bbox: Some((15.0, 70.0, 55.0, 140.0)),
        min_zoom: 3, max_zoom: 18,
        priority: 1,
        rate_limit_rps: 50.0,
    },
    TileSource {
        name: "Esri World Imagery",
        url_template: "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
        bbox: None, // 全球
        min_zoom: 0, max_zoom: 18,
        priority: 2,
        rate_limit_rps: 100.0,
    },
    TileSource {
        name: "Mapbox Satellite",
        url_template: "https://api.mapbox.com/v4/mapbox.satellite/{z}/{x}/{y}.jpg?access_token={token}",
        bbox: None,
        min_zoom: 0, max_zoom: 18,
        priority: 3,
        rate_limit_rps: 100.0,
    },
];
```

## 6. 存储估算

- z3-z10 中国全量预下载：~575MB（已有）
- z11-z15 全球浏览缓存：~5-20GB
- z16-z18 热点区域：~10-50GB
- L1 内存缓存：256MB
- L2 默认上限：50GB（可配置，0=无上限）

## 7. 性能目标

- L1 命中延迟：<1ms
- L2 命中延迟：<5ms
- 下载 + 返回延迟：<500ms（取决于 CDN）
- 并发处理能力：1000+ req/s
- 瓦片加载无可见闪烁（渐进式替换）

## 8. 实施阶段（旧版 — 见 §12 更新版）

### Phase 1: Rust 代理服务器核心 (actix-web + L1 + L2)
- 添加 actix-web, moka, sled, reqwest 依赖
- 实现 `proxy_server.rs` + `tile_middleware.rs`
- 实现 `l1_cache.rs` + `l2_cache.rs`
- 保留现有 `tile_server.rs` 静态文件服务
- 测试：L1/L2 缓存命中、未命中下载、并发压测

### Phase 2: 多源下载器 + 故障转移
- 实现 `tile_sources.rs` + `tile_downloader.rs`
- 源选择逻辑（bbox 匹配 + 优先级）
- 限速 + 并发控制
- 测试：源故障转移、限速行为

### Phase 3: 前端渐进式加载 + 预取
- 实现 `TileQuality.tsx` + `CameraPrefetcher.ts`
- 修改 `Earth.tsx`：maxLevel→18，集成渐进式加载
- 修改 `Overlay.tsx`：缓存监控 UI
- 测试：缩放体验、预取准确性

### Phase 4: 管理命令 + 设置面板
- 实现 Tauri commands：cache_stats/set_limit/prefetch/clear/health
- 前端设置面板：缓存统计、存储上限、预取区域
- 集成测试 + 端到端验证

## 9. 风险与缓解

| 风险 | 缓解 |
|------|------|
| actix-web 编译时间增加 | 开发期用 debug build，CI 用 sccache |
| L2 缓存文件累积 | 可配置上限 + LRU 驱逐 + atime 清理 |
| CDN 限流/封禁 | 多源故障转移 + 限速 + User-Agent 伪装 |
| 磁盘空间耗尽 | 可配置上限 + LRU 驱逐 + 告警 |
| 高德 API 变更 | 多源自动切换，单源失效不影响全局 |

## 10. 依赖变更

### Cargo.toml 新增
```toml
actix-web = "4"
actix-rt = "2"
moka = { version = "0.12", features = ["sync"] }
reqwest = { version = "0.12", features = ["rustls-tls", "stream"] }
tokio = { version = "1", features = ["full"] }
dirs = "5"
```

### package.json 不变（前端无新依赖）

---

## 11. 生产级加固 (P1/P2 审计响应)

### 11.1 错误分类与恢复矩阵

| 错误类型 | 严重度 | 可恢复 | 用户可见行为 | 恢复策略 |
|----------|--------|--------|-------------|----------|
| 端口 21337 被占用 | P1 | 是 | 自动切换端口，前端轮询新端口 | `find_available_port(21337)` 扫描 100 个端口 |
| 数据目录不可写 | P1 | 是 | 回退到临时目录 + 通知 | `dirs::cache_dir()` 回退 |
| CesiumJS 初始化失败 | P1 | 是 | 回退到 react-globe.gl 纯黑球体 | 已有 fallback 机制 |
| 所有瓦片源不可达 | P1 | 是 | 显示低级别拉伸瓦片或纯色球体 | 多源故障转移 + fallback |
| WebGL 上下文丢失 | P0 | 否 | 回退到 react-globe.gl | 已有 renderError 处理 |
| 单个瓦片下载失败 | P2 | 是 | 该瓦片区域显示低级别拉伸 | 跳过该瓦片，记录日志 |
| 磁盘空间不足 | P1 | 是 | LRU 驱逐 + 通知用户 | 自动清理最旧瓦片 |
| WebView2 缺失 | P1 | 否 | 安装引导 | `webviewInstallMode: "fixRuntime"` |

### 11.2 性能预算

| 指标 | 目标 | 测量方法 |
|------|------|----------|
| 冷启动 → 地球可见 | < 5 秒 | `performance.now()` 差值 |
| 热启动 (全缓存) → 地球可见 | < 2 秒 | 同上 |
| 瓦片 L1 命中延迟 | < 1ms | actix-web middleware 计时 |
| 瓦片 L2 命中延迟 | < 5ms | 文件读取计时 |
| 瓦片下载延迟 | < 500ms | reqwest 超时设置 |
| 内存占用 (空闲) | < 300MB | OS 进程监控 |
| 内存占用 (浏览中) | < 600MB | 含 L1 256MB |
| 帧率 (旋转中) | ≥ 30fps | CesiumJS `scene.debugShowFramesPerSecond` |
| 帧率 (静止) | ≥ 60fps | 同上 |

### 11.3 诊断与可观测性

- **Rust 端**: 所有 tile 操作通过 `logger::log()` 写入 `~/.aurora/logs/`
- **前端**: `diag()` 写入 localStorage + 转发 Rust 日志（已有）
- **actix-web**: 内置 `Logger` middleware 记录每个请求的方法/路径/状态码/延迟
- **缓存统计**: `get_cache_stats` Tauri command 返回 L1/L2 命中率、总大小、文件数

### 11.4 高 DPI 适配

- CesiumJS `useBrowserRecommendedResolution: true`（已设置）— 自动适配 Retina/4K
- Tauri 窗口设置 `"useHdpi": true`（tauri.conf.json）
- react-globe.gl canvas 尺寸使用 `window.devicePixelRatio` 缩放

### 11.5 多显示器

- 全屏在 `window.current_monitor()` 上执行（Tauri 默认行为）
- 窗口记忆上次位置和大小（localStorage 持久化）

### 11.6 首次运行体验

- 启动时自动下载核心资源（纹理 + CesiumJS + z0-z2 瓦片，~30MB）
- 下载期间显示加载遮罩 + 进度
- 中国瓦片后台下载，不阻塞
- 无网络时：使用已缓存资源，缺失瓦片显示灰色

### 11.7 前端 API 规范化

- 替换 `window.__TAURI_INTERNALS__` 为 `@tauri-apps/api/core` 的 `invoke()`
- 类型安全的 Tauri command 调用
- 移除所有 `@ts-ignore` 注释

### 11.8 WebView2 部署

```json
// tauri.conf.json
{
  "bundle": {
    "windows": {
      "webviewInstallMode": "fixRuntime",
      "wix": {
        "includeWebView2Runtime": "bootstrapper"
      }
    }
  }
}
```

### 11.9 CesiumJS 体积控制

- 当前：npm tarball 解压 ~40MB
- 优化：只保留 `Build/Cesium/` 下的 `Cesium.js` + `Workers/` + `Assets/` + `Widgets/` + `ThirdParty/`
- 排除 `Documentation/`、`Source/`、`Specs/` 等开发文件
- 目标：~25MB 压缩后 ~8MB

---

## 12. 更新后的实施阶段

### Phase 0: P0 修复 (与 Phase 1 并行)
- 健康检查端点 `/health`
- `dirs` crate 替代硬编码路径 + `AURORA_DATA_DIR` 环境变量
- HTTP 缓存头 `Cache-Control: immutable`
- 版权合规确认
- 端口冲突自动扫描

### Phase 1: Rust 代理服务器核心 (actix-web + L1 + L2)
- 添加 actix-web, moka, reqwest, dirs 依赖
- 实现 `proxy_server.rs` + `tile_middleware.rs`
- 实现 `l1_cache.rs` + `l2_cache.rs`
- 保留现有 `tile_server.rs` 静态文件服务（cesium/assets 等非瓦片资源）
- 测试：L1/L2 缓存命中、未命中下载、并发压测、健康检查

### Phase 2: 多源下载器 + 故障转移
- 实现 `tile_sources.rs` + `tile_downloader.rs`
- 源选择逻辑（bbox 匹配 + 优先级）
- 限速 + 并发控制 (16 并发, 可配置)
- 测试：源故障转移、限速行为、Y 轴转换

### Phase 3: 前端渐进式加载 + 预取
- 实现 `TileQuality.tsx` + `CameraPrefetcher.ts`
- 修改 `Earth.tsx`：maxLevel→18，服务器就绪探针，集成渐进式加载
- 修改 `Overlay.tsx`：缓存监控 UI
- 替换 `__TAURI_INTERNALS__` 为官方 `@tauri-apps/api/core`
- 测试：缩放体验、预取准确性

### Phase 4: 管理命令 + 设置面板 + 生产加固
- 实现 Tauri commands：cache_stats/set_limit/prefetch/clear/health
- 前端设置面板：缓存统计、存储上限、预取区域
- 错误恢复矩阵验证
- 性能预算验证
- 集成测试 + 端到端验证
