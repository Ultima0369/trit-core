// ui/src/BootScreen.tsx — 启动加载屏：太阳系 3D 持续转动 + 真实里程碑读条
//
// 盖住 Tauri 启动期（Cesium.js 拉取 + 首次管线分析）的空白。借鉴 Earth.tsx
// Cosmos preset 的 three.js 生命周期模式（场景/星场/rAF/cleanup）。零新增依赖——
// three 已由 react-globe.gl 带入。淡出由父组件经 fadeOut 触发，动画结束后卸载。
//
// 太阳系视觉设计借鉴 solar-system-master (MIT, Copyright (c) 2020 Richard Chan)：
// 9 行星贴图 + 土星环 + 轨道环 + 20000 颗多色星场。贴图来自 ui/public/solar/。

import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import diag, { isTauriEnvironment } from './utils/diag';

const RESOURCE_SERVER = 'http://localhost:21337';
/// 拉取资源状态报告（与 Earth.tsx 同款）。
async function fetchAssetStatus(): Promise<any> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<any>('get_asset_status');
}

interface Milestone {
  label: string;
  done: boolean;
}

interface Props {
  /// 外部注入的里程碑（地球引擎就绪 / 首次分析完成），与内部探测的后端/资源里程碑合并。
  externalMilestones: { label: string; done: boolean }[];
  fadeOut: boolean;
  onFadeComplete: () => void;
  /// 进度条满 + 至少 2 秒后触发，通知父组件开始"地球登场"过渡。
  onTransitionReady: () => void;
}

export default function BootScreen({ externalMilestones, fadeOut, onFadeComplete, onTransitionReady }: Props) {
  const canvasHostRef = useRef<HTMLDivElement>(null);
  const [backendReady, setBackendReady] = useState(false);
  const [assetsReady, setAssetsReady] = useState(false);

  // ── 里程碑 1: 后端服务就绪（fetch /health 轮询，复用 Earth.tsx:124 模式）──
  useEffect(() => {
    if (!isTauriEnvironment()) {
      setBackendReady(true);
      return;
    }
    let cancelled = false;
    const check = setInterval(async () => {
      try {
        const res = await fetch(`${RESOURCE_SERVER}/health`);
        if (res.ok && !cancelled) {
          diag('Boot', 'INFO', '后端服务就绪');
          setBackendReady(true);
          clearInterval(check);
        }
      } catch { /* server not ready yet */ }
    }, 200);
    const timeout = setTimeout(() => {
      clearInterval(check);
      if (!cancelled) {
        diag('Boot', 'WARN', '后端探测超时，标记就绪以避免死锁');
        setBackendReady(true);
      }
    }, 15000);
    return () => { cancelled = true; clearInterval(check); clearTimeout(timeout); };
  }, []);

  // ── 里程碑 2: 资源就绪（cesium category cached/ok，复用 Earth.tsx:164 判断）──
  useEffect(() => {
    if (!isTauriEnvironment()) {
      setAssetsReady(true);
      return;
    }
    let cancelled = false;
    const check = setInterval(async () => {
      try {
        const report = await fetchAssetStatus();
        const ok = report.assets?.some(
          (a: any) => a.category === 'cesium' && (a.status === 'cached' || a.status === 'ok')
        );
        if (ok && !cancelled) {
          diag('Boot', 'INFO', 'Cesium 资源就绪');
          setAssetsReady(true);
          clearInterval(check);
        }
      } catch { /* not ready */ }
    }, 500);
    const timeout = setTimeout(() => {
      clearInterval(check);
      if (!cancelled) {
        diag('Boot', 'WARN', '资源探测超时，标记就绪以避免死锁');
        setAssetsReady(true);
      }
    }, 20000);
    return () => { cancelled = true; clearInterval(check); clearTimeout(timeout); };
  }, []);

  // ── Three.js 太阳系（生命周期借鉴 Earth.tsx Cosmos preset）──
  useEffect(() => {
    const host = canvasHostRef.current;
    if (!host) return;

    // jsdom/无 WebGL 守卫：测试环境无 WebGLRenderingContext，跳过 3D。
    const WebGLCtx = (window as any).WebGLRenderingContext;
    if (!WebGLCtx) {
      diag('Boot', 'INFO', '无 WebGL，跳过太阳系 3D（测试环境）');
      return;
    }

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(45, host.clientWidth / host.clientHeight, 1, 6000);
    camera.position.set(0, 180, 320);
    camera.lookAt(0, 0, 0);

    const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true });
    renderer.setSize(host.clientWidth, host.clientHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    host.appendChild(renderer.domElement);

    // 光照：太阳点光源 + 微弱环境光
    scene.add(new THREE.AmbientLight(0x222233, 0.6));
    const sunLight = new THREE.PointLight(0xffd9a0, 2.5, 1000);
    sunLight.position.set(0, 0, 0);
    scene.add(sunLight);

    const loader = new THREE.TextureLoader();
    // 贴图在 ui/public/solar/，Vite 以根路径 / 服务；Tauri 打包后同路径。
    const tex = (name: string) => loader.load(`/solar/${name}`);

    // 整个太阳系父级（公转通过旋转它实现，借鉴 solar-system-master）
    const sunSystem = new THREE.Object3D();
    scene.add(sunSystem);

    // ── 太阳：贴图 + 多层光晕 + 表面耀斑粒子 ──
    const sun = new THREE.Mesh(
      new THREE.SphereGeometry(8, 48, 48),
      new THREE.MeshBasicMaterial({ map: tex('sun_bg.jpg') })
    );
    sunSystem.add(sun);
    const glowLayers = [9.5, 11, 13].map((r, i) => {
      const mesh = new THREE.Mesh(
        new THREE.SphereGeometry(r, 32, 32),
        new THREE.MeshBasicMaterial({ color: 0xffaa44, side: THREE.BackSide, transparent: true, opacity: 0.22 - i * 0.06 })
      );
      scene.add(mesh);
      return mesh;
    });
    const flareCount = 300;
    const flarePos = new Float32Array(flareCount * 3);
    for (let i = 0; i < flareCount; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const r = 8 + Math.random() * 2;
      flarePos[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      flarePos[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      flarePos[i * 3 + 2] = r * Math.cos(phi);
    }
    const flareGeo = new THREE.BufferGeometry();
    flareGeo.setAttribute('position', new THREE.BufferAttribute(flarePos, 3));
    const flareMat = new THREE.PointsMaterial({ size: 0.7, color: 0xffd98a, transparent: true, opacity: 0.8, blending: THREE.AdditiveBlending });
    const flares = new THREE.Points(flareGeo, flareMat);
    scene.add(flares);

    // ── 9 行星：贴图 + 轨道环 + 土星环（借鉴 solar-system-master loadPlanet）──
    // [name, radius, orbit, speed] — 半径/轨道借鉴原项目，速度放慢适配过场。
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
    for (const d of planetDefs) {
      const planet = new THREE.Mesh(
        new THREE.SphereGeometry(d.r, 32, 32),
        new THREE.MeshBasicMaterial({ map: tex(`${d.name}_bg.jpg`) })
      );
      planet.position.z = -d.orbit;
      planet.userData = { speed: d.speed };
      sunSystem.add(planet);
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

      // 轨道环线
      const track = new THREE.Mesh(
        new THREE.RingGeometry(d.orbit, d.orbit + 0.15, 96, 1),
        new THREE.MeshBasicMaterial({ color: 0x2a3548, side: THREE.DoubleSide, transparent: true, opacity: 0.5 })
      );
      track.rotation.x = -Math.PI / 2;
      scene.add(track);
      planetTracks.push(track);
    }

    // ── 星场：20000 颗多色星，box 分布 + 70% 彩色（借鉴 solar-system-master initParticle）──
    const starCount = 20000;
    const starPos = new Float32Array(starCount * 3);
    const starColors = new Float32Array(starCount * 3);
    const gap = 900;
    const tmpColor = new THREE.Color();
    for (let i = 0; i < starCount; i++) {
      let x = (Math.random() * gap * 2) * (Math.random() < 0.5 ? -1 : 1);
      let y = (Math.random() * gap * 2) * (Math.random() < 0.5 ? -1 : 1);
      let z = (Math.random() * gap * 2) * (Math.random() < 0.5 ? -1 : 1);
      // 确保星星在 gap 距离之外（不扎进太阳系内部）
      const biggest = Math.abs(x) > Math.abs(y) ? (Math.abs(x) > Math.abs(z) ? 'x' : 'z') : (Math.abs(y) > Math.abs(z) ? 'y' : 'z');
      const pos: Record<string, number> = { x, y, z };
      if (Math.abs(pos[biggest]) < gap) pos[biggest] = pos[biggest] < 0 ? -gap : gap;
      x = pos.x; y = pos.y; z = pos.z;
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

    // 动画循环：整体公转（sunSystem.rotation.y）+ 行星自转 + 太阳光晕脉动 + 星场缓转
    let rafId = 0;
    let lastTime = performance.now();
    const animate = () => {
      const now = performance.now();
      const dt = (now - lastTime) / 1000;
      lastTime = now;
      const t = now / 1000;

      sun.rotation.y += 0.05 * dt;
      glowLayers.forEach((g, i) => { g.scale.setScalar(1 + Math.sin(t * 0.6 + i) * 0.04); });
      flares.rotation.y += 0.04 * dt;

      // 整体公转（借鉴 solar-system-master：旋转父级）
      sunSystem.rotation.y -= 0.02 * dt;
      for (const p of planets) {
        p.rotation.y += 0.3 * dt;
      }

      starField.rotation.y += 0.003 * dt;

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

    // cleanup（dispose 所有几何/材质/贴图，防泄漏）
    const disposables: { geo: THREE.BufferGeometry; mat: THREE.Material }[] = [
      ...glowLayers.map(g => ({ geo: g.geometry, mat: g.material as THREE.Material })),
      { geo: flareGeo, mat: flareMat },
      ...planetTracks.map(t => ({ geo: t.geometry, mat: t.material as THREE.Material })),
      ...saturnRings.map(r => ({ geo: r.geometry, mat: r.material as THREE.Material })),
    ];
    return () => {
      cancelAnimationFrame(rafId);
      window.removeEventListener('resize', onResize);
      renderer.dispose();
      sun.geometry.dispose(); (sun.material as THREE.Material).dispose();
      planets.forEach(p => { p.geometry.dispose(); (p.material as THREE.Material).dispose(); });
      starGeo.dispose(); starMat.dispose();
      disposables.forEach(d => { d.geo.dispose(); d.mat.dispose(); });
      if (renderer.domElement.parentNode === host) host.removeChild(renderer.domElement);
    };
  }, []);

  // ── 淡出动画结束通知（与 CSS 1.2s 时长一致）──
  useEffect(() => {
    if (!fadeOut) return;
    const t = setTimeout(onFadeComplete, 1200);
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

  // 进度条满 → 进入"等待点击"态（不自动进入地球），用户点窗口任意处才开始登场。
  useEffect(() => {
    if (allDone && !awaitingClick) setAwaitingClick(true);
  }, [allDone, awaitingClick]);

  useEffect(() => {
    if (!awaitingClick || fadeOut) return;
    const onClick = () => {
      diag('Boot', 'INFO', '用户点击，地球登场');
      onTransitionReady();
    };
    window.addEventListener('pointerdown', onClick);
    return () => window.removeEventListener('pointerdown', onClick);
  }, [awaitingClick, fadeOut, onTransitionReady]);

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
              <div className="aur-boot-progress">
                <div className="aur-boot-progress__fill" style={{ width: `${pct}%` }} />
              </div>
            </>
          )}
          {awaitingClick && (
            <div className="aur-boot-hint">点击任意处进入</div>
          )}
        </div>
      </div>
    </div>
  );
}
