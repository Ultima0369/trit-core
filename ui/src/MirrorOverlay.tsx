// ui/src/MirrorOverlay.tsx — Stagnation Mirror dashboard (Lever 3)
//
// Semi-transparent overlay on the 3D globe showing human activity indicators
// alongside planetary boundary readings — "增长 vs 承载" scissors gap.
// Polls get_mirror_snapshot every 60 seconds. Silent on failure.

import { useCallback, useEffect, useRef, useState } from 'react';
import type { MirrorSnapshot, MirrorIndicator } from './types';

const POLL_INTERVAL_MS = 60_000;

interface MirrorOverlayProps {
  /** Whether the mirror is visible. Controlled by parent. */
  visible: boolean;
  /** Called when user closes the mirror. */
  onClose: () => void;
}

/** Fetch mirror snapshot from Tauri backend. Returns null on any failure. */
async function fetchSnapshot(): Promise<MirrorSnapshot | null> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<MirrorSnapshot>('get_mirror_snapshot');
  } catch {
    return null;
  }
}

/** Single indicator bar — colored by side and trend. */
function IndicatorBar({ indicator }: { indicator: MirrorIndicator }) {
  const barColor =
    indicator.side === 'planetary'
      ? indicator.exceeded
        ? 'var(--aurora-danger, #dc3545)'
        : 'var(--aurora-warn, #ffc107)'
      : 'var(--aurora-accent, #0dcaf0)';

  const trendIcon = indicator.trend === 'up' ? '↑' : indicator.trend === 'down' ? '↓' : '→';

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 4,
        fontSize: 11,
        fontFamily: 'monospace',
        color: 'var(--aurora-text, #e0e0e0)',
      }}
    >
      <span
        style={{
          display: 'inline-block',
          width: 6,
          height: 6,
          borderRadius: '50%',
          backgroundColor: barColor,
          flexShrink: 0,
        }}
      />
      <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {indicator.label}
      </span>
      <span style={{ fontWeight: 600 }}>
        {indicator.value}
        {indicator.unit}
      </span>
      <span style={{ color: barColor, width: 12, textAlign: 'center' }}>{trendIcon}</span>
    </div>
  );
}

export default function MirrorOverlay({ visible, onClose }: MirrorOverlayProps) {
  const [snapshot, setSnapshot] = useState<MirrorSnapshot | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const poll = useCallback(async () => {
    const data = await fetchSnapshot();
    if (data) setSnapshot(data);
  }, []);

  useEffect(() => {
    if (!visible) {
      if (intervalRef.current) clearInterval(intervalRef.current);
      return;
    }
    poll();
    intervalRef.current = setInterval(poll, POLL_INTERVAL_MS);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [visible, poll]);

  if (!visible) return null;

  return (
    <div
      style={{
        position: 'absolute',
        bottom: 16,
        right: 16,
        width: 280,
        maxHeight: '70vh',
        overflowY: 'auto',
        background: 'rgba(0, 0, 0, 0.75)',
        backdropFilter: 'blur(8px)',
        borderRadius: 8,
        padding: 12,
        border: '1px solid rgba(255,255,255,0.1)',
        zIndex: 100,
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 8,
        }}
      >
        <span style={{ fontSize: 12, fontWeight: 700, color: 'var(--aurora-text, #e0e0e0)' }}>
          停滞镜
        </span>
        <button
          onClick={onClose}
          style={{
            background: 'none',
            border: 'none',
            color: 'var(--aurora-text, #e0e0e0)',
            cursor: 'pointer',
            fontSize: 14,
            padding: 0,
          }}
          aria-label="关闭停滞镜"
        >
          ✕
        </button>
      </div>

      {!snapshot ? (
        <div style={{ color: 'var(--aurora-muted, #888)', fontSize: 11 }}>加载中...</div>
      ) : (
        <>
          {/* Stagnation warning banner */}
          {snapshot.stagnating && (
            <div
              style={{
                background: 'rgba(220, 53, 69, 0.2)',
                border: '1px solid var(--aurora-danger, #dc3545)',
                borderRadius: 4,
                padding: '6px 8px',
                marginBottom: 8,
                fontSize: 10,
                color: 'var(--aurora-danger, #dc3545)',
              }}
            >
              停滞检测：你的决策相位在过去 {snapshot.trajectory_runs ?? '?'} 轮中未发生有意义的变化。
              参考系可能已收窄。
            </div>
          )}

          {/* Human activity */}
          <div style={{ marginBottom: 10 }}>
            <div
              style={{
                fontSize: 10,
                fontWeight: 600,
                color: 'var(--aurora-accent, #0dcaf0)',
                marginBottom: 4,
                textTransform: 'uppercase',
                letterSpacing: 1,
              }}
            >
              人类活动
            </div>
            {snapshot.human_activity.map((indicator, i) => (
              <IndicatorBar key={i} indicator={indicator} />
            ))}
          </div>

          {/* Divider */}
          <div
            style={{
              height: 1,
              background: 'rgba(255,255,255,0.15)',
              margin: '8px 0',
            }}
          />

          {/* Planetary boundaries */}
          <div>
            <div
              style={{
                fontSize: 10,
                fontWeight: 600,
                color: 'var(--aurora-danger, #dc3545)',
                marginBottom: 4,
                textTransform: 'uppercase',
                letterSpacing: 1,
              }}
            >
              地球边界
            </div>
            {snapshot.planetary_boundaries.map((indicator, i) => (
              <IndicatorBar key={i} indicator={indicator} />
            ))}

            {/* Scissors gap summary */}
            <div
              style={{
                marginTop: 8,
                padding: '6px 8px',
                background: 'rgba(255,255,255,0.05)',
                borderRadius: 4,
                fontSize: 10,
                color: 'var(--aurora-muted, #888)',
                display: 'flex',
                justifyContent: 'space-between',
              }}
            >
              <span>
                边界突破：{snapshot.planetary_boundaries.filter(i => i.exceeded).length}/
                {snapshot.planetary_boundaries.length}
              </span>
              <span style={{ color: 'var(--aurora-danger, #dc3545)' }}>
                {snapshot.planetary_boundaries.filter(i => i.exceeded).length >= 4
                  ? '⚠ 多重临界'
                  : snapshot.planetary_boundaries.filter(i => i.exceeded).length >= 2
                  ? '● 压力'
                  : '○ 稳定'}
              </span>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
