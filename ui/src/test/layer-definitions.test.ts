import { describe, it, expect } from 'vitest';
import {
  LAYER_REGISTRY,
  DEFAULT_LAYERS,
  isLayerExecutable,
  getExecutableLayers,
  type MapRenderer,
} from '../config/layer-definitions';

// ponytail: 图层 gate 是非平凡逻辑（声明驱动渲染与否），留一个 runnable
// check 证明 gate 真的拒绝错误的渲染器，而不是总返回 true。
describe('layer-definitions', () => {
  const flat: MapRenderer = 'flat';
  const globe: MapRenderer = 'globe';

  it('每个注册图层都声明了 renderers', () => {
    for (const key of Object.keys(LAYER_REGISTRY) as Array<keyof typeof LAYER_REGISTRY>) {
      expect(LAYER_REGISTRY[key].renderers.length).toBeGreaterThan(0);
    }
  });

  it('isLayerExecutable 在 flat 下放行 thermalStations（声明含 flat）', () => {
    expect(isLayerExecutable('thermalStations', flat)).toBe(true);
  });

  it('isLayerExecutable 在 globe 下拒绝 thermalStations（声明仅 flat）', () => {
    expect(isLayerExecutable('thermalStations', globe)).toBe(false);
  });

  it('getExecutableLayers(globe) 返回空（当前所有图层仅 flat）', () => {
    expect(getExecutableLayers(globe)).toHaveLength(0);
  });

  it('DEFAULT_LAYERS 全部关闭', () => {
    for (const v of Object.values(DEFAULT_LAYERS)) {
      expect(v).toBe(false);
    }
  });
});
