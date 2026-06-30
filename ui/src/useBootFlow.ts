// ui/src/useBootFlow.ts — 启动过场状态机
//
// 收口 App 里的过场编排：cesiumReady/earthEntering/bootDone 三态 + 60s 兜底。
// App 只消费结果，不再持有状态机细节。

import { useCallback, useEffect, useState } from 'react';
import diag from './utils/diag';

const FALLBACK_TIMEOUT_MS = 60000; // Boot 按钮也没出现时的最终兜底

export interface BootFlowState {
  /** 地球引擎就绪（Cesium/globe.gl onReady 触发）。 */
  cesiumReady: boolean;
  /** Earth.onReady 回调——传给 Earth 组件。 */
  setEarthReady: () => void;
  /** 地球登场过渡进行中——驱动地球容器 class + BootScreen fadeOut。 */
  earthEntering: boolean;
  /** 过场完成，BootScreen 已卸载。 */
  bootDone: boolean;
  /** BootScreen onTransitionReady——进度条满+用户点按钮时触发地球登场。 */
  onTransitionReady: () => void;
  /** BootScreen onFadeComplete——淡出结束，卸载 BootScreen。 */
  onFadeComplete: () => void;
}

/**
 * 启动过场状态机。
 * 返回 cesiumReady 供 App 构造 BootScreen 的 externalMilestones。
 */
export function useBootFlow(): BootFlowState {
  const [cesiumReady, setCesiumReady] = useState(false);
  const [earthEntering, setEarthEntering] = useState(false);
  const [bootDone, setBootDone] = useState(false);

  const setEarthReady = useCallback(() => setCesiumReady(true), []);
  const onTransitionReady = useCallback(() => {
    diag('Boot', 'INFO', '进度条满+用户点击，地球登场');
    setEarthEntering(true);
  }, []);
  const onFadeComplete = useCallback(() => setBootDone(true), []);

  // 60s 最终兜底：Boot 按钮未出现（组件异常）时强制进入，不卡死。
  useEffect(() => {
    if (earthEntering || bootDone) return;
    const t = setTimeout(() => {
      diag('Boot', 'WARN', '启动最终超时兜底，强制进入界面');
      setEarthEntering(true);
    }, FALLBACK_TIMEOUT_MS);
    return () => clearTimeout(t);
  }, [earthEntering, bootDone]);

  return { cesiumReady, setEarthReady, earthEntering, bootDone, onTransitionReady, onFadeComplete };
}
