# Aurora-App CTO 审计报告

**审计日期**：2026-06-30
**审计范围**：aurora-app 全栈（Rust 后端 src-tauri、Trit-Core、Aurora 业务层、UI 前端）
**审计人**：CTO 视角

---

## 一、总体结论

| 维度 | 状态 |
|---|---|
| Rust 编译 (`cargo check --workspace --all-features`) | ✅ 通过 |
| Clippy (`-D warnings`) | ✅ 零警告 |
| 测试套件 (`cargo test --workspace`) | ✅ 全部通过 |
| 前端类型检查 (`tsc --noEmit`) | ✅ 零错误 |
| Tauri CSP / 安全配置 | ✅ 严格、自洽 |
| IPC 数据契约（前后端字段对齐） | ✅ 完全一致 |
| 路径遍历 / 输入校验 | ✅ 到位 |

**项目可构建、可运行、测试可跑通。** 代码整体质量高——输入校验、原子写入、多源故障转移、缓存 LRU 驱逐、安全 CSP 都有扎实实现。

但发现 **2 个真实 Bug**、**3 个数据完整性/一致性问题**、**若干代码质量问题**，按严重度分级如下。

---

## 二、文件结构与模块树

```
trit-core/                          # Rust workspace
├── src/                            # trit-core 库 (v0.3.0) — 三态决策引擎
│   ├── core/                       # Layer 4: TritValue/Phase/Frame/TritWord/TernaryAlgebra
│   ├── anchor/                     # Layer 1: 5 个稳态约束（否决权）
│   ├── hook/                       # Layer 2: 场景识别 + 模块调度
│   ├── adapters/                   # Layer 3: 10 个认知模块
│   ├── meta/                       # MetaInterrupt / SafeFallback / ResolutionPolicy
│   ├── feedback/                   # Layer 5: 实践测试与校正
│   ├── security/ budget/ calibration/ clock/ sandbox/ baseline/
│   └── bin/                        # sandbox / dhat_profile / launcher / adversarial_audit
├── aurora/src/                     # aurora 库 (v0.1.0) — 认知主权工具
│   ├── app.rs                      # AuroraApp facade（CLI + Tauri 共用入口）
│   ├── pipeline/                   # analysis.rs + attention.rs 两条独立链
│   ├── bc/                         # 6 个 bounded contexts
│   ├── percept/                    # LLM 感知链（cloud/local/fft 降级）
│   ├── db/                         # SQLite 持久化 + 迁移
│   ├── config/ ingest/ wavelet/ cli.rs main.rs
├── src-tauri/src/                  # aurora-desktop (v0.1.0) — Tauri 桌面壳
│   ├── lib.rs                      # run() 主入口：日志/缓存/代理服务器/AuroraApp/Tauri
│   ├── commands.rs                 # Tauri 命令桥（run_analysis_pipeline 等）
│   ├── proxy_server.rs             # actix-web 瓦片代理 (127.0.0.1:21337)
│   ├── asset_fetcher.rs            # 多 CDN 资源预下载
│   ├── tile_downloader.rs          # 16 并发瓦片下载器
│   ├── l1_cache.rs / l2_cache.rs   # 内存 + 磁盘两级缓存
│   ├── tile_sources.rs / bucket_template.rs / data_dir.rs / logger.rs / utils.rs
├── ui/src/                         # React + TypeScript 前端
│   ├── App.tsx                     # 三栏布局 + 快捷键 + pipeline 调用
│   ├── Earth.tsx                   # CesiumJS 主引擎 + react-globe.gl 回退
│   ├── Overlay.tsx                 # 分析面板 + 设置抽屉 + 资源/缓存管理
│   ├── TopBar / EditorPanel / ConflictPanel / AsiGauge / ReminderHistory / GalleryOverlay / SplitHandle / ErrorBoundary
│   ├── types.ts                    # 与 Rust 对应的 TS 类型
│   └── utils/                      # diag.ts / tauri.ts
└── scenarios/ tests/ fuzz/ benches/
```

---

## 三、问题清单（按严重度）

### 🔴 P0 — 真实 Bug

#### Bug-1: `app.rs` 冲突卡片帧名硬编码，与 commands 路径不一致

**位置**：`aurora/src/app.rs:200-208`

```rust
for interrupt in &analysis_report.decision.interrupts {
    view.add_conflict(ConflictCard {
        conflict_type: format!("{:?}", interrupt.conflict),
        reason: interrupt.reason.clone(),
        frame_a: "Embodied".into(),   // ← 硬编码
        frame_b: "Individual".into(), // ← 硬编码
        acknowledged: false,
    });
}
```

**问题**：`render_output` 把所有 `MetaInterrupt` 的 `frame_a`/`frame_b` 硬编码为 `"Embodied"` / `"Individual"`，但实际冲突可能发生在任意两帧之间（Science vs Consensus 等）。

而 Tauri 命令路径 `commands.rs:81-90` 的 `extract_frames_from_reason` 是从 `reason` 字符串解析真实帧名的。**两条路径产生不一致的冲突卡片**：
- CLI / HTML 报告（走 `render_output`）→ 帧名永远是 Embodied/Individual（错误）
- Tauri 前端（走 commands.rs）→ 帧名是真实解析值（正确）

**影响**：HTML/JSON 报告中的冲突卡片帧名失真，审计可追溯性受损。违反 CLAUDE.md「跨帧冲突不该被抹平，而应被可审计地记录」核心约束。

**修复**：在 `render_output` 中复用 `extract_frames_from_reason` 逻辑（应抽到共享位置避免 DRY），从 `interrupt.reason` 解析真实帧名。建议把 `extract_frames_from_reason` 从 `commands.rs` 移到 `bc/ternary_decision.rs` 或 `MetaInterrupt` 自身方法，两处共用。

---

#### Bug-2: `capabilities/default.json` 远程 URL 端口错误（31337 vs 21337）

**位置**：`src-tauri/capabilities/default.json:20-22`

```json
"remote": {
  "urls": ["http://localhost:31337/**"]   // ← 31337
}
```

**问题**：代理服务器实际监听 **21337**（`proxy_server.rs:67` bind、`lib.rs:111`、`commands.rs:110`、`Earth.tsx:15`、`tauri.conf.json` CSP 全部是 21337）。`remote.urls` 白名单写的是 **31337**，端口号孤立错误。

**影响**：Tauri v2 的 `remote` 配置控制从远程 URL 加载的页面能否访问 IPC 桥。当前前端主页面运行在 `tauri.localhost`（Cesium 脚本从 21337 子加载），所以**主流程暂不受影响**。但任何未来从 `http://localhost:21337` 加载的远程页面（如 frame-src 场景、bucket.html 模板）将因白名单不匹配而被拒绝 IPC 调用，且该错误极难排查。

**修复**：`31337` → `21337`。一字之差。

---

### 🟡 P1 — 数据完整性 / 一致性

#### Issue-3: `attention.rs` audit 快照决策结果硬编码为 "pending"

**位置**：`aurora/src/pipeline/attention.rs:55-61`

```rust
AuditDecisionSnapshot {
    signal_count: signals.len(),
    signal_frames: ...,
    result_value: "pending".into(),  // ← 永远是 pending
    result_frame: "Meta".into(),     // ← 永远是 Meta
    contact_participation,
}
```

**问题**：`build_snapshot` 在 `run_attention` 中被调用，此时 analysis 已经产出了真实 `DecisionRecord.result`，但快照没有接收/记录它，硬编码为 `"pending"` / `"Meta"`。写入 SQLite `audit_log` 表的决策结果永远是占位值。

**影响**：审计日志的 `result_value` / `result_frame` 字段无业务价值，无法用于事后追溯"当时决策是什么"。与 Aurora 的可审计性定位冲突。

**修复**：给 `build_snapshot` 传入 `&DecisionRecord`（或至少 `result.value()` + `result.frame()`），填充真实值。`run_attention` 签名需增加 `decision: &DecisionRecord` 参数，`app.rs` 调用处传入 `&analysis_report.decision`。

---

#### Issue-4: `Earth.tsx` Sandcastle 代码执行依赖不存在的 `Cesium._viewers`

**位置**：`ui/src/Earth.tsx:590-600`

```typescript
const fn = new Function('Cesium', 'cesiumContainer', code);
fn(Cesium, 'cesiumContainer');

const viewers = (Cesium as any)._viewers;        // ← CesiumJS 不维护此数组
if (viewers && viewers.length > 0) {
  viewerRef.current = viewers[viewers.length - 1];
}
```

**问题**：CesiumJS API **不**维护全局 `_viewers` 数组。`new Function` 执行用户代码后，`Cesium._viewers` 永远是 `undefined`，`viewerRef.current` 不会被赋值。后续 `setEngine('cesium')` + `setReady(true)` 假装成功，但实际没有 viewer 引用——自动旋转 useEffect（依赖 `viewerRef.current`）、resize observer、cleanup 都拿不到 viewer。

**影响**：Sandcastle（编辑器运行 Cesium 代码）功能静默失效——用户代码会执行（Viewer 会创建并显示），但 Earth 组件无法管理该 viewer 的生命周期，旋转/重置/清理都不工作。回退到 globe-gl 时还可能泄漏一个未被 destroy 的 Viewer。

**修复**：让用户代码通过约定的全局变量返回 viewer，或在 `cesiumContainer` 上 MutationObserver 监听 canvas 挂载，或要求示例代码把 viewer 赋给 `window.AURORA_VIEWER`（代码里已有 `(window as any).AURORA_VIEWER = viewerRef.current` 的写入，但读的是 `_viewers`，自相矛盾）。最简方案：约定示例代码 `window.viewer = new Cesium.Viewer(...)`，Earth 从 `window.viewer` 读取。

---

#### Issue-5: `asset_fetcher.rs` 中国瓦片缩放范围注释与常量不一致

**位置**：`src-tauri/src/asset_fetcher.rs`

- 第 9 行注释：`中国区域瓦片（高德卫星 z3-z8）→ ~/.aurora/china-tiles/`
- 第 106-108 行常量：`CHINA_ZOOM_MIN = 3`, `CHINA_ZOOM_MAX = 10`（即 z3-z10）
- 第 281 行注释：`// 中国区域瓦片 (高德卫星 z3-z8)`
- 第 933 行 doc comment：`下载中国区域卫星影像瓦片（高德卫星 z3-z10）`

**问题**：注释在 z3-z8 和 z3-z10 之间反复横跳。常量是 z3-z10（实际行为），但多处注释写 z3-z8。

**影响**：纯文档噪音，不影响运行，但会误导维护者低估下载量（z8 vs z10 瓦片数差几个数量级，z10 中国范围约 575MB）。

**修复**：统一所有注释为 z3-z10。

---

### 🟢 P2 — 代码质量 / Ponytail 违规

#### Q-6: `percept/chain.rs` 冗余的 `map().unwrap_or()` 写法

**位置**：`aurora/src/percept/chain.rs:66-68`

```rust
Err(last_error
    .map(|_| PerceptError::AllUnavailable)
    .unwrap_or(PerceptError::AllUnavailable))
```

**问题**：无论 `last_error` 是 `Some` 还是 `None`，都返回 `AllUnavailable`。`map().unwrap_or()` 链完全多余，且丢失了 `last_error` 的具体信息。

**修复**：
```rust
Err(PerceptError::AllUnavailable)
```
若想保留最后错误上下文，应让 `AllUnavailable` 携带 `source` 字段，而非当前写法。

---

#### Q-7: `utils/tauri.ts` 的 `invokeTauri` 封装从未被使用（死代码）

**位置**：`ui/src/utils/tauri.ts`

**问题**：定义了类型安全的 `invokeTauri<T>` 封装，但全局搜索发现 App.tsx / Overlay.tsx / Earth.tsx 全部直接 `await import('@tauri-apps/api/core')` 动态调用 `invoke`，没人使用这个封装。

**修复**：要么删除 `tauri.ts`（YAGNI），要么统一所有调用点改用 `invokeTauri` 以获得类型安全与统一日志。当前是"为后来准备的脚手架"，违反 ponytail「不要未请求的抽象」。

---

#### Q-8: `tile_downloader.rs` batch 返回元组的 `is_skip` 字段语义被滥用

**位置**：`src-tauri/src/tile_downloader.rs:160-163`

```rust
match self_clone.download_single_tile(z, x, y).await {
    Some(data) => { ... (z, x, y, ok, false) }      // ok=写入结果, is_skip=false
    None => (z, x, y, false, true),                  // 失败: ok=false, is_skip=true ← 语义错误
}
```

**问题**：失败分支返回 `is_skip=true`，但 `is_skip` 字段名暗示"已跳过（已存在）"。收集端 `Ok((_z,_x,_y,false,_is_skip))` 走 failed 分支计数，**结果正确**，但字段语义误导维护者。

**修复**：将元组第 5 字段重命名为 `is_terminal` 或改用 `enum BatchOutcome { Downloaded, Skipped, Failed }`，消除歧义。

---

#### Q-9: `lib.rs` 注释编号错乱

**位置**：`src-tauri/src/lib.rs:154, 157`

第 154 行注释 `// ── 3.5 生成 CesiumJS bucket.html 模板 ──`，紧接第 157 行 `// ── 4. 后台下载核心资源 ──`，但前面第 138 行已经是 `// ── 4. 启动 actix-web 代理服务器`。两个 "4"。纯文档瑕疵。

---

## 四、按模块审计小结

### Rust 后端（src-tauri）— 质量高
- **proxy_server.rs**：路径遍历防护（canonicalize + starts_with + 前缀白名单）、CORS 头、优雅关闭轮询。✅
- **commands.rs**：每个数值输入都做了 `is_finite()` + 范围校验，`prefetch_tiles` 有 50000 瓦片上限防资源耗尽。✅
- **asset_fetcher.rs**：多 CDN fallback、`.tmp` → rename 原子写入、tarball 解压只取 `Build/Cesium/` 前缀。✅
- **tile_downloader.rs**：tokio Semaphore 并发控制、连接池复用、故障转移。✅
- **l2_cache.rs**：LRU 驱逐（atime 排序删最旧 25% 到 75% 目标）、evict_lock 防并发驱逐、批量检查间隔避免每次 put 扫目录。✅
- **lib.rs**：日志初始化失败则 panic、AuroraApp 失败则优雅关闭代理服务器后 exit(1)。✅

### Trit-Core — 质量极高
- **algebra.rs**：4×4 真值表穷举测试、热/冷路径分离、`t_and_n` 等权平均避免左折叠偏差。✅
- 跨帧操作永不强制二元决策，统一产 Hold + MetaInterrupt。✅
- `Phase::new` 返回 Result，`new_clamped` 仅在显式静默归一化时用。✅

### Aurora 业务层 — 质量高，2 处瑕疵
- **app.rs**：facade 设计清晰，CLI/Tauri 共用入口，percept 链降级（cloud→local→fft）。✅
- **analysis.rs**：`PhaseTrajectory` 趋势追踪、logistic 映射。✅
- **attention.rs**：见 Issue-3（audit 快照硬编码 pending）。
- **app.rs**：见 Bug-1（冲突卡片帧名硬编码）。

### UI 前端 — 质量高，1 处真实 Bug
- **App.tsx**：前端镜像了后端的输入校验、ErrorBoundary、mock 降级、快捷键。✅
- **Overlay.tsx**：china-tile 轮询有 5 分钟安全超时 + 关闭面板时 cleanup。✅
- **Earth.tsx**：CesiumJS→globe.gl 多级回退、WebGL 致命错误检测、Cosmos 预设资源 dispose 干净。✅ 但见 Issue-4（Sandcastle `_viewers`）。
- **types.ts**：与 Rust `PipelineResponse`/`CacheStats` 字段逐一对应。✅

---

## 五、优先修复建议

| 优先级 | 项 | 工作量 |
|---|---|---|
| P0 | Bug-1：`render_output` 复用帧名解析 | 30 min（含抽公共函数） |
| P0 | Bug-2：`capabilities` 端口 31337→21337 | 1 min |
| P1 | Issue-3：audit 快照记录真实决策结果 | 20 min |
| P1 | Issue-4：Sandcastle viewer 获取机制重做 | 1-2 h |
| P1 | Issue-5：统一中国瓦片注释为 z3-z10 | 5 min |
| P2 | Q-6~Q-9：代码质量清理 | 30 min |

**结论**：项目地基扎实，可发布。P0 两个 Bug 建议在下次发布前修复——Bug-2 是一字之差，Bug-1 关乎可审计性这一产品核心承诺。
