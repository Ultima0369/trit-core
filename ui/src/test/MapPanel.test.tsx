import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/react';

// mock 动态 import 的 maplibre/pmtiles/protomaps，避免 jsdom 跑 WebGL
vi.mock('maplibre-gl', () => ({
  default: {
    addProtocol: vi.fn(),
    Map: vi.fn().mockImplementation(() => ({ remove: vi.fn() })),
  },
}));
vi.mock('pmtiles', () => ({ Protocol: vi.fn().mockImplementation(() => ({ tile: vi.fn() })) }));
vi.mock('@protomaps/basemaps', () => ({
  layers: vi.fn().mockReturnValue([]),
  namedFlavor: vi.fn().mockReturnValue('light'),
}));

import MapPanel from '../MapPanel';
import { DEFAULT_LAYERS } from '../config/layer-definitions';

describe('MapPanel', () => {
  it('渲染容器 div', async () => {
    const { container } = render(<MapPanel flavor="light" layers={DEFAULT_LAYERS} />);
    const div = container.querySelector('div');
    expect(div).not.toBeNull();
    expect(div!.style.width).toBe('100%');
  });
});
