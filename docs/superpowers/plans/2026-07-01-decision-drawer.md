# 决策结果抽屉（Decision Drawer）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新增 `DecisionDrawer.tsx`，把后端已返回但前端未展示的 phase/asi/signals/conflicts 通过点击顶栏 decision 标签展开的抽屉只读呈现。

**Architecture:** App 持有 `decisionDrawerOpen` state。TopBar 的 `decision` 文字标签改为可点击 `<button>`，点击打开抽屉。抽屉复用既有 `aur-settings-drawer` 容器与 `aur-gauge` 仪表 CSS，只读消费 App 已持有的 `PipelineResponse`。不改后端、不加新 Tauri 命令。

**Tech Stack:** React 18 + TypeScript + Vite + vitest + @testing-library/react。CSS 变量已在 `ui/src/aurora.css` 定义（`--aur-true/hold/false/void`、`aur-gauge__*`、`aur-settings-drawer`）。

## Global Constraints

- 工作目录 `E:\trit-core`，前端在 `ui/`。
- 测试命令：`cd ui && npm test`（vitest run）。构建命令：`cd ui && npm run build`（`tsc && vite build`，TypeScript 必须干净）。
- 颜色一律用 `ui/src/aurora.css` 既有 CSS 变量：`--aur-true`(#5EEAD4) / `--aur-hold`(#F0E68C) / `--aur-false`(#E57373) / `--aur-void` / `--aur-void-dim`。不引入新色值。
- 不改 Rust 后端、不加 Tauri 命令。
- 只读展示；不做 Hold 覆盖交互。
- 遵循既有文件模式：组件默认导出、`src/test/*.test.tsx`、`src/types.ts` 已有全部所需类型（`PipelineResponse`/`SignalWord`/`ConflictResponse`）。
- `ponytail:` 注释标注刻意简化。
- 提交信息末尾加 `Co-Authored-By: Claude <noreply@anthropic.com>`。

---

### Task 1: DecisionDrawer 组件骨架（决策头 + 空态）

**Files:**
- Create: `ui/src/DecisionDrawer.tsx`
- Create: `ui/src/test/DecisionDrawer.test.tsx`

**Interfaces:**
- Consumes: `ui/src/types.ts` 的 `PipelineResponse`（已存在，字段见 spec §1）。
- Produces: `export default function DecisionDrawer(props: Props): JSX.Element | null`，其中
  ```ts
  interface Props {
    open: boolean;
    onClose: () => void;
    data: PipelineResponse | null;
    loading: boolean;
  }
  ```
  `open == false || data == null` 时返回 `null`。

- [ ] **Step 1: 写失败测试**

创建 `ui/src/test/DecisionDrawer.test.tsx`：

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import DecisionDrawer from '../DecisionDrawer';
import type { PipelineResponse } from '../types';

const baseData: PipelineResponse = {
  detected_freq_hz: 2.0,
  decision: 'Hold',
  phase: 0.5,
  final_frame: 'Meta',
  signals: [
    { frame: 'Embodied', value: 'True', phase: 0.8 },
    { frame: 'Individual', value: 'False', phase: 0.2 },
  ],
  asi: 0.62,
  reminder_count: 0,
  active_shift_count: 0,
  conflicts: [],
  reminders: [],
  html: '',
  json: '',
};

describe('DecisionDrawer', () => {
  it('renders nothing when data is null', () => {
    const { container } = render(
      <DecisionDrawer open={true} onClose={vi.fn()} data={null} loading={false} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders nothing when closed', () => {
    const { container } = render(
      <DecisionDrawer open={false} onClose={vi.fn()} data={baseData} loading={false} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders decision head with decision, phase, frame', () => {
    render(<DecisionDrawer open={true} onClose={vi.fn()} data={baseData} loading={false} />);
    expect(screen.getByText('Hold')).toBeInTheDocument();
    expect(screen.getByText(/Phase 0\.50/)).toBeInTheDocument();
    expect(screen.getByText(/Meta/)).toBeInTheDocument();
  });

  it('calls onClose when close button clicked', () => {
    const onClose = vi.fn();
    render(<DecisionDrawer open={true} onClose={onClose} data={baseData} loading={false} />);
    fireEvent.click(screen.getByTitle('关闭'));
    expect(onClose).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd ui && npm test -- DecisionDrawer`
Expected: FAIL — 模块 `../DecisionDrawer` 不存在。

- [ ] **Step 3: 写最小实现**

创建 `ui/src/DecisionDrawer.tsx`：

```tsx
// ui/src/DecisionDrawer.tsx — 决策结果抽屉（只读）
//
// 把后端 PipelineResponse 已返回但前端未展示的 phase/asi/signals/conflicts
// 通过抽屉呈现。点击顶栏 decision 标签打开。只读，不做 Hold 覆盖交互。

import type { PipelineResponse } from './types';

interface Props {
  open: boolean;
  onClose: () => void;
  data: PipelineResponse | null;
  loading: boolean;
}

export default function DecisionDrawer({ open, onClose, data }: Props) {
  if (!open || data == null) return null;

  const { decision, phase, final_frame } = data;

  return (
    <div className="aur-settings-drawer">
      <div className="aur-settings-drawer__header">
        <span className="aur-panel-title">决策结果</span>
        <button className="aur-btn aur-btn--icon" onClick={onClose} title="关闭">✕</button>
      </div>

      <div className="aur-settings-drawer__body">
        {/* 决策头 */}
        <div className="aur-settings__section">
          <div className="aur-decision-head">
            <span className="aur-decision-head__value" data-decision={decision}>
              {decision}
            </span>
            <span className="aur-decision-head__phase">Phase {phase.toFixed(2)}</span>
            <span className="aur-decision-head__frame">Frame: {final_frame}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd ui && npm test -- DecisionDrawer`
Expected: PASS（4 个测试）。

- [ ] **Step 5: 提交**

```bash
cd E:/trit-core
git add ui/src/DecisionDrawer.tsx ui/src/test/DecisionDrawer.test.tsx
git commit -m "feat(ui): DecisionDrawer 决策头骨架 + 空态

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: ASI 仪表 + Frame 张力条

**Files:**
- Modify: `ui/src/DecisionDrawer.tsx`
- Modify: `ui/src/test/DecisionDrawer.test.tsx`

**Interfaces:**
- Consumes: `PipelineResponse.asi`（number 0–1）、`PipelineResponse.signals: SignalWord[]`，其中 `SignalWord = { frame: string; value: string; phase: number }`。
- Produces: 无新导出（内部 JSX 区块）。

- [ ] **Step 1: 追加失败测试**

在 `DecisionDrawer.test.tsx` 的 `describe` 块内追加：

```tsx
  it('renders ASI gauge with percentage', () => {
    render(<DecisionDrawer open={true} onClose={vi.fn()} data={baseData} loading={false} />);
    // asi=0.62 → 62%
    expect(screen.getByText(/62%/)).toBeInTheDocument();
  });

  it('renders a row per signal with frame name', () => {
    render(<DecisionDrawer open={true} onClose={vi.fn()} data={baseData} loading={false} />);
    expect(screen.getByText('Embodied')).toBeInTheDocument();
    expect(screen.getByText('Individual')).toBeInTheDocument();
  });

  it('shows placeholder when signals empty', () => {
    const empty = { ...baseData, signals: [] };
    render(<DecisionDrawer open={true} onClose={vi.fn()} data={empty} loading={false} />);
    expect(screen.getByText('无输入信号')).toBeInTheDocument();
  });
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd ui && npm test -- DecisionDrawer`
Expected: FAIL — 找不到 `62%`、`Embodied` 等。

- [ ] **Step 3: 实现 ASI 仪表 + Frame 张力**

在 `DecisionDrawer.tsx` 中，把决策头 section 之后、`</div>`（body 闭合）之前，插入两个 section。完整的新 body 内容（替换 Task 1 的 body 内的决策头 section 之后的部分）：

在决策头 `</div>`（`aur-settings__section` 闭合）之后追加：

```tsx
        {/* ASI 仪表 */}
        <div className="aur-settings__section">
          <div className="aur-settings__title">注意力主权指数</div>
          <div className="aur-gauge">
            <div className="aur-gauge__track">
              <div
                className="aur-gauge__fill"
                style={{ width: `${Math.round(data.asi * 100)}%` }}
              />
            </div>
            <span className="aur-gauge__value">
              {Math.round(data.asi * 100)}%
            </span>
          </div>
        </div>

        {/* Frame 张力 */}
        <div className="aur-settings__section">
          <div className="aur-settings__title">Frame 张力</div>
          {data.signals.length === 0 ? (
            <div className="aur-summary-empty">无输入信号</div>
          ) : (
            data.signals.map((s, i) => (
              <div key={i} className="aur-tension-row">
                <span
                  className="aur-tension-row__dot"
                  data-value={s.value}
                  aria-label={s.value}
                />
                <span className="aur-tension-row__frame">{s.frame}</span>
                <span className="aur-tension-row__phase">
                  {s.phase.toFixed(2)}
                </span>
              </div>
            ))
          )}
        </div>
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd ui && npm test -- DecisionDrawer`
Expected: PASS（7 个测试）。

- [ ] **Step 5: 提交**

```bash
cd E:/trit-core
git add ui/src/DecisionDrawer.tsx ui/src/test/DecisionDrawer.test.tsx
git commit -m "feat(ui): DecisionDrawer 加 ASI 仪表 + Frame 张力条

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: 冲突列表

**Files:**
- Modify: `ui/src/DecisionDrawer.tsx`
- Modify: `ui/src/test/DecisionDrawer.test.tsx`

**Interfaces:**
- Consumes: `PipelineResponse.conflicts: ConflictResponse[]`，`ConflictResponse = { conflict_type: string; reason: string; frame_a: string; frame_b: string }`。

**借鉴 worldmonitor `CrossSourceSignalsPanel.renderSignal`**：列表项用左侧 4px 色条 + type badge + 内容的紧凑横向布局，比纯文本三行堆叠更可扫读。worldmonitor 是 HTML 字符串 + 内联样式，Aurora 用 React + CSS 类，只借布局模式不搬代码。

- [ ] **Step 1: 追加失败测试**

在 `DecisionDrawer.test.tsx` 顶部追加一个带冲突的 fixture，并在 describe 内追加测试：

```tsx
const withConflicts: PipelineResponse = {
  ...baseData,
  conflicts: [
    {
      conflict_type: 'cross_frame',
      reason: 'Embodied 高频与 Individual 自评正常冲突',
      frame_a: 'Embodied',
      frame_b: 'Individual',
    },
  ],
};
```

```tsx
  it('renders conflicts with frame pair and reason', () => {
    render(<DecisionDrawer open={true} onClose={vi.fn()} data={withConflicts} loading={false} />);
    expect(screen.getByText('Embodied ↔ Individual')).toBeInTheDocument();
    expect(screen.getByText('Embodied 高频与 Individual 自评正常冲突')).toBeInTheDocument();
  });

  it('shows no-conflict hint when conflicts empty', () => {
    render(<DecisionDrawer open={true} onClose={vi.fn()} data={baseData} loading={false} />);
    expect(screen.getByText('无跨帧冲突')).toBeInTheDocument();
  });
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd ui && npm test -- DecisionDrawer`
Expected: FAIL — 找不到 `Embodied ↔ Individual`、`无跨帧冲突`。

- [ ] **Step 3: 实现冲突列表**

在 `DecisionDrawer.tsx` 的 Frame 张力 section 之后追加。每项采用左色条 + type badge + 内容的横向布局（借鉴 worldmonitor `renderSignal`）：

```tsx
        {/* 冲突列表 */}
        <div className="aur-settings__section">
          <div className="aur-settings__title">跨帧冲突</div>
          {data.conflicts.length === 0 ? (
            <div className="aur-summary-empty">
              <span style={{ color: 'var(--aur-true)' }}>✓</span> 无跨帧冲突
            </div>
          ) : (
            data.conflicts.map((c, i) => (
              <div key={i} className="aur-conflict-item">
                <div className="aur-conflict-item__bar" />
                <div className="aur-conflict-item__body">
                  <div className="aur-conflict-item__head">
                    <span className="aur-conflict-item__pair">
                      {c.frame_a} ↔ {c.frame_b}
                    </span>
                    <span className="aur-conflict-item__badge">
                      {c.conflict_type}
                    </span>
                  </div>
                  <div className="aur-conflict-item__reason">{c.reason}</div>
                </div>
              </div>
            ))
          )}
        </div>
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd ui && npm test -- DecisionDrawer`
Expected: PASS（9 个测试）。

- [ ] **Step 5: 提交**

```bash
cd E:/trit-core
git add ui/src/DecisionDrawer.tsx ui/src/test/DecisionDrawer.test.tsx
git commit -m "feat(ui): DecisionDrawer 加冲突列表

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: CSS — 决策头 / Frame 张力 / 冲突项样式

**Files:**
- Modify: `ui/src/aurora.css`

**Interfaces:** 无（纯样式，复用既有变量与 `aur-gauge__*`、`aur-summary-empty` 类）。

- [ ] **Step 1: 追加样式块**

在 `ui/src/aurora.css` 末尾追加：

```css
/* ── Decision Drawer ── */

.aur-decision-head {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  align-items: flex-start;
}
.aur-decision-head__value {
  font-family: var(--font-mono);
  font-size: 1.5rem;
  font-weight: 600;
  letter-spacing: 0.04em;
}
.aur-decision-head__value[data-decision="True"]  { color: var(--aur-true); }
.aur-decision-head__value[data-decision="Hold"]  { color: var(--aur-hold); }
.aur-decision-head__value[data-decision="False"] { color: var(--aur-false); }
.aur-decision-head__phase,
.aur-decision-head__frame {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--aur-void);
}

/* Frame 张力行 */
.aur-tension-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: 0.25rem 0;
  font-size: var(--text-sm);
}
.aur-tension-row__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.aur-tension-row__dot[data-value="True"]    { background: var(--aur-true); }
.aur-tension-row__dot[data-value="Hold"]    { background: var(--aur-hold); }
.aur-tension-row__dot[data-value="False"]   { background: var(--aur-false); }
.aur-tension-row__dot[data-value="Unknown"] { background: var(--aur-void-dim); }
.aur-tension-row__frame {
  flex: 1;
  color: var(--aur-void);
}
.aur-tension-row__phase {
  font-family: var(--font-mono);
  color: var(--aur-void-dim);
}

/* 冲突项：左色条 + body（借鉴 worldmonitor renderSignal 横向布局） */
.aur-conflict-item {
  display: flex;
  align-items: stretch;
  margin-top: 0.4rem;
  background: rgba(255, 255, 255, 0.02);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.aur-conflict-item__bar {
  width: 4px;
  flex-shrink: 0;
  background: var(--aur-hold);
}
.aur-conflict-item__body {
  flex: 1;
  min-width: 0;
  padding: 0.4rem 0.5rem;
}
.aur-conflict-item__head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.aur-conflict-item__pair {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--aur-hold);
}
.aur-conflict-item__badge {
  font-family: var(--font-mono);
  font-size: 0.625rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--aur-void-dim);
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--aur-ice);
  border-radius: 2px;
  padding: 1px 5px;
}
.aur-conflict-item__reason {
  font-size: var(--text-sm);
  color: var(--aur-void);
  margin-top: 0.15rem;
}

/* ASI 仪表行内布局（复用 aur-gauge__track/fill，加自身 flex 排版） */
.aur-gauge {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.aur-gauge__track {
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.08);
  overflow: hidden;
}
.aur-gauge__fill {
  height: 100%;
  background: var(--aur-true);
  border-radius: 3px;
}
.aur-gauge__value {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--aur-true);
  min-width: 3ch;
  text-align: right;
}
```

注意：`.aur-gauge` / `.aur-gauge__track` / `.aur-gauge__fill` / `.aur-gauge__value` 在 `aurora.css` 中已有定义（M0 残留）。Step 1 追加的内容会与既有定义冲突。先执行 Step 1b 去重。

- [ ] **Step 1b: 去重既有 gauge 定义**

先检索既有 gauge 块：

Run: `cd ui && grep -n "aur-gauge" src/aurora.css`

既有块位于约 592–644 行（`aur-gauge` / `__header` / `__label` / `__value` / `__value--high/mid/low` / `__track` / `__fill` / `__fill--high/mid/low`）。

**决策**：既有 gauge 块是 M0 HTML 报告残留，未被任何 tsx 引用（已用 `grep -rn "aur-gauge" src/` 验证为空）。用本 Task Step 1 的内联 flex 版替换它——删除既有 592–644 行整块，追加 Step 1 的新块。`--high/mid/low` 修饰类一并删除（DecisionDrawer 用单一 `--aur-true` 色，不分级；如未来需分级再加）。

操作：用编辑器删除既有 `.aur-gauge` 到 `.aur-gauge__fill--low` 的整段，然后追加 Step 1 的新块。

- [ ] **Step 2: 构建确认无 CSS 语法错误**

Run: `cd ui && npm run build`
Expected: build 成功（CSS 由 Vite 处理，语法错误会导致构建失败）。

- [ ] **Step 3: 测试仍全绿**

Run: `cd ui && npm test`
Expected: 全部 PASS（5 个测试文件，含 DecisionDrawer 9 个）。

- [ ] **Step 4: 提交**

```bash
cd E:/trit-core
git add ui/src/aurora.css
git commit -m "style(ui): 决策抽屉样式（决策头/张力/冲突/ASI 仪表）

替换 M0 残留的 aur-gauge 块为 DecisionDrawer 使用的内联 flex 版。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: 顶栏 decision 标签改为可点击按钮 + App 接线

**Files:**
- Modify: `ui/src/TopBar.tsx`
- Modify: `ui/src/test/TopBar.test.tsx`
- Modify: `ui/src/App.tsx`

**Interfaces:**
- TopBar 新增 prop：`onOpenDecision: () => void`。
- App 新增 state `decisionDrawerOpen`，传 `onOpenDecision={() => setDecisionDrawerOpen(true)}`，渲染 `<DecisionDrawer open={decisionDrawerOpen} onClose={...} data={data} loading={loading} />`。

- [ ] **Step 1: 追加失败测试**

在 `ui/src/test/TopBar.test.tsx` 的 `defaultProps` 中加 `onOpenDecision: vi.fn()`，并追加测试：

```tsx
  it('calls onOpenDecision when decision label clicked', () => {
    const onOpenDecision = vi.fn();
    render(<TopBar {...defaultProps} decision="Hold" onOpenDecision={onOpenDecision} />);
    fireEvent.click(screen.getByText('Hold'));
    expect(onOpenDecision).toHaveBeenCalled();
  });

  it('disables decision button when decision is null', () => {
    render(<TopBar {...defaultProps} decision={null} />);
    // decision 为 null 时标签不渲染（既有行为），无从点击
    expect(screen.queryByText('Hold')).not.toBeInTheDocument();
  });
```

注：既有 `defaultProps` 缺 `onOpenDecision`，需补上（见 Step 3）。

- [ ] **Step 2: 运行测试确认失败**

Run: `cd ui && npm test -- TopBar`
Expected: FAIL — `onOpenDecision` 未传入 / decision 标签不可点。

- [ ] **Step 3: 改 TopBar**

在 `ui/src/TopBar.tsx`：

(a) Props 接口加 `onOpenDecision: () => void;`，函数参数解构加 `onOpenDecision`。

(b) `defaultProps` 测试 fixture 加 `onOpenDecision: vi.fn()`（在测试文件里改，不在 TopBar.tsx）。

(c) 把顶栏的 decision `<span>` 改为 `<button>`：

替换：
```tsx
      {decision && (
        <span className="aur-topbar-decision" data-decision={decision}>
          {decision}
        </span>
      )}
```
为：
```tsx
      {decision && (
        <button
          className="aur-topbar-decision aur-topbar-decision--btn"
          data-decision={decision}
          onClick={onOpenDecision}
          disabled={loading}
          title="查看决策结果"
        >
          {decision}
        </button>
      )}
```

- [ ] **Step 4: 运行 TopBar 测试确认通过**

Run: `cd ui && npm test -- TopBar`
Expected: PASS（含新 2 个，共 11 个）。

- [ ] **Step 5: App 接线**

在 `ui/src/App.tsx`：

(a) 顶部 import 加：
```tsx
import DecisionDrawer from './DecisionDrawer';
```

(b) 在 `const [settingsOpen, setSettingsOpen] = useState(false);` 下一行加：
```tsx
  const [decisionDrawerOpen, setDecisionDrawerOpen] = useState(false);
```

(c) 在 `<TopBar ... />` 调用中，于 `loading={loading}` 之后加 prop：
```tsx
        onOpenDecision={() => setDecisionDrawerOpen(true)}
```

(d) 在 `<Overlay ... />` 块之前（或之后，同级）加：
```tsx
      {/* 决策结果抽屉：点击顶栏 decision 标签展开 */}
      <DecisionDrawer
        open={decisionDrawerOpen}
        onClose={() => setDecisionDrawerOpen(false)}
        data={data}
        loading={loading}
      />
```

- [ ] **Step 6: 全量构建 + 测试**

Run: `cd ui && npm run build && npm test`
Expected: build 成功（TypeScript 干净），全部测试 PASS。

- [ ] **Step 7: 提交**

```bash
cd E:/trit-core
git add ui/src/TopBar.tsx ui/src/test/TopBar.test.tsx ui/src/App.tsx
git commit -m "feat(ui): 顶栏 decision 标签可点击打开决策抽屉

App 持有 decisionDrawerOpen state，TopBar decision 标签改为
button，点击展开 DecisionDrawer。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Esc 关闭决策抽屉 + 收尾验证

**Files:**
- Modify: `ui/src/App.tsx`

**Interfaces:** 无新接口；复用 App 既有键盘处理。

- [ ] **Step 1: Esc 同时关闭设置抽屉与决策抽屉**

在 `ui/src/App.tsx` 的键盘 `onKey` 处理中，`Escape` 分支当前只关 settings：

```tsx
      } else if (e.key === 'Escape') {
        if (settingsOpen) {
          setSettingsOpen(false);
        } else {
          ...
```

改为优先关任意打开的抽屉：

```tsx
      } else if (e.key === 'Escape') {
        if (settingsOpen) {
          setSettingsOpen(false);
        } else if (decisionDrawerOpen) {
          setDecisionDrawerOpen(false);
        } else {
          diag('Earth', 'INFO', 'Esc 退出');
          try {
            import('@tauri-apps/api/core').then(m => m.invoke('exit_app')).catch(() => window.close());
          } catch {
            window.close();
          }
        }
      }
```

并把 `decisionDrawerOpen` 加入该 `useEffect` 的依赖数组（当前是 `[handleRun, settingsOpen]`，改为 `[handleRun, settingsOpen, decisionDrawerOpen]`）。

- [ ] **Step 2: 全量验证**

Run: `cd ui && npm run build && npm test`
Expected: build 成功，全部测试 PASS。

- [ ] **Step 3: 更新 SESSION_START.md 当前进度**

在 `SESSION_START.md` 的"上次决策"表顶部加一行（日期 2026-07-01），并在"当前进度"Aurora 阶段行追加"决策结果抽屉完成（顶栏 decision 标签可点击 → 抽屉展示 phase/asi/signals/conflicts）"。

- [ ] **Step 4: 提交**

```bash
cd E:/trit-core
git add ui/src/App.tsx SESSION_START.md
git commit -m "feat(ui): Esc 关闭决策抽屉 + SESSION_START 更新

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Self-Review

**1. Spec 覆盖**：
- §2 触发与状态 → Task 5（TopBar button + App state）+ Task 6（Esc）。✓
- §3.2 决策头 → Task 1。✓
- §3.2 ASI 仪表 → Task 2。✓
- §3.2 Frame 张力 → Task 2。✓
- §3.2 冲突列表 → Task 3。✓
- §3.4 样式 → Task 4。✓
- §3.5 测试 → Task 1–3。✓
- §4 验收（build + test + 浏览器）→ 每个 Task 末尾 + Task 5/6 全量验证。浏览器手动验证不在自动化内，由实施者运行时确认。✓

**2. 占位符扫描**：无 TBD/TODO/"适当处理"。每个代码步骤含完整代码。✓

**3. 类型一致性**：
- `Props { open, onClose, data, loading }` — Task 1 定义，Task 5 App 传入匹配。✓
- `onOpenDecision: () => void` — Task 5 TopBar 定义，App 传入匹配。✓
- `decisionDrawerOpen` state — Task 5 定义，Task 6 引用一致。✓
- `PipelineResponse` 字段名（`asi`/`signals`/`conflicts`/`phase`/`final_frame`/`decision`）全部与 `ui/src/types.ts` 一致。✓

无问题。
