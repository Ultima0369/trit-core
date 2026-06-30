// ui/src/Earth.tsx — 3D globe (the 长见识 geographic anchor for the HUD)
//
// CesiumJS primary, react-globe.gl fallback. Fixed product initialization
// (no Sandcastle code execution). Cosmos preset adapted from World Monitor.
// Drag-to-pause + auto-resume rotation.

import { useCallback, useEffect, useRef, useState } from 'react';
import Globe, { GlobeMethods } from 'react-globe.gl';
import * as THREE from 'three';
import diag, { isTauriEnvironment } from './utils/diag';
import type { GlobeTexture, MapCoord } from './types';
import { EVENTS, CESIUM_CONTAINER_ID } from './types';

/// 拉取资源状态报告。收口 4 处重复的 dynamic-import + invoke('get_asset_status')。
async function fetchAssetStatus(): Promise<any> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<any>('get_asset_status');
}

const DEFAULT_ROTATION_SPEED_DEG_PER_SEC = 6;
const RESOURCE_SERVER = 'http://localhost:21337';
const CESIUM_BASE_URL = `${RESOURCE_SERVER}/cesium`;
const CESIUM_INIT_TIMEOUT_MS = 15000;

const TEXTURE_URLS: Record<GlobeTexture, string> = {
  'blue-marble': 'earth-blue-marble.jpg',
  'topographic': 'earth-topo-bathy.jpg',
};

const FALLBACK_TEXTURES = {
  globe: 'earth-blue-marble.jpg',
  bump: 'earth-topology.png',
  background: 'night-sky.png',
};

type Engine = 'cesium' | 'globe-gl' | 'loading' | 'error';

interface Props {
  resumeDelayMs: number;
  rotationSpeed?: number;
  /** 视角/鼠标坐标变化回调 → App → StatusBar。 */
  onViewChange?: (coord: MapCoord) => void;
  /** 引擎就绪回调 → App → BootScreen 里程碑。Cesium/globe.gl 任一就绪即触发一次。 */
  onReady?: () => void;
}

// 从 get_asset_status 报告中按需取纹理的代理 URL。
// 仅当三张纹理文件都已就绪（cached/ok）时返回，否则 null。
// 故意不看 all_ready——它包含中国瓦片（~575MB 后台下载），会误判纹理缺失。
// 导出以便单测，无需启动整套异步启动流。
export function buildTextureUrls(report: any): { globe: string; bump: string; background: string } | null {
  const ready = new Set(
    (report?.assets ?? [])
      .filter((a: any) => a.status === 'cached' || a.status === 'ok')
      .map((a: any) => a.name)
  );
  const need = ['earth-blue-marble.jpg', 'earth-topology.png', 'night-sky.png'];
  if (!need.every(n => ready.has(n))) return null;
  const url = (name: string) => `${RESOURCE_SERVER}/assets/${name}`;
  return {
    globe: url('earth-blue-marble.jpg'),
    bump: url('earth-topology.png'),
    background: url('night-sky.png'),
  };
}

export default function Earth({ resumeDelayMs, rotationSpeed = DEFAULT_ROTATION_SPEED_DEG_PER_SEC, onViewChange, onReady }: Props) {
  const [engine, setEngine] = useState<Engine>('loading');
  const engineRef = useRef<Engine>('loading');
  const coordRef = useRef(onViewChange);
  coordRef.current = onViewChange;
  const readyRef = useRef(onReady);
  readyRef.current = onReady;
  const readyFiredRef = useRef(false);
  const globeRef = useRef<GlobeMethods | undefined>(undefined);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewerRef = useRef<any>(null);
  const [ready, setReady] = useState(false);
  const [textures, setTextures] = useState(FALLBACK_TEXTURES);
  const rotatingRef = useRef(true);
  const resumeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [serverReady, setServerReady] = useState(false);
  const serverReadyRef = useRef(false);
  const [resetCounter, setResetCounter] = useState(0);

  // Tracked container size — drives react-globe.gl width/height and CesiumJS resize.
  // ponytail: window.innerWidth is a snapshot, not reactive; ResizeObserver is the
  // correct primitive so the canvas follows window/edge resizes without manual listeners.
  const [size, setSize] = useState({ w: 0, h: 0 });

  // Cosmos preset refs (globe.gl path only)
  const cosmosRef = useRef<{
    outerGlow: THREE.Mesh | null;
    innerGlow: THREE.Mesh | null;
    starField: THREE.Points | null;
    cyanLight: THREE.PointLight | null;
    savedMaterial: THREE.Material | null;
    animFrameId: number | null;
  }>({ outerGlow: null, innerGlow: null, starField: null, cyanLight: null, savedMaterial: null, animFrameId: null });

  // Texture switcher
  const [globeTexture, setGlobeTexture] = useState<GlobeTexture>('blue-marble');

  useEffect(() => {
    window.dispatchEvent(new CustomEvent(EVENTS.GLOBE_TEXTURE_CHANGED, {
      detail: { texture: globeTexture },
    }));
  }, [globeTexture]);

  // Listen for texture switch from TopBar
  useEffect(() => {
    const handler = (e: Event) => {
      const { texture } = (e as CustomEvent).detail as { texture: GlobeTexture };
      setGlobeTexture(texture);
      if (globeRef.current) {
        const url = isTauriEnvironment()
          ? textures.globe.replace(/earth-(blue-marble|topo-bathy)\.jpg$/, TEXTURE_URLS[texture])
          : TEXTURE_URLS[texture];
        (globeRef.current as any).globeImageUrl(url);
      }
    };
    window.addEventListener(EVENTS.SET_GLOBE_TEXTURE, handler);
    return () => window.removeEventListener(EVENTS.SET_GLOBE_TEXTURE, handler);
  }, [textures.globe]);

  useEffect(() => { engineRef.current = engine; }, [engine]);

  // Server readiness probe
  useEffect(() => {
    if (!isTauriEnvironment()) {
      setServerReady(true);
      serverReadyRef.current = true;
      return;
    }
    const check = setInterval(async () => {
      try {
        const res = await fetch(`${RESOURCE_SERVER}/health`);
        if (res.ok) {
          diag('Earth', 'INFO', '代理服务器就绪');
          setServerReady(true);
          serverReadyRef.current = true;
          clearInterval(check);
        }
      } catch { /* server not ready yet */ }
    }, 100);
    const timeout = setTimeout(() => {
      clearInterval(check);
      if (!serverReadyRef.current) {
        diag('Earth', 'WARN', '服务器超时，回退到 react-globe.gl');
        setEngine('globe-gl');
      }
    }, 10000);
    return () => { clearInterval(check); clearTimeout(timeout); };
  }, []);

  // Startup flow (depends on serverReady + resetCounter)
  useEffect(() => {
    if (!serverReady) return;
    if (!isTauriEnvironment()) {
      setEngine('globe-gl');
      return;
    }

    let destroyed = false;

    const startup = async () => {
      try {
        const report = await fetchAssetStatus();
        const cesiumReady = report.assets?.some(
          (a: any) => a.category === 'cesium' && (a.status === 'cached' || a.status === 'ok')
        );
        if (!cesiumReady) {
          diag('Earth', 'INFO', 'CesiumJS 未就绪，回退到 react-globe.gl');
          setEngine('globe-gl');
          return;
        }
      } catch (e) {
        diag('Earth', 'WARN', `检查资源失败: ${e}`);
        setEngine('globe-gl');
        return;
      }

      await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
      if (destroyed) return;

      diag('Earth', 'INFO', '尝试 CesiumJS 引擎...');
      // ponytail: local flag avoids engineRef sync race with useEffect
      let cesiumOk = false;
      try {
        await initCesiumJS(destroyed, () => {
          if (!destroyed) {
            cesiumOk = true;
            setEngine('cesium');
            setReady(true);
            fireReadyOnce();
          }
        });
      } catch (e) {
        diag('Earth', 'WARN', `CesiumJS 初始化失败: ${e}，回退到 react-globe.gl`);
        setEngine('globe-gl');
        return;
      }

      setTimeout(() => {
        if (destroyed) return;
        if (!cesiumOk) {
          diag('Earth', 'WARN', 'CesiumJS 渲染超时，回退到 react-globe.gl');
          if (viewerRef.current && !viewerRef.current.isDestroyed()) {
            viewerRef.current.destroy();
            viewerRef.current = null;
          }
          setEngine('globe-gl');
        }
      }, CESIUM_INIT_TIMEOUT_MS);
    };

    startup();
    return () => { destroyed = true; };
  }, [serverReady, resetCounter]);

  // Load textures for react-globe.gl.
  // ponytail: 不能用 check_cached_assets()——它只在 all_ready(含中国瓦片 575MB)
  // 为真时返回路径，而纹理是秒级下载的快速资源。中国瓦片还在后台下时纹理早就
  // 就绪了，却被 all_ready 误判为缺失，导致地球无贴图。改查 get_asset_status
  // 的逐项 status，并用代理 URL（CSP img-src 已放行 localhost:21337，
  // 且避免 file:/// 被 webview 拦截）。
  useEffect(() => {
    if (engine !== 'globe-gl' || !isTauriEnvironment()) return;

    let cancelled = false;
    (async () => {
      try {
        const report = await fetchAssetStatus();
        const urls = buildTextureUrls(report);
        if (cancelled) return;
        if (urls) {
          setTextures(urls);
          diag('Earth', 'INFO', '使用本地缓存纹理 (代理 URL)');
        } else {
          diag('Earth', 'WARN', '纹理未就绪，使用回退');
        }
      } catch (e) {
        diag('Earth', 'WARN', `纹理加载失败，使用回退: ${e}`);
      }
    })();
    return () => { cancelled = true; };
  }, [engine]);

  // CesiumJS init — fixed product code (OSM base + China satellite overlay + China camera)
  async function initCesiumJS(destroyed: boolean, onSuccess: () => void) {
    (window as any).CESIUM_BASE_URL = CESIUM_BASE_URL;

    await loadScript(`${CESIUM_BASE_URL}/Cesium.js`);
    if (destroyed || !(window as any).Cesium) throw new Error('Cesium.js 加载失败');

    await loadCSS(`${CESIUM_BASE_URL}/Widgets/Widgets.css`);

    const Cesium = (window as any).Cesium;
    Cesium.Ion.defaultAccessToken = undefined;

    if (!containerRef.current) throw new Error('容器不存在');

    diag('Earth', 'INFO', `containerRef 已就绪 (${containerRef.current.offsetWidth}x${containerRef.current.offsetHeight})，创建 CesiumJS Viewer...`);

    // 全球底图走代理 china-tiles 端点：select_sources 自动按区域选源
    // （中国→高德卫星，全球→Esri World Imagery）。z3-z6 后端预下载，z7+ 按需下载回退。
    const baseProvider = new Cesium.UrlTemplateImageryProvider({
      url: `${RESOURCE_SERVER}/china-tiles/{z}/{x}/{y}.jpg`,
      tilingScheme: new Cesium.WebMercatorTilingScheme(),
      minimumLevel: 3,
      maximumLevel: 18,
    });

    const viewer = new Cesium.Viewer(containerRef.current, {
      baseLayer: new Cesium.ImageryLayer(baseProvider),
      baseLayerPicker: false,
      geocoder: false,
      homeButton: false,
      sceneModePicker: false,
      navigationHelpButton: false,
      animation: false,
      timeline: false,
      fullscreenButton: false,
      vrButton: false,
      infoBox: false,
      selectionIndicator: false,
      creditContainer: document.createElement('div'),
      requestRenderMode: false,
      useBrowserRecommendedResolution: true,
      showRenderLoopErrors: false,
    });

    // 底图 baseProvider 已覆盖全球（走代理 china-tiles 端点，select_sources 统一选 Esri）。
    // 不再叠加高德中国层——避免高德卫星与全球 Esri 卫星的色调色差。

    viewer.scene.renderError.addEventListener((_scene: any, error: any) => {
      let errMsg = 'unknown';
      try {
        if (error instanceof Error) {
          errMsg = error.message + '\n' + error.stack;
        } else if (typeof error === 'object' && error !== null) {
          errMsg = JSON.stringify(error, Object.getOwnPropertyNames(error));
        } else {
          errMsg = String(error);
        }
      } catch {
        errMsg = String(error);
      }

      const isFatal =
        (errMsg.includes('context') && errMsg.includes('lost')) ||
        (errMsg.includes('WebGL') && errMsg.includes('failed')) ||
        errMsg.includes('Unable to find WebGL') ||
        errMsg.includes('GPU process');

      if (isFatal) {
        diag('Earth', 'ERROR', `CesiumJS 致命渲染错误: ${errMsg}`);
        if (viewer && !viewer.isDestroyed()) viewer.destroy();
        setEngine('globe-gl');
      } else {
        diag('Earth', 'WARN', `CesiumJS 非致命渲染错误（已忽略）: ${errMsg.substring(0, 500)}`);
      }
    });

    viewer.camera.flyTo({
      destination: Cesium.Cartesian3.fromDegrees(104, 30, 25000000),
      duration: 0,
    });

    // 坐标状态栏：相机移动 + 鼠标移动时上报视角中心坐标 + 海拔。
    const emitView = () => {
      const c = viewer.camera.positionCartographic;
      if (!c) return;
      const Cesium2 = (window as any).Cesium;
      coordRef.current?.({
        lng: Cesium2.Math.toDegrees(c.longitude),
        lat: Cesium2.Math.toDegrees(c.latitude),
        zoom: c.height / 1000, // km
        scale: null, // 3D 暂不报比例尺
      });
    };
    viewer.camera.moveEnd.addEventListener(emitView);

    const handler = new Cesium.ScreenSpaceEventHandler(viewer.scene.canvas);
    handler.setInputAction((movement: any) => {
      const cartesian = viewer.camera.pickEllipsoid(movement.endPosition, viewer.scene.globe.ellipsoid);
      if (cartesian) {
        const carto = viewer.scene.globe.ellipsoid.cartesianToCartographic(cartesian);
        const Cesium2 = (window as any).Cesium;
        coordRef.current?.({
          lng: Cesium2.Math.toDegrees(carto.longitude),
          lat: Cesium2.Math.toDegrees(carto.latitude),
          zoom: viewer.camera.positionCartographic?.height != null
            ? viewer.camera.positionCartographic.height / 1000
            : null,
          scale: null,
        });
      }
    }, Cesium.ScreenSpaceEventType.MOUSE_MOVE);
    // ponytail: handler 随 viewer 销毁（Cesium destroy 链），不单独 track。

    viewerRef.current = viewer;
    diag('Earth', 'INFO', 'CesiumJS Viewer 创建成功');
    onSuccess();
  }

  // Shared interaction: pause rotation on user drag/scroll
  // 用户操作后延迟固定时间恢复自转（由设置档位决定，1/3/5/10 分钟或关闭）。
  const pauseRotation = useCallback(() => {
    rotatingRef.current = false;
    if (resumeTimerRef.current) clearTimeout(resumeTimerRef.current);
    if (resumeDelayMs > 0) {
      diag('Earth', 'INFO', `自转暂停，${Math.round(resumeDelayMs / 1000)}s 后恢复`);
      resumeTimerRef.current = setTimeout(() => {
        rotatingRef.current = true;
        resumeTimerRef.current = null;
      }, resumeDelayMs);
    }
  }, [resumeDelayMs]);

  // CesiumJS auto-rotation
  useEffect(() => {
    if (engine !== 'cesium' || !ready || !viewerRef.current) return;
    const viewer = viewerRef.current;
    const Cesium = (window as any).Cesium;
    let lastTime = performance.now();
    let rafId: number;
    const rotate = (now: number) => {
      const dt = (now - lastTime) / 1000;
      lastTime = now;
      if (rotatingRef.current && !viewer.isDestroyed()) {
        viewer.scene.camera.rotate(
          Cesium.Cartesian3.UNIT_Z,
          -Cesium.Math.toRadians(rotationSpeed * dt)
        );
      }
      rafId = requestAnimationFrame(rotate);
    };
    rafId = requestAnimationFrame(rotate);
    return () => cancelAnimationFrame(rafId);
  }, [engine, ready, rotationSpeed]);

  // react-globe.gl auto-rotation
  useEffect(() => {
    if (engine !== 'globe-gl' || !ready || !globeRef.current) return;
    let lastTime = performance.now();
    let rafId: number;
    const rotate = (now: number) => {
      const dt = (now - lastTime) / 1000;
      lastTime = now;
      if (rotatingRef.current) {
        const pov = globeRef.current?.pointOfView();
        if (pov) {
          globeRef.current?.pointOfView({
            lat: pov.lat,
            lng: (pov.lng + rotationSpeed * dt) % 360,
            altitude: pov.altitude,
          });
        }
      }
      // 坐标状态栏：react-globe.gl 每帧上报视角（altitude 单位是地球半径倍数）。
      const pov = globeRef.current?.pointOfView();
      if (pov) {
        coordRef.current?.({
          lng: pov.lng,
          lat: pov.lat,
          zoom: pov.altitude * 6371, // 地球半径倍数 → km
          scale: null,
        });
      }
      rafId = requestAnimationFrame(rotate);
    };
    rafId = requestAnimationFrame(rotate);
    return () => cancelAnimationFrame(rafId);
  }, [engine, ready, rotationSpeed]);

  // 引擎就绪只通知一次（Cesium 与 globe.gl 互斥，但回退路径可能重复触发）。
  const fireReadyOnce = useCallback(() => {
    if (readyFiredRef.current) return;
    readyFiredRef.current = true;
    readyRef.current?.();
  }, []);

  const onGlobeReady = useCallback(() => {
    diag('Earth', 'INFO', 'react-globe.gl 加载完成');
    setReady(true);
    fireReadyOnce();
    if (globeRef.current) {
      globeRef.current.pointOfView({ lat: 30, lng: 104, altitude: 2.5 }, 0);
      applyCosmosPreset();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ── Cosmos preset (adapted from World Monitor GlobeMap.ts L3439-3536) ──

  const applyCosmosPreset = useCallback(() => {
    const g = globeRef.current as any;
    if (!g) return;
    if (cosmosRef.current.outerGlow) return;

    try {
      const scene: THREE.Scene = g.scene();

      let globeMesh: THREE.Mesh | null = null;
      scene.traverse((obj: THREE.Object3D) => {
        if (!globeMesh && obj instanceof THREE.Mesh && (obj as any).geometry?.type === 'SphereGeometry') {
          const geo = (obj as any).geometry;
          if (geo.parameters?.radius > 1.5) {
            globeMesh = obj as THREE.Mesh;
          }
        }
      });

      if (globeMesh) {
        const mesh = globeMesh as THREE.Mesh;
        const oldMat = mesh.material as THREE.Material;
        cosmosRef.current.savedMaterial = oldMat;
        const stdMat = new THREE.MeshStandardMaterial({
          color: 0xffffff,
          roughness: 0.8,
          metalness: 0.1,
          emissive: new THREE.Color(0x0a1f2e),
          emissiveIntensity: 0.3,
        });
        if ((oldMat as any).map) stdMat.map = (oldMat as any).map;
        mesh.material = stdMat;
      }

      const cyanLight = new THREE.PointLight(0x00d4ff, 0.3);
      cyanLight.position.set(-10, -10, -10);
      scene.add(cyanLight);
      cosmosRef.current.cyanLight = cyanLight;

      const outerGeo = new THREE.SphereGeometry(2.15, 24, 24);
      const outerMat = new THREE.MeshBasicMaterial({
        color: 0x00d4ff,
        side: THREE.BackSide,
        transparent: true,
        opacity: 0.15,
      });
      const outerGlow = new THREE.Mesh(outerGeo, outerMat);
      scene.add(outerGlow);
      cosmosRef.current.outerGlow = outerGlow;

      const innerGeo = new THREE.SphereGeometry(2.08, 24, 24);
      const innerMat = new THREE.MeshBasicMaterial({
        color: 0x00a8cc,
        side: THREE.BackSide,
        transparent: true,
        opacity: 0.1,
      });
      const innerGlow = new THREE.Mesh(innerGeo, innerMat);
      scene.add(innerGlow);
      cosmosRef.current.innerGlow = innerGlow;

      const starCount = 600;
      const starPositions = new Float32Array(starCount * 3);
      const starColors = new Float32Array(starCount * 3);
      for (let i = 0; i < starCount; i++) {
        const r = 50 + Math.random() * 50;
        const theta = Math.random() * Math.PI * 2;
        const phi = Math.acos(2 * Math.random() - 1);
        starPositions[i * 3] = r * Math.sin(phi) * Math.cos(theta);
        starPositions[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
        starPositions[i * 3 + 2] = r * Math.cos(phi);
        const brightness = 0.5 + Math.random() * 0.5;
        starColors[i * 3] = brightness;
        starColors[i * 3 + 1] = brightness;
        starColors[i * 3 + 2] = brightness;
      }
      const starGeo = new THREE.BufferGeometry();
      starGeo.setAttribute('position', new THREE.BufferAttribute(starPositions, 3));
      starGeo.setAttribute('color', new THREE.BufferAttribute(starColors, 3));
      const starMat = new THREE.PointsMaterial({
        size: 0.1,
        vertexColors: true,
        transparent: true,
      });
      const starField = new THREE.Points(starGeo, starMat);
      scene.add(starField);
      cosmosRef.current.starField = starField;

      const animate = () => {
        if (!cosmosRef.current.outerGlow) return;
        cosmosRef.current.outerGlow!.rotation.y += 0.0003;
        if (cosmosRef.current.starField) {
          cosmosRef.current.starField.rotation.y += 0.00005;
        }
        cosmosRef.current.animFrameId = requestAnimationFrame(animate);
      };
      cosmosRef.current.animFrameId = requestAnimationFrame(animate);

      diag('Earth', 'INFO', 'Cosmos 视觉增强已应用');
    } catch (e) {
      diag('Earth', 'WARN', `Cosmos 增强失败: ${e}`);
    }
  }, []);

  const removeCosmosPreset = useCallback(() => {
    const g = globeRef.current as any;
    const c = cosmosRef.current;

    if (c.animFrameId != null) {
      cancelAnimationFrame(c.animFrameId);
      c.animFrameId = null;
    }

    const scene: THREE.Scene | null = g ? g.scene() : null;

    for (const obj of [c.outerGlow, c.innerGlow, c.starField, c.cyanLight]) {
      if (!obj) continue;
      if (scene) scene.remove(obj);
      if (obj instanceof THREE.Mesh || obj instanceof THREE.Points) {
        obj.geometry?.dispose();
        (obj.material as THREE.Material)?.dispose();
      }
    }

    if (scene && c.savedMaterial) {
      scene.traverse((obj: THREE.Object3D) => {
        if (obj instanceof THREE.Mesh && (obj.material as any)?.isMeshStandardMaterial && c.savedMaterial) {
          const texMap = (obj.material as any).map;
          (obj.material as THREE.Material).dispose();
          if (texMap) (c.savedMaterial as any).map = texMap;
          obj.material = c.savedMaterial;
        }
      });
    }

    c.outerGlow = null;
    c.innerGlow = null;
    c.starField = null;
    c.cyanLight = null;
    c.savedMaterial = null;
  }, []);

  // Mouse interaction
  useEffect(() => {
    if (engine === 'cesium') {
      const container = containerRef.current;
      if (!container) return;
      const onDown = () => pauseRotation();
      const onWheel = () => pauseRotation();
      container.addEventListener('pointerdown', onDown);
      container.addEventListener('wheel', onWheel, { passive: true });
      return () => {
        container.removeEventListener('pointerdown', onDown);
        container.removeEventListener('wheel', onWheel);
      };
    } else if (engine === 'globe-gl') {
      const container = document.querySelector('.react-globe-gl canvas')?.parentElement ?? window;
      const onDown = () => pauseRotation();
      const onWheel = () => pauseRotation();
      container.addEventListener('pointerdown', onDown);
      container.addEventListener('wheel', onWheel, { passive: true });
      return () => {
        container.removeEventListener('pointerdown', onDown);
        container.removeEventListener('wheel', onWheel);
      };
    }
  }, [engine, pauseRotation]);

  // Resize observer — drives size state (for react-globe.gl width/height) and
  // CesiumJS viewer.resize(). One observer covers both engines.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const observer = new ResizeObserver(entries => {
      for (const entry of entries) {
        const cr = entry.contentRect;
        setSize({ w: Math.round(cr.width), h: Math.round(cr.height) });
      }
      if (viewerRef.current && !viewerRef.current.isDestroyed()) {
        viewerRef.current.resize();
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, [engine, ready]);

  // Cleanup timers + viewer + cosmos
  useEffect(() => {
    return () => {
      if (resumeTimerRef.current) clearTimeout(resumeTimerRef.current);
      removeCosmosPreset();
      if (viewerRef.current && !viewerRef.current.isDestroyed()) {
        viewerRef.current.destroy();
      }
    };
  }, [removeCosmosPreset]);

  // Reset globe (rebuild viewer with fixed product code — no user code execution)
  useEffect(() => {
    const handleReset = () => {
      diag('Earth', 'INFO', '重置地球');
      if (viewerRef.current && !viewerRef.current.isDestroyed()) {
        viewerRef.current.destroy();
        viewerRef.current = null;
      }
      removeCosmosPreset();
      setEngine('loading');
      setReady(false);
      setTimeout(() => setResetCounter(c => c + 1), 50);
    };

    window.addEventListener(EVENTS.RESET_GLOBE, handleReset);
    return () => window.removeEventListener(EVENTS.RESET_GLOBE, handleReset);
  }, [removeCosmosPreset]);

  const showCesiumContainer = engine === 'loading' || engine === 'cesium';
  const showLoading = engine === 'loading';

  return (
    <>
      {/* CesiumJS container */}
      <div
        ref={containerRef}
        id={CESIUM_CONTAINER_ID}
        style={{
          position: 'absolute',
          inset: 0,
          visibility: showCesiumContainer ? 'visible' : 'hidden',
          pointerEvents: showCesiumContainer ? 'auto' : 'none',
        }}
      />

      {/* react-globe.gl fallback */}
      {engine === 'globe-gl' && (
        <Globe
          ref={globeRef}
          width={size.w || window.innerWidth}
          height={size.h || window.innerHeight}
          globeImageUrl={isTauriEnvironment() ? textures.globe : TEXTURE_URLS[globeTexture]}
          bumpImageUrl={textures.bump}
          backgroundImageUrl={textures.background}
          atmosphereColor="#4466cc"
          atmosphereAltitude={0.18}
          onGlobeReady={onGlobeReady}
          backgroundColor="rgba(0,0,0,0)"
          enablePointerInteraction={true}
        />
      )}

      {/* Loading overlay */}
      {showLoading && (
        <div className="aur-globe-loader">
          <span className="aur-globe-loader__icon">🌍</span>
          <span className="aur-globe-loader__text">正在初始化地球</span>
          <span className="aur-globe-loader__credit">由 CesiumJS 驱动</span>
        </div>
      )}
    </>
  );
}

function loadScript(src: string): Promise<void> {
  return new Promise((resolve, reject) => {
    if (document.querySelector(`script[src="${src}"]`)) { resolve(); return; }
    const s = document.createElement('script');
    s.src = src;
    s.type = 'text/javascript';
    s.onload = () => resolve();
    s.onerror = () => reject(new Error(`加载脚本失败: ${src}`));
    document.head.appendChild(s);
  });
}

function loadCSS(href: string): Promise<void> {
  return new Promise((resolve, reject) => {
    if (document.querySelector(`link[href="${href}"]`)) { resolve(); return; }
    const l = document.createElement('link');
    l.rel = 'stylesheet';
    l.href = href;
    l.onload = () => resolve();
    l.onerror = () => reject(new Error(`加载 CSS 失败: ${href}`));
    document.head.appendChild(l);
  });
}
