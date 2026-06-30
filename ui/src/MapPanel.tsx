// ui/src/MapPanel.tsx — 2D 矢量地图面板 (MapLibre + PMTiles)
//
// 借鉴 worldmonitor：底图 style 构建下沉到 config/basemap-styles.ts（lazy），
// 图层声明集中到 config/layer-definitions.ts。本组件做：挂载地图 + 应用
// 底图 style + 图层选择器 + 叠加已启用图层的真实监测站点 + 点站点弹
// popup 展示 anchor 传感器读数 vs 阈值。
// maplibre/pmtiles/protomaps 动态 import，不进主 bundle。
import { useEffect, useRef, useState } from 'react';
import diag, { isTauriEnvironment } from './utils/diag';
import { resolveBasemapStyle } from './config/basemap-styles';
import type { PMTilesTheme } from './config/basemap';
import {
  getExecutableLayers,
  type MapLayers,
  type LayerKey,
} from './config/layer-definitions';
import type { AnchorStatus, GeoEvent, MapCoord, MonitoringStation, SensorReading } from './types';

interface Props {
  flavor?: PMTilesTheme;
  /** 启用的图层（受控，由 App/Sidebar 持有）。 */
  layers: MapLayers;
  /** 视角/鼠标坐标变化回调 → 传给 App → StatusBar。 */
  onCoordChange?: (coord: MapCoord) => void;
}

/** anchor 站点图层 key → station kind（geoEvents 不在此映射，走独立数据源）。 */
const LAYER_KEY_TO_KIND: Partial<Record<LayerKey, MonitoringStation['kind']>> = {
  thermalStations: 'thermal',
  ecologicalStations: 'ecological',
};

/** anchor 站点正常色（违例统一红，见 paint）。 */
const LAYER_SOURCE_COLORS: Record<string, string> = {
  thermalStations: '#ff6b6b',
  ecologicalStations: '#51cf66',
};

/** UCDP 暴力类型分色 — 借鉴 worldmonitor COLORS.ucdpStateBased/NonState/OneSided。 */
const VIOLENCE_TYPE_COLORS: Record<string, string> = {
  'state-based': '#ff3232', // 红
  'non-state': '#ffa500', // 橙
  'one-sided': '#ffff00', // 黄
};

/** 拉取监测站坐标。非 Tauri 环境返回空（开发态无后端）。 */
async function fetchStations(): Promise<MonitoringStation[]> {
  if (!isTauriEnvironment()) return [];
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<MonitoringStation[]>('get_monitoring_stations');
}

/** 拉取 anchor 状态快照。非 Tauri 环境返回空。 */
async function fetchAnchorStatus(): Promise<AnchorStatus[]> {
  if (!isTauriEnvironment()) return [];
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AnchorStatus[]>('get_anchor_status', { degraded: false });
}

/** 拉取地缘冲突事件（UCDP）。非 Tauri 环境返回空。 */
async function fetchGeoEvents(): Promise<GeoEvent[]> {
  if (!isTauriEnvironment()) return [];
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GeoEvent[]>('get_geo_events');
}

/** 渲染单个传感器读数为 HTML 片段。 */
function renderReading(r: SensorReading): string {
  const val = Number.isNaN(r.value) ? 'N/A' : r.value.toFixed(2);
  const mark = r.violated ? '<span style="color:#ff6b6b">✗</span>' : '<span style="color:#51cf66">✓</span>';
  return `<div style="display:flex;justify-content:space-between;gap:12px">
    <span>${r.name} ${mark}</span>
    <span style="opacity:0.85">${val} / ${r.threshold} ${r.unit}</span>
  </div>`;
}

export default function MapPanel({ flavor = 'light', layers, onCoordChange }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<any>(null);
  // 跨 effect 共享的数据缓存——挂载时拉一次，图层开关只读不重拉。
  const dataRef = useRef<{ stations: MonitoringStation[]; anchorStatus: AnchorStatus[]; geoEvents: GeoEvent[] }>({ stations: [], anchorStatus: [], geoEvents: [] });
  const activeLayers = layers;
  const coordRef = useRef(onCoordChange);
  coordRef.current = onCoordChange;
  const [dataReady, setDataReady] = useState(false);

  /// 缩放级别 → 米/像素（标准 Web Mercator 比例尺公式）。
  const metersPerPixel = (zoom: number, lat: number): number =>
    (40075016.7 * Math.cos((lat * Math.PI) / 180)) / Math.pow(2, zoom + 8);

  // 挂载地图 + 底图 style（仅 flavor 变化时重建）。
  useEffect(() => {
    let map: any = null;
    let cancelled = false;

    (async () => {
      try {
        const { style, source } = await resolveBasemapStyle(flavor);
        if (cancelled || !containerRef.current) return;
        const maplibregl = (await import('maplibre-gl')).default;
        map = new maplibregl.Map({
          container: containerRef.current,
          style: style as any,
          center: [116, 36],
          zoom: 3,
        });
        mapRef.current = map;
        diag('MapPanel', 'INFO', `2D 地图已加载 (底图来源: ${source})`);

        // 坐标状态栏：鼠标移动 + 缩放变化时上报坐标。
        const emit = (lng: number | null, lat: number | null, zoom: number | null) => {
          coordRef.current?.({
            lng,
            lat,
            zoom,
            scale: lng != null && lat != null && zoom != null ? metersPerPixel(zoom, lat) : null,
          });
        };
        map.on('mousemove', (e: any) => emit(e.lngLat.lng, e.lngLat.lat, map.getZoom()));
        map.on('zoom', () => emit(null, null, map.getZoom()));
        map.on('moveend', () => emit(null, null, map.getZoom()));
      } catch (e) {
        diag('MapPanel', 'WARN', `2D 地图加载失败: ${e}`);
      }
    })();

    return () => {
      cancelled = true;
      map?.remove();
      mapRef.current = null;
    };
  }, [flavor]);

  // 图层叠加 + 站点 popup：activeLayers 变化时重算 features。
  // 借鉴 worldmonitor 效果模式：分色 + 半径按严重度缩放 + 半透明区域填充。
  // ponytail: 只读 dataRef 缓存，不重新 fetch —— 开关图层不应触发网络请求。
  // 数据由下面的 [] effect 挂载时拉取一次。
  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;

    const { stations, anchorStatus, geoEvents } = dataRef.current;
    let applied = false;

    const apply = () => {
      if (applied) return;
      applied = true;
      for (const def of getExecutableLayers('flat')) {
        const srcId = `layer-${def.key}`;
        const enabled = activeLayers[def.key];

        // ── 构造该图层的 features ──
        let features: any[] = [];
        if (enabled) {
          if (def.key === 'geoEvents') {
            // UCDP 事件：deaths + violence_type 驱动半径/分色。
            features = geoEvents.map(e => ({
              type: 'Feature',
              geometry: { type: 'Point', coordinates: [e.lng, e.lat] },
              properties: {
                violence_type: e.violence_type,
                deaths: e.deaths,
                country: e.country,
                date: e.date,
              },
            }));
          } else {
            // anchor 站点：violated + deviation 驱动半径/分色。
            const kind = LAYER_KEY_TO_KIND[def.key];
            const status = anchorStatus.find(a => a.kind === kind);
            const violated = status?.has_violations ?? false;
            // 偏离程度：取该 anchor 最大读数偏离阈值的比例，用于缩放半径。
            const deviation = status
              ? Math.max(...status.readings.map(r => {
                  if (Number.isNaN(r.value) || r.threshold === 0) return 1;
                  return Math.abs(r.value - r.threshold) / Math.abs(r.threshold);
                }))
              : 0;
            features = stations
              .filter(s => s.kind === kind)
              .map(s => ({
                type: 'Feature',
                geometry: { type: 'Point', coordinates: [s.lng, s.lat] },
                properties: { name: s.name, kind: s.kind, violated, deviation },
              }));
          }
        }

        const fc = { type: 'FeatureCollection', features };

        // geoEvents 额外构造国家聚合数据（借鉴 worldmonitor buildConflictZoneGeoJson
        // 按国家聚合的思路）。不依赖国家边界几何——用事件质心 + 缓冲圆近似
        // "冲突区域"，半径按该国总死伤缩放。
        let countryAgg: any[] = [];
        if (def.key === 'geoEvents' && enabled) {
          const byCountry = new Map<string, { lat: number; lng: number; deaths: number; n: number; type: string }>();
          for (const e of geoEvents) {
            if (!e.country) continue;
            const c = byCountry.get(e.country) ?? { lat: 0, lng: 0, deaths: 0, n: 0, type: e.violence_type };
            c.lat += e.lat; c.lng += e.lng; c.deaths += e.deaths; c.n += 1;
            // 主导暴力类型：state-based 优先（最严重）。
            if (e.violence_type === 'state-based') c.type = 'state-based';
            byCountry.set(e.country, c);
          }
          countryAgg = [...byCountry.entries()].map(([country, c]) => ({
            type: 'Feature',
            geometry: { type: 'Point', coordinates: [c.lng / c.n, c.lat / c.n] },
            properties: { country, deaths: c.deaths, violence_type: c.type, count: c.n },
          }));
        }

        if (map.getSource(srcId)) {
          map.getSource(srcId).setData(fc);
          if (def.key === 'geoEvents') {
            const aggId = `${srcId}-country`;
            if (map.getSource(aggId)) {
              map.getSource(aggId).setData({ type: 'FeatureCollection', features: countryAgg });
            }
          }
        } else {
          map.addSource(srcId, { type: 'geojson', data: fc });

          // geoEvents 国家冲突区域 fill（在单事件 fill 下方，最大半径低透明度）。
          if (def.key === 'geoEvents') {
            const aggId = `${srcId}-country`;
            map.addSource(aggId, { type: 'geojson', data: { type: 'FeatureCollection', features: countryAgg } });
            map.addLayer({
              id: aggId,
              type: 'circle',
              source: aggId,
              paint: {
                // 国家级区域：半径按总死伤放大（比单事件更大），体现整体冲突强度。
                'circle-radius': ['interpolate', ['linear'], ['sqrt', ['get', 'deaths']], 0, 30, 100, 80, 1000, 160],
                'circle-color': ['match', ['get', 'violence_type'],
                  'state-based', '#ff3232',
                  'non-state', '#ffa500',
                  'one-sided', '#ffff00',
                  '#ff3232'],
                'circle-opacity': 0.12,
                'circle-stroke-width': 1,
                'circle-stroke-color': ['match', ['get', 'violence_type'],
                  'state-based', '#ff3232',
                  'non-state', '#ffa500',
                  'one-sided', '#ffff00',
                  '#ff3232'],
                'circle-stroke-opacity': 0.3,
              },
            });
          }

          // 半透明区域填充层（在主 circle 下方，借鉴 worldmonitor 冲突区 fill）。
          // anchor 违例站点 / 高死伤事件 → 大半径低透明度圆，体现"影响区域"。
          if (def.key === 'geoEvents') {
            map.addLayer({
              id: `${srcId}-fill`,
              type: 'circle',
              source: srcId,
              paint: {
                // 借鉴 worldmonitor: getRadius = sqrt(deaths)*scale，区域层放大 3x。
                'circle-radius': ['interpolate', ['linear'], ['sqrt', ['get', 'deaths']], 0, 8, 50, 40, 500, 80],
                'circle-color': ['match', ['get', 'violence_type'],
                  'state-based', '#ff3232',
                  'non-state', '#ffa500',
                  'one-sided', '#ffff00',
                  '#ff3232'],
                'circle-opacity': 0.18,
                'circle-stroke-width': 0,
              },
            });
          } else {
            map.addLayer({
              id: `${srcId}-fill`,
              type: 'circle',
              source: srcId,
              paint: {
                // 违例站点区域填充：半径按偏离程度放大，正常站点不显示填充。
                'circle-radius': ['interpolate', ['linear'], ['get', 'deviation'], 0, 0, 0.5, 30, 2, 80],
                'circle-color': LAYER_SOURCE_COLORS[def.key],
                'circle-opacity': ['case', ['get', 'violated'], 0.2, 0],
                'circle-stroke-width': 0,
              },
            });
          }

          // 主 circle 层（点位 + 边框）。
          map.addLayer({
            id: srcId,
            type: 'circle',
            source: srcId,
            paint: def.key === 'geoEvents'
              ? {
                  // 借鉴 worldmonitor ScatterplotLayer: radiusMinPixels 3 / MaxPixels 20
                  // + sqrt(deaths)*scale。MapLibre interpolate 等价。
                  'circle-radius': ['interpolate', ['linear'], ['sqrt', ['get', 'deaths']], 0, 4, 50, 10, 500, 20],
                  'circle-color': ['match', ['get', 'violence_type'],
                    'state-based', '#ff3232',
                    'non-state', '#ffa500',
                    'one-sided', '#ffff00',
                    '#ff3232'],
                  'circle-stroke-width': 1,
                  'circle-stroke-color': '#fff',
                  'circle-stroke-opacity': 0.8,
                }
              : {
                  // anchor 站点：违例红 / 正常本色，半径按偏离程度微调。
                  'circle-radius': ['interpolate', ['linear'], ['get', 'deviation'], 0, 6, 1, 10, 2, 14],
                  'circle-color': ['case', ['get', 'violated'], '#ff3232', LAYER_SOURCE_COLORS[def.key]],
                  'circle-stroke-width': 1.5,
                  'circle-stroke-color': '#fff',
                },
          });

          // 点站点/事件 → popup。
          map.on('click', srcId, async (e: any) => {
            const feat = e.features?.[0];
            if (!feat) return;
            const p = feat.properties || {};
            const { default: maplibregl } = await import('maplibre-gl');
            let html: string;
            if (def.key === 'geoEvents') {
              const color = VIOLENCE_TYPE_COLORS[p.violence_type] ?? '#fff';
              html = `<div style="font:12px/1.5 system-ui;color:#fff;min-width:180px">
                <div style="font-weight:600;margin-bottom:4px">${p.country || '未知'}</div>
                <div><span style="color:${color}">●</span> ${p.violence_type}</div>
                <div style="opacity:0.85">死伤估计: ${p.deaths} · ${p.date}</div>
              </div>`;
            } else {
              const status = anchorStatus.find(a => a.kind === p.kind);
              const readingsHtml = status
                ? status.readings.map(renderReading).join('')
                : '<div style="opacity:0.6">无 anchor 数据</div>';
              const header = status?.has_violations
                ? '<span style="color:#ff3232">● 违例</span>'
                : '<span style="color:#51cf66">● 正常</span>';
              html = `<div style="font:12px/1.5 system-ui;color:#fff;min-width:200px">
                <div style="font-weight:600;margin-bottom:4px">${p.name} ${header}</div>
                ${readingsHtml}
              </div>`;
            }
            new maplibregl.Popup({ maxWidth: '320px' })
              .setLngLat(e.lngLat)
              .setHTML(html)
              .addTo(map);
          });
          map.on('mouseenter', srcId, () => (map.getCanvas().style.cursor = 'pointer'));
          map.on('mouseleave', srcId, () => (map.getCanvas().style.cursor = ''));
        }
      }
      diag('MapPanel', 'INFO', `图层已重算: ${Object.entries(activeLayers).filter(([,v]) => v).map(([k]) => k).join(',') || '无'}`);
    };

    if (map.loaded?.() ?? false) apply();
    else map.once?.('load', apply);
  }, [activeLayers, dataReady]);

  // 数据拉取：挂载时拉一次，存 dataRef + 置 dataReady 触发上图层重算。
  useEffect(() => {
    let cancelled = false;
    Promise.all([fetchStations(), fetchAnchorStatus(), fetchGeoEvents()])
      .then(([s, a, g]) => {
        if (cancelled) return;
        dataRef.current = { stations: s, anchorStatus: a, geoEvents: g };
        setDataReady(true);
      })
      .catch(e => diag('MapPanel', 'WARN', `数据拉取失败: ${e}`));
    return () => { cancelled = true; };
  }, []);

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%' }}>
      <div ref={containerRef} style={{ width: '100%', height: '100%' }} />
    </div>
  );
}
