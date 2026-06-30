// ui/src/config/basemap.ts — 底图 provider / 主题配置（pure config）
//
// 借鉴 worldmonitor/src/config/basemap.ts 的拆分：本文件不引入任何
// maplibre/pmtiles/protomaps 运行时依赖，只持有 provider/theme 常量与
// 类型。maplibre 相关的 style 构建逻辑放在 ./basemap-styles.ts，随
// MapPanel 懒加载，不进主 bundle。
//
// 与 worldmonitor 的差异：trit-core 是 local-first 桌面工具，底图主源是
// 本地资源服务器（src-tauri proxy_server），不依赖远程 R2。CARTO /
// OpenFreeMap 仅作为本地服务不可用时的可选 fallback。

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// 本地资源服务器：Tauri 环境下由 src-tauri proxy_server 提供 PMTiles /
// 字体 / sprite。ponytail: 端口暂固定 21337；需要多实例/远程源时再改为
// env / settings 可配。
const FALLBACK_RESOURCE_SERVER = 'http://localhost:21337';

/**
 * 解析本地资源服务器地址。Tauri 环境向 Rust 取 get_resource_server_url
 * （与 proxy_server 实际监听地址一致，避免端口漂移时前端写死）；
 * 非 Tauri 环境（开发/测试）回落硬编码。
 */
export async function resolveResourceServer(): Promise<string> {
  if (!isTauri) return FALLBACK_RESOURCE_SERVER;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const url = await invoke<string>('get_resource_server_url');
    return url || FALLBACK_RESOURCE_SERVER;
  } catch {
    return FALLBACK_RESOURCE_SERVER;
  }
}

export type PMTilesTheme = 'light' | 'dark';

export type MapProvider = 'local' | 'openfreemap' | 'carto';

export const HAS_LOCAL_TILES = isTauri;

// OpenFreeMap 作为本地服务不可用时的远程 fallback（positron=light, dark=dark）。
export const FALLBACK_LIGHT_STYLE = 'https://tiles.openfreemap.org/styles/positron';
export const FALLBACK_DARK_STYLE = 'https://tiles.openfreemap.org/styles/dark';

export function isLightTheme(theme: PMTilesTheme): boolean {
  return theme === 'light';
}
