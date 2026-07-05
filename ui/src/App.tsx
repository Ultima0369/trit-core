// ui/src/App.tsx — Aurora HUD: globe backdrop + floating decision panels
//
// Globe fills the viewport; HUD panels float over it. No Sandcastle editor.

import { useState, useCallback, useEffect, useRef, lazy, Suspense } from 'react';
import Earth from './Earth';
import TopBar from './TopBar';
import Overlay from './Overlay';
import DecisionDrawer from './DecisionDrawer';
import MirrorOverlay from './MirrorOverlay';
import Sidebar from './Sidebar';
import StatusBar from './StatusBar';
import BootScreen from './BootScreen';
import { useBootFlow } from './useBootFlow';
import diag, { isTauriEnvironment } from './utils/diag';
import type { MapCoord, PipelineRequest, PipelineResponse } from './types';
import { DEFAULT_LAYERS, type MapLayers } from './config/layer-definitions';

// 2D 矢量地图面板懒加载 — maplibre/pmtiles/protomaps 不进主 bundle。
const MapPanel = lazy(() => import('./MapPanel'));

const DEFAULT_PIPELINE_REQUEST: PipelineRequest = {
  freq: 2.0,
  sample_rate: 100.0,
  duration_secs: 1.0,
  noise_std: 0.1,
  frequency_threshold: 1.5,
  user_feels_normal: true,
};

const DEFAULT_RESUME_DELAY_MS = 60000; // 1 分钟（固定延迟，见 Earth pauseRotation）
const DEFAULT_FONT_SCALE = 1;
const DEFAULT_ROTATION_SPEED = 2;

// localStorage 读取（带解析容错，Tauri/浏览器通用）。
// ponytail: getItem 返回 null（键不存在）必须走 fallback——Number(null)=0 会把
// fontScale 变成 0，导致 calc(rem * 0)=0、所有文字 0px、按钮文字/图标不可见。
export function readStored(key: string, fallback: number): number {
  const raw = localStorage.getItem(key);
  if (raw === null) return fallback;
  const v = Number(raw);
  return Number.isFinite(v) && v >= 0 ? v : fallback;
}



async function invokeRunPipeline(req: PipelineRequest): Promise<PipelineResponse> {
  if (!isTauriEnvironment()) {
    diag('invoke', 'ERROR', '无 Tauri 后端 — 无法运行管线分析');
    throw new Error('无 Tauri 后端 — 请在 Aurora 桌面应用中运行');
  }
  diag('invoke', 'INFO', '调用 run_analysis_pipeline (Tauri)');
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const result = await invoke<PipelineResponse>('run_analysis_pipeline', { request: req });
    diag('invoke', 'INFO', `pipeline 返回成功: decision=${result.decision} asi=${result.asi}`);
    return result;
  } catch (e: any) {
    diag('invoke', 'ERROR', `Tauri invoke 失败: ${e}`);
    throw e;
  }
}

export default function App() {
  const [data, setData] = useState<PipelineResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [resumeDelayMs, setResumeDelayMs] = useState(DEFAULT_RESUME_DELAY_MS);
  const [fontScale, setFontScale] = useState(() => {
    // fontScale 必须 > 0，否则 --font-scale:0 让所有文字归零、按钮不可见
    const v = readStored('aurora.fontScale', DEFAULT_FONT_SCALE);
    return v > 0 ? v : DEFAULT_FONT_SCALE;
  });
  const [rotationSpeed, setRotationSpeed] = useState(() => readStored('aurora.rotationSpeed', DEFAULT_ROTATION_SPEED));
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [decisionDrawerOpen, setDecisionDrawerOpen] = useState(false);
  const [view2D, setView2D] = useState(false);
  const [mapLayers, setMapLayers] = useState<MapLayers>(DEFAULT_LAYERS);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [coord, setCoord] = useState<MapCoord>({ lng: null, lat: null, zoom: null, scale: null });
  const [pipelineRequest, setPipelineRequest] = useState(DEFAULT_PIPELINE_REQUEST);
  const [mirrorVisible, setMirrorVisible] = useState(false);

  // 启动过场状态机（cesiumReady/earthEntering/bootDone + 60s 兜底）收口到 hook。
  const boot = useBootFlow();

  const initialRunDone = useRef(false);

  const handleRun = useCallback(async () => {
    const { freq, sample_rate, duration_secs, noise_std, frequency_threshold } = pipelineRequest;
    if (!isFinite(freq) || freq <= 0 || freq > 1000) { diag('App', 'ERROR', `无效频率: ${freq}`); return; }
    if (!isFinite(sample_rate) || sample_rate <= 0 || sample_rate > 100000) { diag('App', 'ERROR', `无效采样率: ${sample_rate}`); return; }
    if (!isFinite(duration_secs) || duration_secs <= 0 || duration_secs > 3600) { diag('App', 'ERROR', `无效时长: ${duration_secs}`); return; }
    if (!isFinite(noise_std) || noise_std < 0 || noise_std > 100) { diag('App', 'ERROR', `无效噪声: ${noise_std}`); return; }
    if (!isFinite(frequency_threshold) || frequency_threshold <= 0 || frequency_threshold > 1000) { diag('App', 'ERROR', `无效阈值: ${frequency_threshold}`); return; }

    diag('App', 'INFO', '运行 Aurora 管线分析');
    setLoading(true);
    try {
      const result = await invokeRunPipeline(pipelineRequest);
      setData(result);
      diag('App', 'INFO', '分析完成，数据已更新');
    } catch (err) {
      diag('App', 'ERROR', `分析失败: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [pipelineRequest]);

  const handleReset = useCallback(() => {
    diag('App', 'INFO', '重置地球');
    // Refresh tiles first (Tauri only): clears china-tiles cache and force-redownloads,
    // so a source fix (e.g. Esri Y-axis) takes effect instead of reloading stale files.
    if (isTauriEnvironment()) {
      import('@tauri-apps/api/core')
        .then(({ invoke }) => invoke('refresh_tiles'))
        .catch(e => diag('App', 'WARN', `刷新瓦片失败: ${e}`));
    }
    window.dispatchEvent(new CustomEvent('aurora-reset-globe'));
  }, []);

  // 全屏：Tauri setFullscreen 优先，浏览器 requestFullscreen 回落。
  // 之前是死的 onToggleFullscreen 只打日志——现在真切换。
  const handleToggleFullscreen = useCallback(async () => {
    const next = !isFullscreen;
    if (isTauriEnvironment()) {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setFullscreen(next);
        setIsFullscreen(next);
      } catch (e) {
        diag('App', 'WARN', `Tauri 全屏失败: ${e}`);
      }
    } else if (document.fullscreenElement != null) {
      await document.exitFullscreen();
      setIsFullscreen(false);
    } else {
      await document.documentElement.requestFullscreen();
      setIsFullscreen(true);
    }
  }, [isFullscreen]);

  // 字体缩放：写入 :root --font-scale，CSS 全局缩放所有 rem 文字
  useEffect(() => {
    document.documentElement.style.setProperty('--font-scale', String(fontScale));
    localStorage.setItem('aurora.fontScale', String(fontScale));
  }, [fontScale]);

  // 转动速度持久化
  useEffect(() => {
    localStorage.setItem('aurora.rotationSpeed', String(rotationSpeed));
  }, [rotationSpeed]);

  // Keyboard shortcuts
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      if (ctrl && e.key === 'Enter') {
        e.preventDefault();
        handleRun();
      } else if (e.key === 'F11') {
        e.preventDefault();
        diag('App', 'INFO', 'F11 全屏（由系统处理）');
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
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [handleRun, settingsOpen, decisionDrawerOpen]);

  useEffect(() => {
    if (initialRunDone.current) return;
    initialRunDone.current = true;

    const isTauri = isTauriEnvironment();
    diag('App', 'INFO', `应用启动 — 环境: ${isTauri ? 'Tauri' : '浏览器'}`);
    diag('App', 'INFO', `location: ${location.href}`);

    handleRun();
  }, [handleRun]);

  return (
    <div className="aur-app aur-app--hud">
      <TopBar
        onRun={handleRun}
        onToggleSettings={() => setSettingsOpen(o => !o)}
        onReset={handleReset}
        onToggleFullscreen={handleToggleFullscreen}
        isFullscreen={isFullscreen}
        view2D={view2D}
        onToggleView={() => setView2D(v => !v)}
        decision={data?.decision ?? null}
        loading={loading}
        onOpenDecision={() => setDecisionDrawerOpen(true)}
        mirrorVisible={mirrorVisible}
        onToggleMirror={() => setMirrorVisible(v => !v)}
      />

      {/* 左侧栏目：图层 + 数据摘要（2D/3D 视图都保留） */}
      <Sidebar layers={mapLayers} onLayersChange={setMapLayers} />

      {/* Globe / Map backdrop fills the content area.
          is-entering = 初始小尺寸+裁剪态（被 BootScreen 盖住）；
          earthEntering 触发时移除该 class → CSS 过渡放大到全屏（地球登场）。 */}
      <div className={`aur-globe-area aur-globe-area--full${(!boot.earthEntering && !boot.bootDone) ? ' is-entering' : ''}`}>
        <MirrorOverlay
          visible={mirrorVisible}
          onClose={() => setMirrorVisible(false)}
        />
        {view2D ? (
          <Suspense fallback={<div style={{ color: '#fff' }}>加载 2D 地图…</div>}>
            <MapPanel flavor="light" layers={mapLayers} onCoordChange={setCoord} />
          </Suspense>
        ) : (
          <Earth
            resumeDelayMs={resumeDelayMs}
            rotationSpeed={rotationSpeed}
            onViewChange={setCoord}
            onReady={boot.setEarthReady}
          />
        )}
      </div>

      {/* 底部状态栏：坐标 / 缩放 / 比例尺 / 视图模式 */}
      <StatusBar coord={coord} mode={view2D ? '2D 地图' : '3D 地球'} />

      {/* 决策结果抽屉：点击顶栏 decision 标签展开 */}
      <DecisionDrawer
        open={decisionDrawerOpen}
        onClose={() => setDecisionDrawerOpen(false)}
        data={data}
        loading={loading}
      />

      {/* Settings drawer */}
      <Overlay
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        resumeDelayMs={resumeDelayMs}
        onResumeDelayChange={setResumeDelayMs}
        fontScale={fontScale}
        onFontScaleChange={setFontScale}
        rotationSpeed={rotationSpeed}
        onRotationSpeedChange={setRotationSpeed}
        pipelineRequest={pipelineRequest}
        onPipelineRequestChange={setPipelineRequest}
      />

      {/* 启动加载屏：太阳系转动 + 里程碑读条，淡出后卸载 */}
      {!boot.bootDone && (
        <BootScreen
          externalMilestones={[
            { label: '地球引擎', done: view2D || boot.cesiumReady },
            { label: '首次分析', done: data != null },
          ]}
          fadeOut={boot.earthEntering}
          onFadeComplete={boot.onFadeComplete}
          onTransitionReady={boot.onTransitionReady}
        />
      )}
    </div>
  );
}
