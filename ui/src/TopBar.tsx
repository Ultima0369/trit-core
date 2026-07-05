// ui/src/TopBar.tsx — Aurora HUD top bar (overlays the globe)
//
// Transparent gradient bar pressed over the globe. Product controls only:
// Run, 2D/3D view toggle, Reset, Fullscreen, Settings, globe texture + live
// decision indicator. Icons via lucide-react (consistent, cold-palette via
// currentColor).

import { useState, useEffect } from 'react';
import { Play, RotateCw, Maximize, Minimize, Settings, Map, Globe2, Layers, Eye } from 'lucide-react';
import type { GlobeTexture } from './types';
import { EVENTS } from './types';

interface Props {
  onRun: () => void;
  onToggleSettings: () => void;
  onReset: () => void;
  onToggleFullscreen: () => void;
  isFullscreen: boolean;
  /** 当前是否 2D 视图（驱动切换图标）。 */
  view2D: boolean;
  onToggleView: () => void;
  decision: string | null;
  loading: boolean;
  onOpenDecision: () => void;
  /** Whether the stagnation mirror is visible. */
  mirrorVisible: boolean;
  onToggleMirror: () => void;
}

export default function TopBar({
  onRun,
  onToggleSettings,
  onReset,
  onToggleFullscreen,
  isFullscreen,
  view2D,
  onToggleView,
  decision,
  loading,
  onOpenDecision,
  mirrorVisible,
  onToggleMirror,
}: Props) {
  const [globeTexture, setGlobeTexture] = useState<GlobeTexture>('blue-marble');

  useEffect(() => {
    const handler = (e: Event) => {
      const { texture } = (e as CustomEvent).detail as { texture: GlobeTexture };
      setGlobeTexture(texture);
    };
    window.addEventListener(EVENTS.GLOBE_TEXTURE_CHANGED, handler);
    return () => window.removeEventListener(EVENTS.GLOBE_TEXTURE_CHANGED, handler);
  }, []);

  const cycleTexture = () => {
    const next: GlobeTexture = globeTexture === 'blue-marble' ? 'topographic' : 'blue-marble';
    setGlobeTexture(next);
    window.dispatchEvent(new CustomEvent(EVENTS.SET_GLOBE_TEXTURE, {
      detail: { texture: next },
    }));
  };

  return (
    <header className="aur-topbar aur-topbar--overlay">
      <span className="aur-wordmark">Aurora</span>

      {/* 设置按钮置于左上角 wordmark 旁 */}
      <button
        className="aur-btn aur-btn--icon"
        onClick={onToggleSettings}
        title="设置"
      >
        <Settings size={16} />
      </button>

      <div className="aur-topbar-divider" />

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

      <div className="aur-topbar-spacer" />

      {/* 视图切换：2D ↔ 3D */}
      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={onToggleView}
        title={view2D ? '切到 3D 地球' : '切到 2D 地图'}
      >
        {view2D ? <Globe2 size={15} /> : <Map size={15} />}
      </button>

      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={cycleTexture}
        title={`地球纹理：${globeTexture === 'blue-marble' ? '蓝色弹珠' : '地形'}`}
      >
        <Layers size={15} />
      </button>

      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={onReset}
        title="刷新地球"
      >
        <RotateCw size={15} />
      </button>

      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={onToggleFullscreen}
        title={isFullscreen ? '退出全屏 (F11)' : '全屏 (F11)'}
      >
        {isFullscreen ? <Minimize size={15} /> : <Maximize size={15} />}
      </button>

      <button
        className={`aur-btn aur-btn--ghost aur-btn--small${mirrorVisible ? ' aur-btn--active' : ''}`}
        onClick={onToggleMirror}
        title={mirrorVisible ? '隐藏停滞镜' : '停滞镜'}
      >
        <Eye size={15} />
      </button>

      <button
        className="aur-btn aur-btn--primary aur-btn--small"
        onClick={onRun}
        disabled={loading}
        title="运行分析"
      >
        {loading ? '运行中…' : (<><Play size={14} /> 运行</>)}
      </button>

      <span className="aur-esc-hint">Esc 退出</span>
    </header>
  );
}
