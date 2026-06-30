# Aurora 启动欢迎页 + 真实进度条 设计

> 日期：2026-06-30
> 状态：已确认，待实现计划
> 范围：前端 UI（`ui/src/`），不改 Rust 命令

## 1. 问题

程序启动时没有欢迎页面，也没有读条（进度条）。根因：

1. **首次启动地球静默退化**：`Earth.tsx` 启动流在 `serverReady` 后查 `get_asset_status`，若 CesiumJS 未就绪就 `setEngine('globe-gl')` 回退——但此时本地纹理也还没下完，globe-gl 渲染破损，用户看到空画面而非"加载中"。
2. **没有进度条**：现有 `aur-globe-loader` 只有静态文字"正在初始化地球"，不轮询下载进度。
3. **没有欢迎页**：当前直接进 HUD，无品牌/介绍页。

## 2. 目标

启动时显示：
- 欢迎页（Aurora 品牌 + 产品定位）
- 真实进度条，反映阻塞资源（纹理 + CesiumJS）下载进度

已缓存时秒过（<500ms），不强制停顿。

## 3. 数据源（已存在，不改后端）

- `get_asset_status()` → `AssetReport { assets: [{status: cached|missing|ok|failed}], all_ready }`
- Rust 启动时已 spawn `asset-downloader` 线程跑 `ensure_all_resources()`，自动下载纹理 + CesiumJS + 中国瓦片。
- 阻塞进度 = `assets` 中 `status ∈ {cached, ok}` 的比例 × 100%。
- `all_ready === true` = 所有阻塞资源就绪，可进主界面。
- 中国瓦片（~575MB）是后台增量下载，**不在 critical path**——欢迎页进度条不追踪它。

## 4. 组件设计

### 4.1 新增 `ui/src/Welcome.tsx`（单一职责：启动门面 + 进度）

阶段机：`probing`（探服务器）→ `downloading`（轮询资源）→ `ready`（淡出）。

- 每 300ms 轮询 `get_asset_status`，计算 `ready/total` 百分比。
- UI：
  - Aurora 标题
  - 产品定位一句："长见识输入源 · 注意力主权"
  - 百分比文字（如 `42%`）
  - CSS 进度条（width 跟百分比）
  - 当前阶段提示文字（"正在连接服务器…" / "下载地球资源…" / "准备就绪"）
- 触发 `onReady` 的条件（任一）：
  - `all_ready === true`
  - 超时 15s（与现有 `CESIUM_INIT_TIMEOUT_MS` 对齐）
- 淡出动画后从 DOM 移除。

### 4.2 改 `ui/src/App.tsx`

- 新增 `started` state。
  - `started === false`：只渲染 `<Welcome onReady={() => setStarted(true)} />`。
  - `started === true`：渲染现有 HUD（TopBar + Earth + Overlay）。
- 首次 `handleRun()` 调用从 Welcome `onReady` 后触发（而非当前 `useEffect` 里的自动首调），避免欢迎页还在时就跑管线。
- 删除 `App.tsx` 中 `useEffect` 里 `initialRunDone` 自动 `handleRun()` 的逻辑，移到 `started` 变 true 后。

### 4.3 改 `ui/src/Earth.tsx`

- **删除静默回退**：CesiumJS 未就绪时不再立即 `setEngine('globe-gl')`，保持 `loading` 直到 `all_ready`。消除"破损地球"症状。
- 超时仍回退 globe-gl（保底，不卡死）。
- `aur-globe-loader` 静态遮罩保留作地球自身渲染间隙占位，启动阶段由 Welcome 接管。

### 4.4 CSS（`ui/src/aurora.css`）

- 新增 `.aur-welcome` 容器：全屏、深色背景、居中 flex。
- 新增 `.aur-welcome__progress` 进度条容器 + `.aur-welcome__bar`（width 过渡动画）。
- 复用现有设计 token（`--aur-night`、`--font-mono` 等）。

## 5. 边界与失败处理

- **非 Tauri 环境**（浏览器/dev）：Welcome 跳过下载阶段，立即 `onReady`（mock 数据，无需资源）。
- **下载超时/失败**：15s 后强制 `onReady`，进 HUD，地球走 globe-gl 回退。
- **断网**：`get_asset_status` 返回 `all_ready=false`，超时后进主界面，地球显示回退纹理。
- **已缓存**：`all_ready` 首次轮询即为 true，Welcome 秒过（<500ms）。

## 6. 测试

- `Welcome.test.tsx`：
  - mock `invoke` 返回未就绪 → 百分比 0%，不触发 `onReady`。
  - mock `invoke` 返回就绪 → 触发 `onReady`。
  - 超时 → 触发 `onReady`。
  - 非 Tauri 环境 → 立即 `onReady`。
- `App.test.tsx` 集成：
  - `started=false` 渲染 Welcome，不渲染 TopBar/Earth。
  - `onReady` 后渲染 HUD。

## 7. 不做（YAGNI）

- 不做 china-tiles 完整下载进度（575MB 后台，不阻塞进入主界面）。
- 不做欢迎页跳过按钮（15s 超时已是逃生阀）。
- 不改 Rust 侧命令（`get_asset_status` 已够用）。
- 不做强制停顿（已缓存即秒过）。

## 8. 受影响文件

| 文件 | 改动 |
|------|------|
| `ui/src/Welcome.tsx` | 新增 |
| `ui/src/App.tsx` | 加 `started` 门面、移首调到 onReady 后 |
| `ui/src/Earth.tsx` | 删静默回退、保超时回退 |
| `ui/src/aurora.css` | 加 welcome + progress 样式 |
| `ui/src/test/Welcome.test.tsx` | 新增 |
| `ui/src/test/App.test.tsx` | 改：started 门面断言 |
