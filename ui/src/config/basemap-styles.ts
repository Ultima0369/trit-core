// ui/src/config/basemap-styles.ts — maplibre style 构建（lazy）
//
// 借鉴 worldmonitor/src/config/basemap-styles.ts：所有 maplibre/pmtiles/
// protomaps 的动态 import 集中在此，且只在 MapPanel 挂载时才加载，
// 因此这些重依赖不进入主 bundle。registerPMTilesProtocol 单例化，
// 避免重复注册 protocol。
//
// 与 worldmonitor 的差异：主源是本地 RESOURCE_SERVER，fallback 链更短
// （local PMTiles → OpenFreeMap 远程）。
import {
  resolveResourceServer,
  FALLBACK_DARK_STYLE,
  FALLBACK_LIGHT_STYLE,
  type PMTilesTheme,
} from './basemap';

type StyleSpec = Record<string, unknown>;

let registered = false;
let registerPromise: Promise<void> | null = null;

/** 注册 pmtiles:// 协议到 maplibre（幂等，单例）。 */
export async function registerPMTilesProtocol(): Promise<void> {
  if (registered) return;
  registerPromise ??= (async () => {
    try {
      const { Protocol } = await import('pmtiles');
      if (registered) return;
      const maplibregl = (await import('maplibre-gl')).default;
      const protocol = new Protocol();
      maplibregl.addProtocol('pmtiles', protocol.tile);
      registered = true;
    } catch (err) {
      registerPromise = null; // 失败可重试
      throw err;
    }
  })();
  await registerPromise;
}

/** 用本地 PMTiles + protomaps basemaps 构建 vector style。 */
async function buildLocalStyle(
  flavor: PMTilesTheme,
  resourceServer: string,
): Promise<StyleSpec> {
  const { layers, namedFlavor } = await import('@protomaps/basemaps');
  const spriteName = flavor === 'light' ? 'light' : 'dark';
  return {
    version: 8,
    glyphs: `${resourceServer}/fonts/{fontstack}/{range}.pbf`,
    sprite: `${resourceServer}/sprites/v4/${spriteName}`,
    sources: {
      basemap: {
        type: 'vector',
        url: `pmtiles://${resourceServer}/pmtiles/basemap.pmtiles`,
        maxzoom: 15,
      },
    },
    layers: layers('basemap', namedFlavor(flavor), { lang: 'en' }),
  };
}

/**
 * 解析当前底图 style。优先本地 PMTiles；本地构建/注册失败时回落到
 * OpenFreeMap 远程 style（仍可用，但失去离线能力）。
 */
export async function resolveBasemapStyle(
  flavor: PMTilesTheme,
): Promise<{ style: StyleSpec | string; source: 'local' | 'fallback' }> {
  try {
    await registerPMTilesProtocol();
    const resourceServer = await resolveResourceServer();
    return { style: await buildLocalStyle(flavor, resourceServer), source: 'local' };
  } catch (err) {
    // ponytail: 本地服务挂了不能让地图整面板空白——回落远程 style。
    console.warn('[basemap] local PMTiles unavailable, falling back:', (err as Error)?.message);
    return {
      style: flavor === 'light' ? FALLBACK_LIGHT_STYLE : FALLBACK_DARK_STYLE,
      source: 'fallback',
    };
  }
}
