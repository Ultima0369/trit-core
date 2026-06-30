// ui/src/config/layer-definitions.ts — 地图图层注册表（骨架）
//
// 借鉴 worldmonitor/src/config/map-layer-definitions.ts 的 LayerDefinition
// 结构：每个图层是一个声明式条目（key / icon / label / renderers /
// source 描述），渲染与否由声明 + isLayerExecutable gate 决定，而非
// 散落在各组件的 if 分支里。
//
// 与 worldmonitor 的差异：trit-core 是认知决策引擎，不是情报监控面板。
// 不搬它的 70 个 conflict/shipping/cyber 图层，也不搬 variant / synonym
// / explanation 子系统（YAGNI——没有消费者）。先放 2 个真实图层，对应
// trit-core Layer 1 anchor 模块的生态/热监测站点，证明模式可工作。

/** 图层渲染器（trit-core 目前只有 2D flat，globe 留作扩展位）。 */
export type MapRenderer = 'flat' | 'globe';

/** 声明式图层定义。key 同时是 MapLayers 的字段名。 */
export interface LayerDefinition {
  key: LayerKey;
  label: string;
  /** 该图层支持哪些渲染器；不包含当前渲染器 → gate 拒绝渲染。 */
  renderers: MapRenderer[];
  /** 数据来源描述（人类可读，供将来图层说明卡使用）。 */
  source: string;
}

/** 当前全部图层 key。新增图层在此 union 与 LAYER_REGISTRY 同步登记。 */
export type LayerKey = 'thermalStations' | 'ecologicalStations' | 'geoEvents';

/** 图层开关状态：key → 是否启用。 */
export type MapLayers = Record<LayerKey, boolean>;

export const DEFAULT_LAYERS: MapLayers = {
  thermalStations: false,
  ecologicalStations: false,
  geoEvents: false,
};

const def = (
  key: LayerKey,
  label: string,
  renderers: MapRenderer[],
  source: string,
): LayerDefinition => ({ key, label, renderers, source });

/**
 * 图层注册表。借鉴 worldmonitor 的 LAYER_REGISTRY：单一事实来源，
 * 图层选择器 / 渲染器 / CMD+K 派发都从这里读。
 *
 * 图标不在声明里——由 Sidebar 的 LAYER_ICONS 映射渲染（lucide-react），
 * 避免纯 config 文件依赖 React。
 *
 * 当前 2 个图层对应 trit-core Layer 1 anchor（src/anchor/）：
 * - thermalStations  ← thermal_baseline.rs（OLR / CO2 / 能量失衡监测）
 * - ecologicalStations ← ecological_base.rs（BII / 碳汇 / 海洋 pH）
 */
export const LAYER_REGISTRY: Record<LayerKey, LayerDefinition> = {
  thermalStations: def(
    'thermalStations',
    '热基线监测站',
    ['flat'],
    'Earth outgoing longwave radiation / CO2 / energy imbalance monitoring (src/anchor/thermal_baseline.rs)',
  ),
  ecologicalStations: def(
    'ecologicalStations',
    '生态基线监测站',
    ['flat'],
    'Biodiversity Intactness / carbon sink / ocean pH monitoring (src/anchor/ecological_base.rs)',
  ),
  geoEvents: def(
    'geoEvents',
    '地缘冲突事件',
    ['flat'],
    'UCDP GED armed conflict events — state-based / non-state / one-sided (https://ucdpapi.pcr.uu.se)',
  ),
};

/**
 * 图层能否在当前渲染器下真正渲染。借鉴 worldmonitor 的 isLayerExecutable：
 * 图层选择器和渲染器都应先调它，避免 no-op toggle。
 *
 * 规则：图层声明的 renderers 必须包含 currentRenderer。
 * （trit-core 暂无 deckGLOnly 概念——只有一个 flat 渲染器。）
 */
export function isLayerExecutable(
  layerKey: LayerKey,
  currentRenderer: MapRenderer,
): boolean {
  const d = LAYER_REGISTRY[layerKey];
  return !!d && d.renderers.includes(currentRenderer);
}

/** 当前渲染器下可执行的图层列表（已按注册顺序）。 */
export function getExecutableLayers(currentRenderer: MapRenderer): LayerDefinition[] {
  return (Object.keys(LAYER_REGISTRY) as LayerKey[])
    .map(k => LAYER_REGISTRY[k])
    .filter(d => d.renderers.includes(currentRenderer));
}
