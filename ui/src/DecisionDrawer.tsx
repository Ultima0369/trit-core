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
      </div>
    </div>
  );
}
