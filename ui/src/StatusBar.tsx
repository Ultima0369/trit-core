// ui/src/StatusBar.tsx — 底部状态栏：坐标 / 缩放 / 比例尺 / 视图模式
//
// 地图工具的基本信息层。MapPanel 与 Earth 通过 onCoordChange 把统一的
// MapCoord 传到 App，再下发给本组件渲染。
import type { MapCoord } from './types';

interface Props {
  coord: MapCoord;
  /** "2D 地图" | "3D 地球" */
  mode: string;
}

/// 米/像素 → 人类可读比例尺（如 "100 km"）。
function formatScale(metersPerPixel: number | null): string {
  if (metersPerPixel == null || !isFinite(metersPerPixel)) return '—';
  const m = metersPerPixel * 100; // 100px 视野宽度
  if (m >= 1000) return `${(m / 1000).toFixed(m >= 10000 ? 0 : 1)} km`;
  return `${m.toFixed(0)} m`;
}

function fmt(v: number | null, digits = 3): string {
  if (v == null || !isFinite(v)) return '—';
  return v.toFixed(digits);
}

export default function StatusBar({ coord, mode }: Props) {
  return (
    <footer className="aur-statusbar">
      <div className="aur-statusbar__item">
        <span className="aur-statusbar__label">经度</span>
        <span className="aur-statusbar__value">{fmt(coord.lng, 3)}°</span>
      </div>
      <div className="aur-statusbar__item">
        <span className="aur-statusbar__label">纬度</span>
        <span className="aur-statusbar__value">{fmt(coord.lat, 3)}°</span>
      </div>
      <span className="aur-statusbar__sep">·</span>
      <div className="aur-statusbar__item">
        <span className="aur-statusbar__label">{mode === '3D 地球' ? '海拔' : '缩放'}</span>
        <span className="aur-statusbar__value">
          {mode === '3D 地球' ? `${fmt(coord.zoom, 0)} km` : `z${fmt(coord.zoom, 1)}`}
        </span>
      </div>
      <div className="aur-statusbar__item">
        <span className="aur-statusbar__label">比例尺</span>
        <span className="aur-statusbar__value">{formatScale(coord.scale)}</span>
      </div>
      <div className="aur-statusbar__spacer" />
      <div className="aur-statusbar__item">
        <span className="aur-statusbar__mode">{mode}</span>
      </div>
    </footer>
  );
}
