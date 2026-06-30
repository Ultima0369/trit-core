// ui/src/BootScreen.tsx — 启动加载屏：太阳系 3D 持续转动 + 真实里程碑读条
//
// 盖住 Tauri 启动期（Cesium.js 拉取 + 首次管线分析）的空白。three.js 生命周期
// 模式：场景/星场/rAF/cleanup 全套 dispose。零新增依赖——three 已由 react-globe.gl
// 带入。淡出由父组件经 fadeOut 触发，动画结束后卸载。
//
// 太阳系视觉设计借鉴 solar-system-master (MIT, Copyright (c) 2020 Richard Chan)：
// 9 行星贴图 + 土星环 + 轨道环 + 20000 颗多色星场。贴图来自 ui/public/solar/。

import { useEffect, useRef, useState, type CSSProperties } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import diag, { isTauriEnvironment } from './utils/diag';

const RESOURCE_SERVER = 'http://localhost:21337';

// ── 视觉/时序配置（集中管理，避免魔法数字散落）──
const SUN_RADIUS = 8;
const GLOW_RADII = [9.5, 14, 22, 34, 50, 70, 92, 118, 145]; // 光晕层半径，覆盖到冥王星轨道
const GLOW_COLOR = 0xffe8c0;        // 暖白（非橙黄，避免火烧感）
const FLARE_COUNT = 300;            // 太阳表面耀斑粒子数
const FLARE_OPACITY = 0.27;
const STAR_COUNT = 20000;           // 背景星场粒子数
const STAR_GAP = 900;               // 星场距太阳系中心的最近距离
const ORBIT_TRACK_COLOR = 0x2a3548;
const ORBIT_TRACK_OPACITY = 0.25;
const MOON_RADIUS_RATIO = 0.35;     // 月亮半径 = 地球半径 * 此值
const MOON_ORBIT_RATIO = 2.4;       // 月亮轨道 = 地球半径 * 此值
const MOON_SPEED = 0.6;
const CAMERA_FOV = 45;
const CAMERA_POS: [number, number, number] = [0, 95, 180];
const ORBIT_DIST_MIN = 50;
const ORBIT_DIST_MAX = 500;
const FADE_MS = 1200;               // 淡出动画时长（与 CSS 一致）
const AWAIT_TIMEOUT_MS = 30000;     // 里程碑卡住时强制显示按钮

// ── 类型定义（替代 any）──
interface AssetReport {
  assets?: { name: string; category: string; status: string }[];
}
interface PlanetUserData {
  speed: number;
  phase: number;
  orbit: number;
}

/// 拉取资源状态报告（与 Earth.tsx 同款）。
async function fetchAssetStatus(): Promise<AssetReport> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<AssetReport>('get_asset_status');
}

/// 通用就绪探测 hook：轮询 probe() 直到返回 true 或超时，然后调 onReady。
/// 收口后端/资源两处重复的 setInterval+setTimeout+cancelled+clear 模式。
function useReadyProbe(
  probe: () => Promise<boolean>,
  intervalMs: number,
  timeoutMs: number,
  onReady: () => void,
  label: string,
) {
  useEffect(() => {
    if (!isTauriEnvironment()) {
      onReady();
      return;
    }
    let cancelled = false;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const check = setInterval(async () => {
      try {
        if (await probe() && !cancelled) {
          diag('Boot', 'INFO', `${label}就绪`);
          onReady();
          clearInterval(check);
          if (timeout) clearTimeout(timeout);
        }
      } catch { /* not ready yet */ }
    }, intervalMs);
    timeout = setTimeout(() => {
      clearInterval(check);
      if (!cancelled) {
        diag('Boot', 'WARN', `${label}探测超时，标记就绪以避免死锁`);
        onReady();
      }
    }, timeoutMs);
    return () => { cancelled = true; clearInterval(check); if (timeout) clearTimeout(timeout); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}

interface Milestone {
  label: string;
  done: boolean;
}

interface Props {
  /// 外部注入的里程碑（地球引擎就绪 / 首次分析完成），与内部探测的后端/资源里程碑合并。
  externalMilestones: Milestone[];
  fadeOut: boolean;
  onFadeComplete: () => void;
  /// 进度条满后显示按钮，用户点击时触发——通知父组件开始"地球登场"过渡。
  onTransitionReady: () => void;
}

export default function BootScreen({ externalMilestones, fadeOut, onFadeComplete, onTransitionReady }: Props) {
  const canvasHostRef = useRef<HTMLDivElement>(null);
  const [backendReady, setBackendReady] = useState(false);
  const [assetsReady, setAssetsReady] = useState(false);

  // 里程碑 1: 后端服务就绪（fetch /health 轮询）
  useReadyProbe(
    async () => (await fetch(`${RESOURCE_SERVER}/health`)).ok,
    200, 15000,
    () => setBackendReady(true),
    '后端服务',
  );

  // 里程碑 2: 资源就绪（cesium category cached/ok）
  useReadyProbe(
    async () => {
      const report = await fetchAssetStatus();
      return !!report.assets?.some(
        (a) => a.category === 'cesium' && (a.status === 'cached' || a.status === 'ok'),
      );
    },
    500, 20000,
    () => setAssetsReady(true),
    'Cesium 资源',
  );

  // ── Three.js 太阳系 ──
  useEffect(() => {
    const host = canvasHostRef.current;
    if (!host) return;

    // 无 WebGL 时优雅跳过（jsdom 测试环境或无 GPU 设备）。
    // 用 try/catch 兜底——仅检测 WebGLRenderingContext 类型不够，jsdom 定义类型却无实际能力。
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true });
    } catch (e) {
      diag('Boot', 'INFO', `无 WebGL，跳过太阳系 3D: ${e}`);
      return;
    }

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(CAMERA_FOV, host.clientWidth / host.clientHeight, 1, 6000);
    camera.position.set(...CAMERA_POS);
    camera.lookAt(0, 0, 0);

    renderer.setSize(host.clientWidth, host.clientHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    host.appendChild(renderer.domElement);

    // 拖动/缩放控制：过场期允许用户旋转视角看太阳系。
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.minDistance = ORBIT_DIST_MIN;
    controls.maxDistance = ORBIT_DIST_MAX;

    scene.add(new THREE.AmbientLight(0x222233, 0.6));
    const sunLight = new THREE.PointLight(0xffd9a0, 2.5, 1000);
    sunLight.position.set(0, 0, 0);
    scene.add(sunLight);

    const loader = new THREE.TextureLoader();
    // 贴图在 ui/public/solar/，Vite 以根路径 / 服务；Tauri 打包后同路径。
    const tex = (name: string) => loader.load(`/solar/${name}`);

    // ── 太阳：贴图 + 多层光晕 + 表面耀斑粒子 ──
    const sun = new THREE.Mesh(
      new THREE.SphereGeometry(SUN_RADIUS, 48, 48),
      new THREE.MeshBasicMaterial({ map: tex('sun_bg.jpg') })
    );
    scene.add(sun);
    // 多层光晕：9 层递增弥散，从太阳表面扩散到最外围行星轨道。
    // opacity 非线性递减至几乎不可见。
    const glowLayers = GLOW_RADII.map((r, i) => {
      const fade = Math.pow(1 - i / (GLOW_RADII.length - 1), 1.5);
      const mesh = new THREE.Mesh(
        new THREE.SphereGeometry(r, 32, 32),
        new THREE.MeshBasicMaterial({ color: GLOW_COLOR, side: THREE.BackSide, transparent: true, opacity: 0.055 * fade + 0.004 })
      );
      scene.add(mesh);
      return mesh;
    });
    const flarePos = new Float32Array(FLARE_COUNT * 3);
    for (let i = 0; i < FLARE_COUNT; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const r = SUN_RADIUS + Math.random() * 2;
      flarePos[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      flarePos[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      flarePos[i * 3 + 2] = r * Math.cos(phi);
    }
    const flareGeo = new THREE.BufferGeometry();
    flareGeo.setAttribute('position', new THREE.BufferAttribute(flarePos, 3));
    const flareMat = new THREE.PointsMaterial({ size: 0.7, color: GLOW_COLOR, transparent: true, opacity: FLARE_OPACITY, blending: THREE.AdditiveBlending });
    const flares = new THREE.Points(flareGeo, flareMat);
    scene.add(flares);

    // ── 9 行星：贴图 + 轨道环 + 土星环（借鉴 solar-system-master loadPlanet）──
    interface PlanetDef { name: string; r: number; orbit: number; speed: number; }
    const planetDefs: PlanetDef[] = [
      { name: 'mercury', r: 1.2, orbit: 18,  speed: 0.18 },
      { name: 'venus',   r: 2.4, orbit: 26,  speed: 0.14 },
      { name: 'earth',   r: 2.8, orbit: 36,  speed: 0.11 },
      { name: 'mars',    r: 2.2, orbit: 46,  speed: 0.09 },
      { name: 'jupiter', r: 5.0, orbit: 64,  speed: 0.06 },
      { name: 'saturn',  r: 4.2, orbit: 88,  speed: 0.045 },
      { name: 'uranus',  r: 2.6, orbit: 108, speed: 0.03 },
      { name: 'neptune', r: 2.4, orbit: 128, speed: 0.022 },
      { name: 'pluto',   r: 1.0, orbit: 145, speed: 0.016 },
    ];
    const planets: THREE.Mesh[] = [];
    const planetTracks: THREE.Mesh[] = [];
    const saturnRings: THREE.Mesh[] = [];
    const moonPivots: THREE.Object3D[] = [];
    for (const d of planetDefs) {
      const planet = new THREE.Mesh(
        new THREE.SphereGeometry(d.r, 32, 32),
        new THREE.MeshBasicMaterial({ map: tex(`${d.name}_bg.jpg`) })
      );
      // 随机起始相位，避免所有行星排成一线（cos/sin 定位而非纯 -z）。
      const phase = Math.random() * Math.PI * 2;
      planet.position.x = Math.cos(phase) * d.orbit;
      planet.position.z = Math.sin(phase) * d.orbit;
      planet.userData = { speed: d.speed, phase, orbit: d.orbit };
      scene.add(planet);
      planets.push(planet);

      // 土星环
      if (d.name === 'saturn') {
        const ring = new THREE.Mesh(
          new THREE.RingGeometry(d.r * 1.4, d.r * 2.0, 64, 1),
          new THREE.MeshBasicMaterial({ map: tex('saturn_ring.jpg'), side: THREE.DoubleSide, transparent: true })
        );
        ring.rotation.x = -Math.PI / 2;
        planet.add(ring);
        saturnRings.push(ring);
      }

      // 月亮：绕地球公转（pivot 挂地球下，避免跟地球自转）
      if (d.name === 'earth') {
        const moonPivot = new THREE.Object3D();
        planet.add(moonPivot);
        const moon = new THREE.Mesh(
          new THREE.SphereGeometry(d.r * MOON_RADIUS_RATIO, 24, 24),
          new THREE.MeshBasicMaterial({ map: tex('moon_bg.jpg') })
        );
        moon.position.x = d.r * MOON_ORBIT_RATIO;
        moonPivot.add(moon);
        moonPivot.userData = { speed: MOON_SPEED };
        moonPivots.push(moonPivot);
      }

      // 轨道环线
      const track = new THREE.Mesh(
        new THREE.RingGeometry(d.orbit, d.orbit + 0.15, 96, 1),
        new THREE.MeshBasicMaterial({ color: ORBIT_TRACK_COLOR, side: THREE.DoubleSide, transparent: true, opacity: ORBIT_TRACK_OPACITY })
      );
      track.rotation.x = -Math.PI / 2;
      scene.add(track);
      planetTracks.push(track);
    }

    // ── 星场：多色星，box 分布 + 70% 彩色（借鉴 solar-system-master initParticle）──
    const starPos = new Float32Array(STAR_COUNT * 3);
    const starColors = new Float32Array(STAR_COUNT * 3);
    const tmpColor = new THREE.Color();
    for (let i = 0; i < STAR_COUNT; i++) {
      let x = (Math.random() * STAR_GAP * 2) * (Math.random() < 0.5 ? -1 : 1);
      let y = (Math.random() * STAR_GAP * 2) * (Math.random() < 0.5 ? -1 : 1);
      let z = (Math.random() * STAR_GAP * 2) * (Math.random() < 0.5 ? -1 : 1);
      // 确保星星在 STAR_GAP 距离之外（不扎进太阳系内部）：找绝对值最大的轴，不足则补齐
      const ax = Math.abs(x), ay = Math.abs(y), az = Math.abs(z);
      if (ax >= ay && ax >= az) { if (ax < STAR_GAP) x = x < 0 ? -STAR_GAP : STAR_GAP; }
      else if (ay >= az) { if (ay < STAR_GAP) y = y < 0 ? -STAR_GAP : STAR_GAP; }
      else { if (az < STAR_GAP) z = z < 0 ? -STAR_GAP : STAR_GAP; }
      starPos[i * 3] = x; starPos[i * 3 + 1] = y; starPos[i * 3 + 2] = z;
      if (Math.random() > 0.3) {
        tmpColor.setRGB((Math.random() + 1) / 2, (Math.random() + 1) / 2, (Math.random() + 1) / 2);
      } else {
        tmpColor.setRGB(1, 1, 1);
      }
      starColors[i * 3] = tmpColor.r; starColors[i * 3 + 1] = tmpColor.g; starColors[i * 3 + 2] = tmpColor.b;
    }
    const starGeo = new THREE.BufferGeometry();
    starGeo.setAttribute('position', new THREE.BufferAttribute(starPos, 3));
    starGeo.setAttribute('color', new THREE.BufferAttribute(starColors, 3));
    const starMat = new THREE.PointsMaterial({ size: 3, vertexColors: true, transparent: true, opacity: 0.9 });
    const starField = new THREE.Points(starGeo, starMat);
    scene.add(starField);

    // dt 驱动动画，避免帧率漂移导致转速随帧率变化
    let rafId = 0;
    let lastTime = performance.now();
    const animate = () => {
      const now = performance.now();
      const dt = (now - lastTime) / 1000;
      lastTime = now;
      const t = now / 1000;

      sun.rotation.y += 0.05 * dt;
      // 光晕脉动：内层明显，外层几乎不脉动（避免大半径球剧烈摆动）
      glowLayers.forEach((g, i) => {
        const amp = 0.04 * Math.pow(1 - i / glowLayers.length, 1.5);
        g.scale.setScalar(1 + Math.sin(t * 0.6 + i) * amp);
      });
      flares.rotation.y += 0.04 * dt;

      // 每行星独立公转（各自 phase + speed）+ 自转
      for (const p of planets) {
        const ud = p.userData as PlanetUserData;
        ud.phase += ud.speed * dt;
        p.position.x = Math.cos(ud.phase) * ud.orbit;
        p.position.z = Math.sin(ud.phase) * ud.orbit;
        p.rotation.y += 0.3 * dt;
      }
      // 月亮绕地球公转
      for (const mp of moonPivots) {
        mp.rotation.y += (mp.userData as { speed: number }).speed * dt;
      }

      starField.rotation.y += 0.003 * dt;

      controls.update();
      renderer.render(scene, camera);
      rafId = requestAnimationFrame(animate);
    };
    rafId = requestAnimationFrame(animate);

    // resize
    const onResize = () => {
      const w = host.clientWidth, h = host.clientHeight;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    };
    window.addEventListener('resize', onResize);

    // cleanup（dispose 所有几何/材质，防泄漏）——统一收口到 disposables
    const disposables: { geo: THREE.BufferGeometry; mat: THREE.Material }[] = [
      { geo: sun.geometry, mat: sun.material as THREE.Material },
      ...planets.map(p => ({ geo: p.geometry, mat: p.material as THREE.Material })),
      ...glowLayers.map(g => ({ geo: g.geometry, mat: g.material as THREE.Material })),
      { geo: flareGeo, mat: flareMat },
      { geo: starGeo, mat: starMat },
      ...planetTracks.map(t => ({ geo: t.geometry, mat: t.material as THREE.Material })),
      ...saturnRings.map(r => ({ geo: r.geometry, mat: r.material as THREE.Material })),
      ...moonPivots.map(mp => {
        const m = mp.children[0];
        // 防御：moonPivot 应挂一个 moon mesh，若无则跳过（不应发生）
        return m instanceof THREE.Mesh
          ? { geo: m.geometry, mat: m.material as THREE.Material }
          : null;
      }).filter((d): d is { geo: THREE.BufferGeometry; mat: THREE.Material } => d !== null),
    ];
    return () => {
      cancelAnimationFrame(rafId);
      window.removeEventListener('resize', onResize);
      controls.dispose();
      renderer.dispose();
      disposables.forEach(d => { d.geo.dispose(); d.mat.dispose(); });
      if (renderer.domElement.parentNode === host) host.removeChild(renderer.domElement);
    };
  }, []);

  // ── 淡出动画结束通知（与 CSS transition 时长一致，见 FADE_MS）──
  useEffect(() => {
    if (!fadeOut) return;
    const t = setTimeout(onFadeComplete, FADE_MS);
    return () => clearTimeout(t);
  }, [fadeOut, onFadeComplete]);

  const milestones: Milestone[] = [
    { label: '后端服务', done: backendReady },
    { label: 'Cesium 资源', done: assetsReady },
    ...externalMilestones,
  ];
  const doneCount = milestones.filter(m => m.done).length;
  const allDone = doneCount === milestones.length;
  const [awaitingClick, setAwaitingClick] = useState(false);

  // 进度条满 → 进入"等待点击"态。兜底：若里程碑卡住（如管线未返回），
  // AWAIT_TIMEOUT_MS 后也显示按钮让用户进入，不卡死在进度条。
  useEffect(() => {
    if (awaitingClick) return;
    if (allDone) { setAwaitingClick(true); return; }
    const t = setTimeout(() => {
      diag('Boot', 'WARN', '里程碑超时，强制显示进入按钮');
      setAwaitingClick(true);
    }, AWAIT_TIMEOUT_MS);
    return () => clearTimeout(t);
  }, [allDone, awaitingClick]);

  const pct = (doneCount / milestones.length) * 100;

  return (
    <div className={`aur-boot-screen${fadeOut ? ' aur-boot-screen--fade' : ''}${awaitingClick ? ' is-awaiting' : ''}`}>
      <div className="aur-boot-canvas" ref={canvasHostRef} />
      <div className="aur-boot-overlay">
        <div className="aur-boot-title">Aurora 极光</div>
        <div className="aur-boot-bottom">
          {!awaitingClick && (
            <>
              <div className="aur-boot-milestones">
                {milestones.map(m => (
                  <div key={m.label} className={`aur-boot-milestone${m.done ? ' is-done' : ''}`}>
                    <span className="aur-boot-milestone__mark">{m.done ? '✓' : '○'}</span>
                    <span className="aur-boot-milestone__label">{m.label}</span>
                  </div>
                ))}
              </div>
              <div className="aur-boot-progress" style={{ '--pct': `${pct}%` } as CSSProperties}>
                <div className="aur-boot-progress__fill" />
              </div>
            </>
          )}
          {awaitingClick && (
            <button
              className="aur-boot-enter-btn"
              onClick={() => {
                diag('Boot', 'INFO', '用户点击进入按钮，地球登场');
                onTransitionReady();
              }}
            >
              进入 Aurora
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
