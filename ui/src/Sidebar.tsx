// ui/src/Sidebar.tsx — 左侧栏目
//
// 借鉴 worldmonitor：把图层选择器从地图浮层提升为左侧正式栏目，并加
// 数据摘要区（anchor 违例数 / 冲突事件数）。layers state 受控，由 App
// 持有，Sidebar 与 MapPanel 共享同一份开关状态。
import { useEffect, useState, useRef } from 'react';
import { Thermometer, Leaf, Swords, Download, Check, X, type LucideIcon } from 'lucide-react';
import diag, { isTauriEnvironment } from './utils/diag';
import {
  getExecutableLayers,
  type MapLayers,
  type LayerKey,
} from './config/layer-definitions';
import type { AnchorStatus, GeoEvent } from './types';

interface Props {
  layers: MapLayers;
  onLayersChange: (layers: MapLayers) => void;
}

/// 图层 key → lucide 图标组件（替代原 HTML 实体 emoji，跨系统一致）。
const LAYER_ICONS: Record<LayerKey, LucideIcon> = {
  thermalStations: Thermometer,
  ecologicalStations: Leaf,
  geoEvents: Swords,
};

/// 侧边栏宽度持久化 key + 钳制范围。
const SIDEBAR_W_KEY = 'aurora.sidebarWidth';
const SIDEBAR_W_MIN = 200;
const SIDEBAR_W_MAX = 480;
const SIDEBAR_W_DEFAULT = 280;

/// 拉取 anchor 状态摘要（违例计数）。
async function fetchAnchorStatus(): Promise<AnchorStatus[]> {
  if (!isTauriEnvironment()) return [];
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AnchorStatus[]>('get_anchor_status', { degraded: false });
}

/// 拉取冲突事件摘要（计数）。
async function fetchGeoEvents(): Promise<GeoEvent[]> {
  if (!isTauriEnvironment()) return [];
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GeoEvent[]>('get_geo_events');
}

export default function Sidebar({ layers, onLayersChange }: Props) {
  const [anchorStatus, setAnchorStatus] = useState<AnchorStatus[]>([]);
  const [events, setEvents] = useState<GeoEvent[]>([]);
  const [exporting, setExporting] = useState(false);
  const draggingRef = useRef(false);

  /// 挂载时恢复持久化的侧边栏宽度到 :root --aur-sidebar-w。
  /// ponytail: 宽度是 CSS 变量驱动 grid 列宽，改变量即可让 content 区重排。
  useEffect(() => {
    const stored = Number(localStorage.getItem(SIDEBAR_W_KEY));
    const w = Number.isFinite(stored) && stored >= SIDEBAR_W_MIN && stored <= SIDEBAR_W_MAX
      ? stored : SIDEBAR_W_DEFAULT;
    document.documentElement.style.setProperty('--aur-sidebar-w', `${w}px`);
  }, []);

  /// 拖拽分隔条：mousedown 进入拖拽，mousemove 改宽度，mouseup 退出 + 持久化。
  const onResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    draggingRef.current = true;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    const onMove = (ev: MouseEvent) => {
      if (!draggingRef.current) return;
      // ponytail: clamp 到 [MIN, MAX]，防止拖到看不见或过宽挤压地图。
      const w = Math.min(SIDEBAR_W_MAX, Math.max(SIDEBAR_W_MIN, ev.clientX));
      document.documentElement.style.setProperty('--aur-sidebar-w', `${w}px`);
    };
    const onUp = () => {
      draggingRef.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      const cur = getComputedStyle(document.documentElement).getPropertyValue('--aur-sidebar-w');
      const w = Number(cur.replace(/px/, ''));
      if (Number.isFinite(w)) localStorage.setItem(SIDEBAR_W_KEY, String(w));
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  };

  /// 导出全部用户数据为 JSON 并触发浏览器下载。
  /// M1 "数据导出" + CHARTER "不剥夺"：用户可带走自己的数据。
  /// ponytail: Blob + a[download]，纯 web 标准，不依赖 fs/dialog 插件。
  const handleExport = async () => {
    if (!isTauriEnvironment()) {
      diag('Sidebar', 'WARN', '导出需要 Tauri 桌面环境');
      return;
    }
    setExporting(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const json = await invoke<string>('export_user_data');
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `aurora-data-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
      diag('Sidebar', 'INFO', '数据已导出');
    } catch (e) {
      diag('Sidebar', 'ERROR', `导出失败: ${e}`);
    } finally {
      setExporting(false);
    }
  };

  // 摘要数据：挂载时拉一次。ponytail: 不轮询——数据由后台线程刷新，
  // 用户切换图层回到面板时再拉即可。失败静默（摘要非关键）。
  // 摘要数据：挂载时拉 + geoEvents 图层启用时刷新事件计数。
  useEffect(() => {
    Promise.all([fetchAnchorStatus(), fetchGeoEvents()])
      .then(([a, g]) => {
        setAnchorStatus(a);
        setEvents(g);
      })
      .catch(e => diag('Sidebar', 'WARN', `摘要拉取失败: ${e}`));
  }, [layers.geoEvents]);

  const toggleLayer = (key: LayerKey) => {
    onLayersChange({ ...layers, [key]: !layers[key] });
  };

  const executableLayers = getExecutableLayers('flat');
  const stateBased = events.filter(e => e.violence_type === 'state-based').length;
  const nonState = events.filter(e => e.violence_type === 'non-state').length;
  const oneSided = events.filter(e => e.violence_type === 'one-sided').length;

  return (
    <aside className="aur-sidebar">
      {/* ── 图层栏目 ── */}
      <section className="aur-sidebar-section">
        <div className="aur-sidebar-title">图层</div>
        {executableLayers.map(def => {
          const Icon = LAYER_ICONS[def.key];
          return (
            <label key={def.key} className="aur-layer-row">
              <input
                type="checkbox"
                checked={layers[def.key]}
                onChange={() => toggleLayer(def.key)}
              />
              <span className="aur-layer-icon">{Icon && <Icon size={15} />}</span>
              <span className="aur-layer-label">{def.label}</span>
            </label>
          );
        })}
      </section>

      {/* ── Anchor 状态摘要 ── */}
      <section className="aur-sidebar-section">
        <div className="aur-sidebar-title">Anchor 基线</div>
        {anchorStatus.map(a => (
          <div key={a.kind} className="aur-summary-row">
            <span className={a.has_violations ? 'aur-mark-violated' : 'aur-mark-ok'}>
              {a.has_violations ? <X size={14} /> : <Check size={14} />}
            </span>
            <span>{a.kind === 'thermal' ? '热基线' : '生态基线'}</span>
            <span className="aur-summary-sub">
              {a.readings.filter(r => r.violated).length}/{a.readings.length} 违例
            </span>
          </div>
        ))}
        {anchorStatus.length === 0 && (
          <div className="aur-summary-empty">无数据（离线）</div>
        )}
      </section>

      {/* ── 冲突事件摘要 ── */}
      <section className="aur-sidebar-section">
        <div className="aur-sidebar-title">冲突事件 (UCDP)</div>
        <div className="aur-summary-row">
          <span className="aur-dot aur-dot--red" />
          <span>国家间</span>
          <span className="aur-summary-sub">{stateBased}</span>
        </div>
        <div className="aur-summary-row">
          <span className="aur-dot aur-dot--orange" />
          <span>非国家</span>
          <span className="aur-summary-sub">{nonState}</span>
        </div>
        <div className="aur-summary-row">
          <span className="aur-dot aur-dot--yellow" />
          <span>单边</span>
          <span className="aur-summary-sub">{oneSided}</span>
        </div>
        {events.length === 0 && (
          <div className="aur-summary-empty">无数据（离线）</div>
        )}
      </section>

      {/* ── 数据导出（M1 验收 + CHARTER 不剥夺）── */}
      <section className="aur-sidebar-section">
        <div className="aur-sidebar-title">数据主权</div>
        <button className="aur-btn aur-btn--block" onClick={handleExport} disabled={exporting}>
          <Download size={14} /> {exporting ? '导出中…' : '导出我的数据 (JSON)'}
        </button>
        <div className="aur-sidebar-note">全部 SQLite 数据，可离线带走。</div>
      </section>

      {/* 可拖拽分隔条：拖动改 --aur-sidebar-w，grid 自动重排 */}
      <div className="aur-sidebar-resizer" onMouseDown={onResizeStart} title="拖动调整宽度" />
    </aside>
  );
}
