# 决策结果抽屉（Decision Drawer）设计

> **日期**：2026-07-01
> **阶段**：M1 UI 打磨
> **状态**：已确认，待出实施计划

## 1. 背景与动机

后端 `PipelineResponse`（`src-tauri` → 前端 `ui/src/types.ts`）已返回完整的分析结果：

```ts
interface PipelineResponse {
  detected_freq_hz: number;
  decision: string;        // "True" | "Hold" | "False"
  phase: number;           // [0.0, 1.0]
  final_frame: string;     // 跨帧 Hold 时为 "Meta"
  signals: SignalWord[];   // 逐输入信号：frame + value + phase
  asi: number;             // 注意力主权指数 0–1
  reminder_count: number;
  active_shift_count: number;
  conflicts: ConflictResponse[];  // 跨帧冲突列表
  reminders: ReminderResponse[];
  html: string;
  json: string;
}
```

但前端（`App.tsx` / `TopBar.tsx`）只把 `decision` 一个字段以文字标签显示在顶栏。`phase` / `asi` / `signals` / `conflicts` 全部丢弃未渲染。

这是 M1 P0 的明确缺口：

- **实时注意力图谱（Frame 权重）** —— `signals` 数据就绪，无渲染。
- **冲突面板** —— `conflicts` 数据就绪，无渲染。
- **ASI 仪表** —— `asi` 数据就绪，无渲染。

本设计用最小改动把这些已就绪数据呈现出来，不引入新数据源、不改后端。

## 2. 范围

**做**：
- 新增 `DecisionDrawer.tsx` 组件，只读展示 decision/phase + ASI 仪表 + Frame 张力 + 冲突列表。
- 顶栏 `decision` 标签改为可点击按钮，点开抽屉。
- 复用既有抽屉容器样式（`aur-settings-drawer` 系列），不新增浮层类型。
- 新增组件测试。

**不做（YAGNI）**：
- **Hold 可覆盖交互**：后端无对应 Tauri 命令，需改 Rust 侧重仲裁，是另一个工作量。先只读展示，覆盖交互按真需求再加。
- **reminders 区**：M0 HTML 报告已渲染 reminders，抽屉聚焦决策本体。后续按需加。
- **不新增 CSS 体系**：尽量复用 `aur-settings-*` 与 Sidebar 的 `aur-summary-*` 类。

## 3. 设计

### 3.1 触发与状态

- `App.tsx` 新增 `decisionDrawerOpen` state（`useState(false)`）。
- 顶栏 `decision` 标签（`TopBar.tsx` 现有 `<span className="aur-topbar-decision">`）改为 `<button>`：
  - `data == null` 时 `disabled`（灰显，不可点）。
  - `loading` 时 disabled（运行中）。
  - 点击 → `onOpenDecisionDrawer()` → `setDecisionDrawerOpen(true)`。
- 抽屉 `open` 时渲染，`onClose` → `setDecisionDrawerOpen(false)`。
- 与设置抽屉（Overlay）互不干扰：两者可各自独立开关，不强制互斥（ponytail：不引入互斥逻辑，除非真有重叠问题）。

### 3.2 组件：`DecisionDrawer.tsx`

```
ui/src/DecisionDrawer.tsx
```

**Props**：

```ts
interface Props {
  open: boolean;
  onClose: () => void;
  data: PipelineResponse | null;
  loading: boolean;
}
```

**`open == false` 或 `data == null`**：return null（无可展示数据时不渲染）。

**布局**（自上而下）：

1. **决策头**
   - 大字 `decision`（True/Hold/False），用现有三色变量 `--aur-true` / `--aur-hold` / `--aur-false`。
   - `Phase {phase.toFixed(2)}`（0.5 = 中性）。
   - `Frame: {final_frame}`。

2. **ASI 仪表条**
   - 横向条：宽度 = `asi * 100%`。
   - 右侧数值 `{(asi * 100).toFixed(0)}%`。
   - 复用 M0 HTML 渲染的 ASI 仪表语义（注意力主权指数）。

3. **Frame 张力**（`signals[]`）
   - 每项一行：
     - frame 名（`signal.frame`）。
     - value 三态色点：True=绿 / Hold=琥珀 / False=红 / Unknown=灰。
     - phase 条：`<0.5` 偏 False、`>0.5` 偏 True、`=0.5` 中性。条长按 `|phase - 0.5| * 2` 映射，方向左右区分。
   - 空数组（理论不会发生，防御）→ "无输入信号"。

4. **冲突列表**（`conflicts[]`）
   - 每项：
     - `{frame_a} ↔ {frame_b}`。
     - `conflict_type` 小标签。
     - `reason` 文本。
   - 空数组 → "无跨帧冲突"（绿色 ✓，表示决策未触发跨帧 Hold）。

### 3.3 数据流

```
后端 run_analysis_pipeline (Tauri)
  → App.handleRun → setData(result)
  → TopBar 接 decision = data?.decision
  → 用户点 decision 标签 → setDecisionDrawerOpen(true)
  → DecisionDrawer 接 data → 渲染 phase/asi/signals/conflicts
```

无新 Tauri 调用、无新数据拉取——抽屉只消费 App 已持有的 `data`。

### 3.4 样式

- 抽屉容器复用 `aur-settings-drawer` / `aur-settings-drawer__header` / `aur-settings-drawer__body`。
- 内部分区复用 `aur-settings__section` / `aur-settings__title`。
- ASI 仪表条、phase 条、冲突项：新增少量 CSS 类（`aur-asi-bar`、`aur-tension-row`、`aur-conflict-item` 等），加到 `aurora.css`。颜色一律用现有 CSS 变量，不引入新色值。

### 3.5 测试：`src/test/DecisionDrawer.test.tsx`

沿用 `src/test/` 现有模式（vitest + @testing-library/react）：

- 有数据时：四区块（决策头 / ASI / Frame 张力 / 冲突）均渲染，decision 文案与 phase/asi 数值出现。
- `conflicts == []`：显示"无跨帧冲突"。
- `signals` 多项：每项 frame 名都出现。
- `data == null` 或 `open == false`：组件不渲染（DOM 中无决策头）。

## 4. 验收

- `npm run build` 通过（TypeScript 干净）。
- `npm test` 全绿（含新增 DecisionDrawer 测试）。
- 浏览器/Tauri 中：运行分析后，点顶栏 decision 标签，抽屉展开显示完整结果；无数据时标签灰显不可点。

## 5. 不做的事（明确排除）

- Hold 覆盖交互（后端无命令）。
- reminders 渲染（M0 HTML 已覆盖）。
- 新增 CSS 设计体系。
- 后端任何改动。
