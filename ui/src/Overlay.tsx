// ui/src/Overlay.tsx — Aurora settings drawer (signal params, assets, cache)
//
// Pulled out of the old single analysis panel. The HUD panels (LeftHud/RightHud)
// live separately; this drawer is toggled from the TopBar ⚙ button.

import { useState, useEffect, useCallback, useRef } from 'react';
import type { CacheStats, PipelineRequest } from './types';
import diag, { isTauriEnvironment } from './utils/diag';

interface Props {
  open: boolean;
  onClose: () => void;
  resumeDelayMs: number;
  onResumeDelayChange: (ms: number) => void;
  fontScale: number;
  onFontScaleChange: (scale: number) => void;
  rotationSpeed: number;
  onRotationSpeedChange: (deg: number) => void;
  pipelineRequest: PipelineRequest;
  onPipelineRequestChange: (req: PipelineRequest) => void;
}

// 自转恢复：固定延迟档位。拖动地球后，选定的延迟过后恢复自转。
const RESUME_OPTIONS = [
  { value: 60000, label: '1 分钟' },
  { value: 180000, label: '3 分钟' },
  { value: 300000, label: '5 分钟' },
  { value: 600000, label: '10 分钟' },
  { value: 0, label: '关闭自转' },
];

interface AssetInfo {
  name: string;
  category: string;
  status: string;
  size: number;
  size_human: string;
  source: string;
  error: string;
}

interface AssetReport {
  assets_dir: string;
  assets: AssetInfo[];
  all_ready: boolean;
}

const ASSET_LABELS: Record<string, string> = {
  'earth-blue-marble.jpg': '蓝色弹珠',
  'earth-topology.png': '地形图',
  'night-sky.png': '夜空',
  'Cesium.js': 'CesiumJS',
  'tilemapresource.xml': '瓦片元数据',
};

const CATEGORY_LABELS: Record<string, string> = {
  texture: '纹理',
  cesium: 'CesiumJS',
  tiles: '全球瓦片',
  'china-tiles': '中国卫星影像',
  'global-tiles': '全球卫星影像',
  terrain: '地形',
};

function statusBadge(status: string): { char: string; color: string; label: string } {
  switch (status) {
    case 'cached':  return { char: '✓', color: 'var(--aur-true)', label: '已缓存' };
    case 'ok':      return { char: '✓', color: 'var(--aur-true)', label: '就绪' };
    case 'missing': return { char: '○', color: 'var(--aur-hold)', label: '缺失' };
    case 'failed':  return { char: '✗', color: 'var(--aur-false)', label: '失败' };
    default:        return { char: '?', color: 'var(--aur-void-dim)', label: status };
  }
}

function humanSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${bytes} B`;
}

export default function Overlay({
  open, onClose,
  resumeDelayMs, onResumeDelayChange,
  fontScale, onFontScaleChange,
  rotationSpeed, onRotationSpeedChange,
  pipelineRequest, onPipelineRequestChange,
}: Props) {
  const [assetReport, setAssetReport] = useState<AssetReport | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [cacheStats, setCacheStats] = useState<CacheStats | null>(null);
  const pollRef = useRef<{ pollId: ReturnType<typeof setInterval>; safetyTimeout: ReturnType<typeof setTimeout> } | null>(null);

  const refreshAssetStatus = useCallback(async () => {
    if (!isTauriEnvironment()) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const report = await invoke<AssetReport>('get_asset_status');
      setAssetReport(report);
    } catch (e) {
      diag('Overlay', 'WARN', `获取纹理状态失败: ${e}`);
    }
  }, []);

  useEffect(() => { refreshAssetStatus(); }, [refreshAssetStatus]);
  useEffect(() => {
    if (open) {
      refreshAssetStatus();
    } else {
      // ponytail: cleanup china-tile poll when settings drawer closes
      if (pollRef.current) {
        clearInterval(pollRef.current.pollId);
        clearTimeout(pollRef.current.safetyTimeout);
        pollRef.current = null;
        setDownloading(false);
      }
    }
  }, [open, refreshAssetStatus]);

  const refreshCacheStats = useCallback(async () => {
    if (!isTauriEnvironment()) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const stats = await invoke<CacheStats>('cache_stats');
      setCacheStats(stats);
    } catch (e) {
      diag('Overlay', 'WARN', `获取缓存统计失败: ${e}`);
    }
  }, []);

  useEffect(() => {
    if (open) {
      refreshCacheStats();
      const id = setInterval(refreshCacheStats, 10000);
      return () => clearInterval(id);
    }
  }, [open, refreshCacheStats]);

  const handleDownload = useCallback(async (force: boolean) => {
    if (!isTauriEnvironment() || downloading) return;
    setDownloading(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      diag('Overlay', 'INFO', `手动下载 (force=${force})...`);
      const report = await invoke<AssetReport>('download_assets', { force });
      setAssetReport(report);

      const tileAsset = report.assets.find(a => a.category === 'global-tiles');
      if (tileAsset && tileAsset.status === 'ok' && tileAsset.source === '后台下载中') {
        diag('Overlay', 'INFO', '瓦片后台下载中，启动轮询...');
        const pollId = setInterval(async () => {
          const status = await invoke<AssetReport>('get_asset_status');
          setAssetReport(status);
          const tile = status.assets.find(a => a.category === 'global-tiles');
          if (tile && (tile.status === 'cached' || tile.status === 'failed')) {
            diag('Overlay', 'INFO', `瓦片下载完成: ${tile.status}`);
            clearInterval(pollId);
            setDownloading(false);
          }
        }, 3000);
        const safetyTimeout = setTimeout(() => { clearInterval(pollId); setDownloading(false); }, 300000);
        pollRef.current = { pollId, safetyTimeout };
        return;
      }
      diag('Overlay', 'INFO', `下载完成: ${report.assets.filter(a => a.status === 'ok' || a.status === 'cached').length}/${report.assets.length} 就绪`);
      setDownloading(false);
    } catch (e) {
      diag('Overlay', 'ERROR', `手动下载失败: ${e}`);
      setDownloading(false);
    }
  }, [downloading]);

  if (!open) return null;

  return (
    <div className="aur-settings-drawer">
      <div className="aur-settings-drawer__header">
        <span className="aur-panel-title">设置</span>
        <button className="aur-btn aur-btn--icon" onClick={onClose} title="关闭">✕</button>
      </div>

      <div className="aur-settings-drawer__body">
        {/* Globe rotation */}
        <div className="aur-settings__section">
          <div className="aur-settings__title">地球旋转</div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">拖动后恢复</span>
            <select
              className="aur-settings__select"
              value={resumeDelayMs}
              onChange={e => onResumeDelayChange(Number(e.target.value))}
            >
              {RESUME_OPTIONS.map(opt => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">转动速度 ({rotationSpeed}°/s)</span>
            <input
              type="range"
              className="aur-settings__range"
              min={0} max={30} step={1}
              value={rotationSpeed}
              onChange={e => onRotationSpeedChange(Number(e.target.value))}
            />
          </div>
        </div>

        {/* Display */}
        <div className="aur-settings__section">
          <div className="aur-settings__title">显示</div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">字体大小 ({Math.round(fontScale * 100)}%)</span>
            <input
              type="range"
              className="aur-settings__range"
              min={0.8} max={1.3} step={0.05}
              value={fontScale}
              onChange={e => onFontScaleChange(Number(e.target.value))}
            />
          </div>
        </div>

        {/* Pipeline parameters */}
        <div className="aur-settings__section">
          <div className="aur-settings__title">信号参数</div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">频率 (Hz)</span>
            <input
              type="number"
              className="aur-settings__input"
              min={0.1} max={50} step={0.1}
              value={pipelineRequest.freq}
              onChange={e => onPipelineRequestChange({ ...pipelineRequest, freq: parseFloat(e.target.value) || 2.0 })}
            />
          </div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">采样率 (Hz)</span>
            <input
              type="number"
              className="aur-settings__input"
              min={10} max={1000} step={10}
              value={pipelineRequest.sample_rate}
              onChange={e => onPipelineRequestChange({ ...pipelineRequest, sample_rate: parseFloat(e.target.value) || 100 })}
            />
          </div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">时长 (秒)</span>
            <input
              type="number"
              className="aur-settings__input"
              min={0.1} max={10} step={0.1}
              value={pipelineRequest.duration_secs}
              onChange={e => onPipelineRequestChange({ ...pipelineRequest, duration_secs: parseFloat(e.target.value) || 1.0 })}
            />
          </div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">噪声 σ</span>
            <input
              type="number"
              className="aur-settings__input"
              min={0} max={1} step={0.01}
              value={pipelineRequest.noise_std}
              onChange={e => onPipelineRequestChange({ ...pipelineRequest, noise_std: parseFloat(e.target.value) || 0.1 })}
            />
          </div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">频率阈值 (Hz)</span>
            <input
              type="number"
              className="aur-settings__input"
              min={0.1} max={10} step={0.1}
              value={pipelineRequest.frequency_threshold}
              onChange={e => onPipelineRequestChange({ ...pipelineRequest, frequency_threshold: parseFloat(e.target.value) || 1.5 })}
            />
          </div>
          <div className="aur-settings__row">
            <span className="aur-settings__label">用户感觉正常</span>
            <input
              type="checkbox"
              checked={pipelineRequest.user_feels_normal}
              onChange={e => onPipelineRequestChange({ ...pipelineRequest, user_feels_normal: e.target.checked })}
            />
          </div>
        </div>

        {/* Assets */}
        <div className="aur-settings__section">
          <div className="aur-settings__title">资源</div>

          {assetReport ? (
            <>
              {(() => {
                const categories = [...new Set(assetReport.assets.map(a => a.category))];
                return categories.map(cat => {
                  const items = assetReport.assets.filter(a => a.category === cat);
                  const cached = items.filter(a => a.status === 'cached' || a.status === 'ok').length;
                  const totalSize = items.reduce((sum, a) => sum + a.size, 0);
                  const allOk = items.every(a => a.status === 'cached' || a.status === 'ok');
                  return (
                    <div key={cat} className="aur-asset-category">
                      <div className="aur-asset-category__header">
                        <span
                          className="aur-asset-category__title"
                          style={{ color: allOk ? 'var(--aur-true)' : 'var(--aur-hold)' }}
                        >
                          {allOk ? '✓' : '○'} {CATEGORY_LABELS[cat] || cat}
                        </span>
                        <span className="aur-asset-category__meta">
                          {cached}/{items.length}{totalSize > 0 ? ` (${humanSize(totalSize)})` : ''}
                        </span>
                      </div>
                      {(cat === 'texture' || cat === 'china-tiles' || cat === 'global-tiles') && items.map(asset => {
                        const label = ASSET_LABELS[asset.name] || asset.name;
                        const badge = statusBadge(asset.status);
                        return (
                          <div key={asset.name} className="aur-asset-row">
                            <span className="aur-asset-row__name">{label}</span>
                            <span className="aur-asset-row__status" style={{ color: badge.color }}>
                              {badge.char} {badge.label}
                            </span>
                            {asset.size > 0 && (
                              <span className="aur-asset-row__size">{asset.size_human}</span>
                            )}
                            {asset.error && (
                              <span className="aur-asset-row__size" style={{ color: 'var(--aur-hold)' }}>
                                {asset.error}
                              </span>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  );
                });
              })()}

              <div className="aur-asset-summary">
                {assetReport.all_ready
                  ? '✓ 所有资源就绪'
                  : `${assetReport.assets.filter(a => a.status === 'missing' || a.status === 'failed').length} 个文件缺失`}
              </div>

              <div className="aur-asset-actions">
                <button
                  className="aur-btn aur-btn--ghost aur-btn--small"
                  onClick={() => handleDownload(false)}
                  disabled={downloading || assetReport.all_ready}
                >
                  {downloading ? '下载中…' : '下载缺失项'}
                </button>
                <button
                  className="aur-btn aur-btn--ghost aur-btn--small"
                  onClick={() => handleDownload(true)}
                  disabled={downloading}
                >
                  强制重新下载
                </button>
              </div>
            </>
          ) : (
            <div className="aur-asset-summary">
              {isTauriEnvironment() ? '加载中…' : '仅 Tauri 环境'}
            </div>
          )}
        </div>

        {/* Cache stats */}
        {cacheStats && (
          <div className="aur-settings__section">
            <div className="aur-settings__title">缓存</div>

            <div className="aur-cache-grid">
              <div className="aur-cache-item">
                <span className="aur-cache-item__label">L1 命中率</span>
                <span className="aur-cache-item__value">{(cacheStats.l1_hit_rate * 100).toFixed(1)}%</span>
              </div>
              <div className="aur-cache-item">
                <span className="aur-cache-item__label">L1 条目</span>
                <span className="aur-cache-item__value">{cacheStats.l1_entries}</span>
              </div>
              <div className="aur-cache-item">
                <span className="aur-cache-item__label">L2 命中率</span>
                <span className="aur-cache-item__value">{(cacheStats.l2_hit_rate * 100).toFixed(1)}%</span>
              </div>
              <div className="aur-cache-item">
                <span className="aur-cache-item__label">L2 文件</span>
                <span className="aur-cache-item__value">{cacheStats.l2_files.toLocaleString()}</span>
              </div>
              <div className="aur-cache-item">
                <span className="aur-cache-item__label">L2 大小</span>
                <span className="aur-cache-item__value">{humanSize(cacheStats.l2_bytes)}</span>
              </div>
              <div className="aur-cache-item">
                <span className="aur-cache-item__label">下载 成功/失败</span>
                <span className="aur-cache-item__value">{cacheStats.downloads_ok}/{cacheStats.downloads_fail}</span>
              </div>
            </div>

            <div className="aur-settings__row">
              <span className="aur-settings__label">L2 上限 (GB，0=不限)</span>
              <input
                type="number"
                min={0}
                max={1000}
                className="aur-cache-input"
                defaultValue={Math.floor(cacheStats.l2_max_bytes / 1024 / 1024 / 1024)}
                onBlur={async (e) => {
                  const gb = parseInt(e.target.value) || 0;
                  try {
                    const { invoke } = await import('@tauri-apps/api/core');
                    await invoke<string>('set_cache_limit', { maxGb: gb });
                    refreshCacheStats();
                  } catch (err) {
                    diag('Overlay', 'ERROR', `设上限失败: ${err}`);
                  }
                }}
              />
            </div>

            <div className="aur-asset-actions">
              <button
                className="aur-btn aur-btn--ghost aur-btn--small"
                onClick={async () => {
                  try {
                    const { invoke } = await import('@tauri-apps/api/core');
                    await invoke<string>('clear_cache');
                    refreshCacheStats();
                  } catch (err) {
                    diag('Overlay', 'ERROR', `清空失败: ${err}`);
                  }
                }}
              >
                清空缓存
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
