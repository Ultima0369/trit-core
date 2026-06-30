// ui/src/test/Earth.test.tsx
// Smoke tests for Earth component.
// WebGL rendering (globe.gl / CesiumJS) is mocked — visual Cosmos preset
// verification requires a real browser (`npm run dev`).

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import Earth from '../Earth';
import { EVENTS } from '../types';

// Three.js is imported statically by Earth.tsx — jsdom can't run it.
vi.mock('three', () => {
  const Color = vi.fn(function (this: any, _hex: number | string) {
    this.r = 0; this.g = 0; this.b = 0;
  });
  return {
    Color,
    Mesh: vi.fn(),
    MeshStandardMaterial: vi.fn(() => ({ map: null })),
    MeshBasicMaterial: vi.fn(() => ({})),
    PointsMaterial: vi.fn(() => ({})),
    PointLight: vi.fn(() => ({ position: { set: vi.fn() } })),
    SphereGeometry: vi.fn(() => ({})),
    BufferGeometry: vi.fn(() => ({
      setAttribute: vi.fn(),
    })),
    BufferAttribute: vi.fn(),
    Points: vi.fn(),
    BackSide: 1,
    Scene: vi.fn(() => ({ add: vi.fn(), remove: vi.fn() })),
  };
});

vi.mock('../utils/diag', () => ({
  default: () => {},
  isTauriEnvironment: () => false,
}));

describe('Earth', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows loading overlay when engine is loading', async () => {
    render(<Earth resumeDelayMs={5000} />);

    // In non-Tauri dev mode, engine goes straight to 'globe-gl'.
    // Trigger a reset to force engine back to 'loading'.
    await act(() => {
      window.dispatchEvent(new CustomEvent(EVENTS.RESET_GLOBE));
    });

    // After reset, engine='loading' → loading overlay appears
    expect(screen.getByText('正在初始化地球')).toBeDefined();
    expect(screen.getByText('🌍')).toBeDefined();
  });

  it('recovers from reset back to globe-gl', async () => {
    render(<Earth resumeDelayMs={5000} />);

    // Trigger reset
    await act(() => {
      window.dispatchEvent(new CustomEvent(EVENTS.RESET_GLOBE));
    });

    // Loading overlay visible (rendered before resetCounter increment)
    expect(screen.getByText('正在初始化地球')).toBeDefined();

    // Wait for resetCounter timeout + effect
    await act(async () => {
      await new Promise(r => setTimeout(r, 100));
    });

    // Globe should be back
    expect(screen.getByTestId('globe-gl')).toBeDefined();
    expect(screen.queryByText('正在初始化地球')).toBeNull();
  });

  it('switches to globe-gl engine in non-Tauri environment', () => {
    render(<Earth resumeDelayMs={5000} />);
    expect(screen.getByTestId('globe-gl')).toBeDefined();
  });

  it('dispatches texture-changed event on mount', () => {
    let captured: string | null = null;
    const handler = (e: Event) => {
      captured = (e as CustomEvent).detail.texture;
    };
    window.addEventListener(EVENTS.GLOBE_TEXTURE_CHANGED, handler);

    render(<Earth resumeDelayMs={5000} />);

    expect(captured).toBe('blue-marble');
    window.removeEventListener(EVENTS.GLOBE_TEXTURE_CHANGED, handler);
  });

  it('responds to set-globe-texture event from TopBar', () => {
    render(<Earth resumeDelayMs={5000} />);

    act(() => {
      window.dispatchEvent(new CustomEvent(EVENTS.SET_GLOBE_TEXTURE, {
        detail: { texture: 'topographic' },
      }));
    });

    // Texture state updated — no crash, no DOM change visible in jsdom
    // The real visual change requires WebGL
  });

  it('hides loading overlay after globe is ready', () => {
    render(<Earth resumeDelayMs={5000} />);
    // In non-Tauri mode, engine switches to 'globe-gl' synchronously
    expect(screen.queryByText('正在初始化地球')).toBeNull();
  });

  // Regression: textures must load even while china-tiles (575MB) are still
  // downloading — i.e. all_ready === false. Previously check_cached_assets()
  // returned "" unless all_ready, leaving the globe with no texture.
  // buildTextureUrls is the pure gating logic extracted from the load effect.
  it('builds proxy texture URLs even when all_ready is false (china-tiles pending)', async () => {
    const { buildTextureUrls } = await import('../Earth');
    const urls = buildTextureUrls({
      assets_dir: '/fake/aurora',
      all_ready: false, // china-tiles still downloading
      assets: [
        { name: 'earth-blue-marble.jpg', status: 'cached', category: 'texture' },
        { name: 'earth-topology.png', status: 'cached', category: 'texture' },
        { name: 'night-sky.png', status: 'cached', category: 'texture' },
        { name: '中国卫星影像 z3-z10', status: 'ok', category: 'china-tiles' },
      ],
    });
    expect(urls).not.toBeNull();
    expect(urls!.globe).toBe('http://localhost:21337/assets/earth-blue-marble.jpg');
    expect(urls!.bump).toBe('http://localhost:21337/assets/earth-topology.png');
    expect(urls!.background).toBe('http://localhost:21337/assets/night-sky.png');
  });

  it('returns null when a texture file is still missing', async () => {
    const { buildTextureUrls } = await import('../Earth');
    const urls = buildTextureUrls({
      assets_dir: '/fake/aurora',
      all_ready: false,
      assets: [
        { name: 'earth-blue-marble.jpg', status: 'cached' },
        // earth-topology.png missing
        { name: 'night-sky.png', status: 'cached' },
      ],
    });
    expect(urls).toBeNull();
  });
});
