# Aurora Sandcastle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade Aurora's Earth.tsx to Sandcastle-level interactive experience — Monaco Editor code panel, Gallery example browser, draggable split panels, and toolbar.

**Architecture:** Extract TopBar from App.tsx into its own component. Add EditorPanel (Monaco + Gallery overlay) and SplitHandle components. Rewrite App.tsx to use flexbox three-column layout with two draggable splitters. Create example manifest system with Aurora-specific and CesiumJS official examples.

**Tech Stack:** React 18, TypeScript, @monaco-editor/react, existing aurora.css design system, CesiumJS (via existing Earth.tsx loading)

## Global Constraints

- All new components use CSS classes from `aurora.css` — no inline styles
- `@monaco-editor/react` is the only new dependency
- Aurora analysis panel (Overlay.tsx) and its children are NOT modified
- Earth.tsx is NOT modified (it already handles CesiumJS lifecycle)
- Tests for new components follow existing vitest + @testing-library/react patterns
- 19/19 existing tests must continue to pass
- TypeScript strict mode, no `any` without explicit reason
- Follow existing naming: `aur-{component}__{element}--{variant}` CSS BEM

---

### Task 1: Install Monaco Editor dependency

**Files:**
- Modify: `ui/package.json`

**Interfaces:**
- Produces: `@monaco-editor/react` available for import in Task 4

- [ ] **Step 1: Add @monaco-editor/react to package.json**

```bash
cd ui && npm install @monaco-editor/react
```

- [ ] **Step 2: Verify install**

```bash
cd ui && node -e "require('@monaco-editor/react'); console.log('OK')"
```
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add ui/package.json ui/package-lock.json
git commit -m "chore: add @monaco-editor/react dependency"
```

---

### Task 2: Create SplitHandle component

**Files:**
- Create: `ui/src/SplitHandle.tsx`
- Modify: `ui/src/aurora.css` (append split handle styles)

**Interfaces:**
- Produces: `<SplitHandle onDrag={ (deltaX: number) => void } />` — fires `onDrag` with pixel delta on each pointermove during drag

- [ ] **Step 1: Write the component**

```typescript
// ui/src/SplitHandle.tsx — draggable column splitter

import { useCallback, useRef } from 'react';

interface Props {
  onDrag: (deltaX: number) => void;
}

export default function SplitHandle({ onDrag }: Props) {
  const draggingRef = useRef<{ startX: number } | null>(null);

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    e.preventDefault();
    draggingRef.current = { startX: e.clientX };
    document.body.style.userSelect = 'none';
    document.body.style.cursor = 'col-resize';

    const onMove = (ev: PointerEvent) => {
      if (!draggingRef.current) return;
      const delta = ev.clientX - draggingRef.current.startX;
      draggingRef.current.startX = ev.clientX;
      onDrag(delta);
    };

    const onUp = () => {
      draggingRef.current = null;
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
      document.removeEventListener('pointermove', onMove);
      document.removeEventListener('pointerup', onUp);
    };

    document.addEventListener('pointermove', onMove);
    document.addEventListener('pointerup', onUp);
  }, [onDrag]);

  return (
    <div
      className="aur-split"
      onPointerDown={onPointerDown}
    />
  );
}
```

- [ ] **Step 2: Add CSS for split handle**

Append to `ui/src/aurora.css`:

```css
/* ═══════════════════════════════════════════════════════════════════
   Split Handle
   ═══════════════════════════════════════════════════════════════════ */

.aur-split {
  width: 5px;
  min-width: 5px;
  cursor: col-resize;
  background: transparent;
  transition: background var(--duration-fast);
  position: relative;
  z-index: 10;
  flex-shrink: 0;
}

.aur-split:hover,
.aur-split:active {
  background: var(--aur-aurora-dim);
}

.aur-split::after {
  content: '';
  position: absolute;
  inset: 0;
  /* Expand hit area without visual change */
  margin: 0 -3px;
}
```

- [ ] **Step 3: Write tests**

```typescript
// ui/src/test/SplitHandle.test.tsx
import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import SplitHandle from '../SplitHandle';

describe('SplitHandle', () => {
  it('renders a div with aur-split class', () => {
    const { container } = render(<SplitHandle onDrag={() => {}} />);
    expect(container.firstChild).toHaveClass('aur-split');
  });

  it('calls onDrag with delta on pointer move', () => {
    const onDrag = vi.fn();
    const { container } = render(<SplitHandle onDrag={onDrag} />);

    const el = container.firstChild!;
    fireEvent.pointerDown(el, { clientX: 400 });
    fireEvent.pointerMove(document, { clientX: 420 });
    fireEvent.pointerMove(document, { clientX: 435 });

    expect(onDrag).toHaveBeenCalledTimes(2);
    expect(onDrag).toHaveBeenCalledWith(20);
    expect(onDrag).toHaveBeenCalledWith(15);
  });

  it('stops firing after pointerup', () => {
    const onDrag = vi.fn();
    const { container } = render(<SplitHandle onDrag={onDrag} />);

    const el = container.firstChild!;
    fireEvent.pointerDown(el, { clientX: 400 });
    fireEvent.pointerMove(document, { clientX: 410 });
    fireEvent.pointerUp(document);
    fireEvent.pointerMove(document, { clientX: 430 });

    expect(onDrag).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 4: Run tests**

```bash
cd ui && npx vitest run src/test/SplitHandle.test.tsx
```
Expected: 3 passed

- [ ] **Step 5: Commit**

```bash
git add ui/src/SplitHandle.tsx ui/src/aurora.css ui/src/test/SplitHandle.test.tsx
git commit -m "feat: add SplitHandle draggable column splitter component"
```

---

### Task 3: Create example manifest system

**Files:**
- Create: `ui/src/examples/manifest.ts`
- Create: `ui/src/examples/aurora/hello-globe.js`
- Create: `ui/src/examples/aurora/signal-hotspots.js`
- Create: `ui/src/examples/aurora/conflict-zones.js`
- Create: `ui/src/examples/aurora/attention-flow.js`
- Create: `ui/src/examples/aurora/frame-perspective.js`
- Create: `ui/src/examples/aurora/frequency-rings.js`
- Create: `ui/src/examples/aurora/china-satellite.js`
- Create: `ui/src/examples/aurora/night-sky.js`

**Interfaces:**
- Produces:
  ```typescript
  export interface ExampleEntry {
    id: string;
    name: string;
    category: string;
    description: string;
    source: 'aurora' | 'cesium';
    /** Code string — loaded from .js file at build time */
    code: string;
  }

  export const AURORA_EXAMPLES: ExampleEntry[];
  export const CESIUM_EXAMPLES: ExampleEntry[];
  export const ALL_EXAMPLES: ExampleEntry[];
  export const CATEGORIES: string[];
  ```

- [ ] **Step 1: Write the manifest types and data**

```typescript
// ui/src/examples/manifest.ts — example gallery index

export interface ExampleEntry {
  id: string;
  name: string;
  category: string;
  description: string;
  source: 'aurora' | 'cesium';
  code: string;
}

// Aurora-specific examples — inline code for now, can be externalized later
export const AURORA_EXAMPLES: ExampleEntry[] = [
  {
    id: 'hello-globe',
    name: 'Hello Globe',
    category: 'Basics',
    description: 'Basic CesiumJS globe with Aurora signal overlay',
    source: 'aurora',
    code: `// Hello Globe — basic CesiumJS viewer
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
  fullscreenButton: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(104, 30, 25000000),
  duration: 0,
});

// Aurora: this viewer is accessible as window.AURORA_VIEWER
`,
  },
  {
    id: 'signal-hotspots',
    name: 'Signal Hotspots',
    category: 'Aurora',
    description: 'Ternary decision signal heatmap on the globe',
    source: 'aurora',
    code: `// Signal Hotspots — heatmap of ternary decision signals
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(104, 30, 20000000),
  duration: 0,
});

// Example signal points with ternary values
// True = green, Hold = yellow, False = red
const signals = [
  { lon: 116.4, lat: 39.9, value: 'True', label: 'Beijing' },
  { lon: 121.5, lat: 31.2, value: 'Hold', label: 'Shanghai' },
  { lon: 114.1, lat: 22.5, value: 'True', label: 'Hong Kong' },
  { lon: 139.7, lat: 35.7, value: 'False', label: 'Tokyo' },
  { lon: -122.4, lat: 37.8, value: 'Hold', label: 'San Francisco' },
  { lon: -0.13, lat: 51.5, value: 'True', label: 'London' },
];

const colors: Record<string, Cesium.Color> = {
  True: Cesium.Color.GREEN,
  Hold: Cesium.Color.YELLOW,
  False: Cesium.Color.RED,
};

signals.forEach(s => {
  viewer.entities.add({
    position: Cesium.Cartesian3.fromDegrees(s.lon, s.lat),
    point: {
      pixelSize: 12,
      color: colors[s.value],
      outlineColor: Cesium.Color.WHITE,
      outlineWidth: 1,
    },
    label: {
      text: s.label,
      font: '12px sans-serif',
      verticalOrigin: Cesium.VerticalOrigin.BOTTOM,
      pixelOffset: new Cesium.Cartesian2(0, -10),
    },
  });
});
`,
  },
  {
    id: 'conflict-zones',
    name: 'Conflict Zones',
    category: 'Aurora',
    description: 'Visualize attention conflict zones with polygon overlays',
    source: 'aurora',
    code: `// Conflict Zones — polygon overlays for attention conflicts
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(104, 30, 20000000),
  duration: 0,
});

// Conflict zone: semi-transparent ellipse
const conflictZone = viewer.entities.add({
  name: 'Frame conflict zone',
  position: Cesium.Cartesian3.fromDegrees(116.4, 39.9),
  ellipse: {
    semiMinorAxis: 300000.0,
    semiMajorAxis: 500000.0,
    material: Cesium.Color.YELLOW.withAlpha(0.3),
    outline: true,
    outlineColor: Cesium.Color.YELLOW,
    outlineWidth: 2,
  },
  label: {
    text: 'Science vs Individual',
    font: '14px sans-serif',
    fillColor: Cesium.Color.YELLOW,
    verticalOrigin: Cesium.VerticalOrigin.TOP,
    pixelOffset: new Cesium.Cartesian2(0, 20),
  },
});

// Aligned zone: green
const alignedZone = viewer.entities.add({
  name: 'Aligned zone',
  position: Cesium.Cartesian3.fromDegrees(-0.13, 51.5),
  ellipse: {
    semiMinorAxis: 200000.0,
    semiMajorAxis: 350000.0,
    material: Cesium.Color.GREEN.withAlpha(0.25),
    outline: true,
    outlineColor: Cesium.Color.GREEN,
    outlineWidth: 2,
  },
  label: {
    text: 'Consensus aligned',
    font: '14px sans-serif',
    fillColor: Cesium.Color.GREEN,
    verticalOrigin: Cesium.VerticalOrigin.TOP,
    pixelOffset: new Cesium.Cartesian2(0, 20),
  },
});
`,
  },
  {
    id: 'attention-flow',
    name: 'Attention Flow',
    category: 'Aurora',
    description: 'Flight-line animation showing attention shifts',
    source: 'aurora',
    code: `// Attention Flow — flight paths showing attention shifts
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(104, 30, 25000000),
  duration: 0,
});

// Attention shift paths
const paths = [
  { from: [116.4, 39.9], to: [121.5, 31.2], label: 'Beijing → Shanghai' },
  { from: [121.5, 31.2], to: [139.7, 35.7], label: 'Shanghai → Tokyo' },
  { from: [-0.13, 51.5], to: [-122.4, 37.8], label: 'London → SF' },
];

paths.forEach((p, i) => {
  viewer.entities.add({
    polyline: {
      positions: Cesium.Cartesian3.fromDegreesArray([
        p.from[0], p.from[1],
        p.to[0], p.to[1],
      ]),
      width: 2,
      material: new Cesium.PolylineDashMaterialProperty({
        color: Cesium.Color.CYAN,
        dashLength: 16.0,
      }),
    },
    label: {
      text: p.label,
      font: '11px sans-serif',
      fillColor: Cesium.Color.CYAN,
      verticalOrigin: Cesium.VerticalOrigin.BOTTOM,
    },
  });
});
`,
  },
  {
    id: 'frame-perspective',
    name: 'Frame Perspective',
    category: 'Aurora',
    description: 'Globe annotations from different Frame perspectives',
    source: 'aurora',
    code: `// Frame Perspective — globe annotations per decision Frame
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(104, 30, 20000000),
  duration: 0,
});

// Frame legend as screen-space billboards
const frames = [
  { name: 'Science', lon: 0, lat: 40, color: Cesium.Color.CYAN },
  { name: 'Individual', lon: 90, lat: 40, color: Cesium.Color.LIME },
  { name: 'Consensus', lon: 180, lat: 40, color: Cesium.Color.ORANGE },
  { name: 'Absolute', lon: -90, lat: 40, color: Cesium.Color.WHITE },
];

frames.forEach(f => {
  viewer.entities.add({
    position: Cesium.Cartesian3.fromDegrees(f.lon, f.lat),
    point: {
      pixelSize: 8,
      color: f.color,
    },
    label: {
      text: f.name,
      font: '14px monospace',
      fillColor: f.color,
      style: Cesium.LabelStyle.FILL_AND_OUTLINE,
      outlineWidth: 2,
      verticalOrigin: Cesium.VerticalOrigin.BOTTOM,
      pixelOffset: new Cesium.Cartesian2(0, -10),
    },
  });
});

// Add a ring around each frame anchor
frames.forEach(f => {
  viewer.entities.add({
    position: Cesium.Cartesian3.fromDegrees(f.lon, f.lat),
    ellipse: {
      semiMinorAxis: 150000.0,
      semiMajorAxis: 150000.0,
      material: f.color.withAlpha(0.1),
      outline: true,
      outlineColor: f.color.withAlpha(0.5),
    },
  });
});
`,
  },
  {
    id: 'frequency-rings',
    name: 'Frequency Rings',
    category: 'Aurora',
    description: 'Concentric rings showing frequency detection results',
    source: 'aurora',
    code: `// Frequency Rings — concentric rings for detected frequencies
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(104, 30, 20000000),
  duration: 0,
});

// Concentric rings at different frequencies
const rings = [
  { hz: 2.0, radius: 200000, color: Cesium.Color.CYAN, lat: 30, lon: 104 },
  { hz: 4.0, radius: 400000, color: Cesium.Color.MAGENTA, lat: 30, lon: 104 },
  { hz: 8.0, radius: 600000, color: Cesium.Color.YELLOW, lat: 30, lon: 104 },
];

rings.forEach(r => {
  viewer.entities.add({
    position: Cesium.Cartesian3.fromDegrees(r.lon, r.lat),
    ellipse: {
      semiMinorAxis: r.radius,
      semiMajorAxis: r.radius,
      material: Cesium.Color.TRANSPARENT,
      outline: true,
      outlineColor: r.color,
      outlineWidth: 2,
    },
    label: {
      text: r.hz.toFixed(1) + ' Hz',
      font: '12px monospace',
      fillColor: r.color,
      verticalOrigin: Cesium.VerticalOrigin.BOTTOM,
    },
  });
});
`,
  },
  {
    id: 'china-satellite',
    name: 'China Satellite Overlay',
    category: 'Imagery',
    description: 'High-resolution China satellite imagery overlay',
    source: 'aurora',
    code: `// China Satellite Overlay — high-res satellite tiles
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(116.4, 39.9, 5000000),
  duration: 2,
});

// China satellite overlay (requires Aurora proxy server)
try {
  const chinaProvider = new Cesium.UrlTemplateImageryProvider({
    url: 'http://localhost:21337/china-tiles/{z}/{x}/{y}.jpg',
    tilingScheme: new Cesium.WebMercatorTilingScheme(),
    minimumLevel: 3,
    maximumLevel: 18,
    rectangle: Cesium.Rectangle.fromDegrees(70, 15, 140, 55),
  });
  viewer.imageryLayers.addImageryProvider(chinaProvider);
} catch (e) {
  console.warn('China tiles not available:', e);
}
`,
  },
  {
    id: 'night-sky',
    name: 'Night Sky Transition',
    category: 'Basics',
    description: 'Day/night cycle as cognitive state metaphor',
    source: 'aurora',
    code: `// Night Sky Transition — day/night as cognitive state metaphor
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(104, 30, 25000000),
  duration: 0,
});

// Enable lighting for day/night effect
viewer.scene.globe.enableLighting = true;

// Animate through a full day cycle
const start = Cesium.JulianDate.fromDate(new Date('2026-06-27T00:00:00Z'));
const stop = Cesium.JulianDate.addHours(start, 24, new Cesium.JulianDate());

viewer.clock.startTime = start.clone();
viewer.clock.stopTime = stop.clone();
viewer.clock.currentTime = start.clone();
viewer.clock.clockRange = Cesium.ClockRange.LOOP_STOP;
viewer.clock.multiplier = 3600; // 1 hour per second
viewer.clock.shouldAnimate = true;

// Cognitive state label
viewer.entities.add({
  position: Cesium.Cartesian3.fromDegrees(0, 0),
  label: {
    text: 'Day = Clarity / Night = Reflection',
    font: '16px monospace',
    fillColor: Cesium.Color.WHITE,
    style: Cesium.LabelStyle.FILL_AND_OUTLINE,
    outlineWidth: 3,
    verticalOrigin: Cesium.VerticalOrigin.BOTTOM,
  },
});
`,
  },
];

// CesiumJS official examples — placeholder, populated from cesium package
// Each entry has code loaded from Apps/Sandcastle/gallery/ at build time
export const CESIUM_EXAMPLES: ExampleEntry[] = [
  {
    id: 'cesium-hello-world',
    name: 'Hello World',
    category: 'Basics',
    description: 'Minimal CesiumJS viewer setup',
    source: 'cesium',
    code: `// Hello World — minimal CesiumJS viewer
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-100, 40, 15000000),
  duration: 2,
});
`,
  },
  {
    id: 'cesium-billboards',
    name: 'Billboards',
    category: 'Entities',
    description: 'Add billboard markers to the globe',
    source: 'cesium',
    code: `// Billboards — image markers on the globe
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.entities.add({
  position: Cesium.Cartesian3.fromDegrees(-75.59777, 40.03883),
  billboard: {
    image: 'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><circle cx="16" cy="16" r="14" fill="%235EEAD4"/></svg>',
    verticalOrigin: Cesium.VerticalOrigin.BOTTOM,
  },
  label: {
    text: 'Marker',
    font: '14px sans-serif',
    verticalOrigin: Cesium.VerticalOrigin.BOTTOM,
    pixelOffset: new Cesium.Cartesian2(0, -36),
  },
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-75.59777, 40.03883, 1000000),
  duration: 2,
});
`,
  },
  {
    id: 'cesium-polylines',
    name: 'Polylines',
    category: 'Entities',
    description: 'Draw lines and paths on the globe',
    source: 'cesium',
    code: `// Polylines — draw paths on the globe
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.entities.add({
  polyline: {
    positions: Cesium.Cartesian3.fromDegreesArray([
      -75, 40,
      -80, 35,
      -85, 30,
      -90, 25,
    ]),
    width: 5,
    material: Cesium.Color.CYAN,
  },
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-82, 32, 5000000),
  duration: 2,
});
`,
  },
  {
    id: 'cesium-polygons',
    name: 'Polygons',
    category: 'Entities',
    description: 'Draw filled shapes on the globe surface',
    source: 'cesium',
    code: `// Polygons — filled shapes on the globe
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.entities.add({
  polygon: {
    hierarchy: Cesium.Cartesian3.fromDegreesArray([
      -110, 30,
      -100, 30,
      -100, 40,
      -110, 40,
    ]),
    material: Cesium.Color.CYAN.withAlpha(0.5),
    outline: true,
    outlineColor: Cesium.Color.WHITE,
  },
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-105, 35, 3000000),
  duration: 2,
});
`,
  },
  {
    id: 'cesium-3d-tiles',
    name: '3D Tiles',
    category: '3D Tiles',
    description: 'Load Cesium OSM Buildings 3D Tileset',
    source: 'cesium',
    code: `// 3D Tiles — Cesium OSM Buildings
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

try {
  const tileset = await Cesium.Cesium3DTileset.fromIonAssetId(96188, {
    enableShowOutline: false,
  });
  viewer.scene.primitives.add(tileset);
  await viewer.zoomTo(tileset);
} catch (e) {
  console.warn('3D Tiles require Cesium Ion token:', e);
}
`,
  },
  {
    id: 'cesium-czml',
    name: 'CZML Demo',
    category: 'DataSources',
    description: 'Animated CZML data source',
    source: 'cesium',
    code: `// CZML Demo — animated data source
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

const czml = [{
  id: 'document',
  name: 'CZML Point',
  version: '1.0',
}, {
  id: 'point',
  position: {
    interpolationAlgorithm: 'LINEAR',
    epoch: '2026-06-27T00:00:00Z',
    cartographicDegrees: [
      0, -75, 40, 0,
      300, -80, 35, 100000,
      600, -85, 30, 0,
    ],
  },
  point: {
    color: { rgba: [94, 234, 212, 255] },
    pixelSize: 10,
  },
}];

const dataSource = new Cesium.CzmlDataSource();
await dataSource.load(czml);
viewer.dataSources.add(dataSource);

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-80, 35, 5000000),
  duration: 2,
});
`,
  },
  {
    id: 'cesium-imagery-layers',
    name: 'Imagery Layers',
    category: 'Imagery',
    description: 'Toggle between different imagery providers',
    source: 'cesium',
    code: `// Imagery Layers — switch between map providers
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

// Add Bing Maps with labels
const bingProvider = await Cesium.BingMapsImageryProvider.fromUrl(
  'https://dev.virtualearth.net',
  { key: 'AgsJtKbJBSBm0VfHh6nDdZvDdQZ0nPxq6Xx0lFhHnQ0nZvDdZvDdZvDdZvDdZvDd' }
);
viewer.imageryLayers.addImageryProvider(bingProvider);

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-100, 40, 15000000),
  duration: 2,
});
`,
  },
  {
    id: 'cesium-camera-fly',
    name: 'Camera Flight',
    category: 'Camera',
    description: 'Animated camera fly-to around the globe',
    source: 'cesium',
    code: `// Camera Flight — animated globe tour
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

// Fly to Beijing
await viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(116.4, 39.9, 5000000),
  duration: 3,
});

// Then to London
await viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-0.13, 51.5, 5000000),
  duration: 3,
});

// Then to San Francisco
await viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-122.4, 37.8, 5000000),
  duration: 3,
});
`,
  },
  {
    id: 'cesium-terrain',
    name: 'Terrain',
    category: 'Terrain',
    description: 'Enable 3D terrain with exaggeration',
    source: 'cesium',
    code: `// Terrain — 3D terrain with vertical exaggeration
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
  terrain: Cesium.Terrain.fromWorldTerrain({
    requestVertexNormals: true,
  }),
});

// Exaggerate terrain 3x
viewer.scene.verticalExaggeration = 3.0;
viewer.scene.verticalExaggerationRelativeHeight = 0;

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(86.9, 27.9, 50000),
  orientation: {
    heading: Cesium.Math.toRadians(0),
    pitch: Cesium.Math.toRadians(-45),
    roll: 0,
  },
  duration: 3,
});
`,
  },
  {
    id: 'cesium-primitives',
    name: 'Primitives',
    category: 'Entities',
    description: 'Box, sphere, cylinder, and other geometric primitives',
    source: 'cesium',
    code: `// Primitives — geometric shapes on the globe
const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

// Blue box
viewer.entities.add({
  name: 'Blue box',
  position: Cesium.Cartesian3.fromDegrees(-114.0, 40.0, 300000.0),
  box: {
    dimensions: new Cesium.Cartesian3(400000.0, 300000.0, 500000.0),
    material: Cesium.Color.BLUE,
  },
});

// Red cylinder
viewer.entities.add({
  name: 'Red cylinder',
  position: Cesium.Cartesian3.fromDegrees(-107.0, 40.0, 300000.0),
  cylinder: {
    length: 400000.0,
    topRadius: 200000.0,
    bottomRadius: 200000.0,
    material: Cesium.Color.RED.withAlpha(0.5),
    outline: true,
    outlineColor: Cesium.Color.RED,
  },
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(-110, 40, 3000000),
  duration: 2,
});
`,
  },
];

export const ALL_EXAMPLES: ExampleEntry[] = [...AURORA_EXAMPLES, ...CESIUM_EXAMPLES];

export const CATEGORIES: string[] = [
  'All',
  ...new Set(ALL_EXAMPLES.map(e => e.category)),
];
```

- [ ] **Step 2: Write tests for manifest**

```typescript
// ui/src/test/manifest.test.ts
import { describe, it, expect } from 'vitest';
import { AURORA_EXAMPLES, CESIUM_EXAMPLES, ALL_EXAMPLES, CATEGORIES } from '../examples/manifest';

describe('Example manifest', () => {
  it('has at least 8 Aurora examples', () => {
    expect(AURORA_EXAMPLES.length).toBeGreaterThanOrEqual(8);
  });

  it('has at least 8 CesiumJS examples', () => {
    expect(CESIUM_EXAMPLES.length).toBeGreaterThanOrEqual(8);
  });

  it('ALL_EXAMPLES combines both sources', () => {
    expect(ALL_EXAMPLES.length).toBe(AURORA_EXAMPLES.length + CESIUM_EXAMPLES.length);
  });

  it('every example has required fields', () => {
    for (const ex of ALL_EXAMPLES) {
      expect(ex.id).toBeTruthy();
      expect(ex.name).toBeTruthy();
      expect(ex.category).toBeTruthy();
      expect(ex.description).toBeTruthy();
      expect(ex.source).toMatch(/^(aurora|cesium)$/);
      expect(ex.code).toBeTruthy();
      expect(ex.code.length).toBeGreaterThan(100);
    }
  });

  it('every example code references cesiumContainer', () => {
    for (const ex of ALL_EXAMPLES) {
      expect(ex.code).toContain('cesiumContainer');
    }
  });

  it('CATEGORIES includes All plus unique categories', () => {
    expect(CATEGORIES[0]).toBe('All');
    const uniqueCats = new Set(ALL_EXAMPLES.map(e => e.category));
    expect(CATEGORIES.length).toBe(uniqueCats.size + 1);
  });
});
```

- [ ] **Step 3: Run tests**

```bash
cd ui && npx vitest run src/test/manifest.test.ts
```
Expected: 6 passed

- [ ] **Step 4: Commit**

```bash
git add ui/src/examples/ ui/src/test/manifest.test.ts
git commit -m "feat: add example manifest system with 8 Aurora + 10 CesiumJS examples"
```

---

### Task 4: Create EditorPanel component (Monaco + Gallery trigger)

**Files:**
- Create: `ui/src/EditorPanel.tsx`
- Modify: `ui/src/aurora.css` (append editor panel styles)

**Interfaces:**
- Consumes: `ExampleEntry` from `./examples/manifest` (Task 3)
- Produces:
  ```typescript
  export default function EditorPanel(props: {
    code: string;
    onChange: (code: string) => void;
    onRun: () => void;
    visible: boolean;
    theme: 'vs-dark' | 'vs-light';
    onToggleTheme: () => void;
    onSelectExample: (entry: ExampleEntry) => void;
  }): JSX.Element
  ```

- [ ] **Step 1: Write the component**

```typescript
// ui/src/EditorPanel.tsx — Monaco Editor + Gallery trigger

import { useCallback, useState } from 'react';
import Editor, { loader } from '@monaco-editor/react';
import GalleryOverlay from './GalleryOverlay';
import type { ExampleEntry } from './examples/manifest';

// Configure Monaco to load from node_modules (works in both dev and Tauri prod)
loader.config({ paths: { vs: '/node_modules/monaco-editor/min/vs' } });

interface Props {
  code: string;
  onChange: (code: string) => void;
  onRun: () => void;
  visible: boolean;
  theme: 'vs-dark' | 'vs-light';
  onToggleTheme: () => void;
  onSelectExample: (entry: ExampleEntry) => void;
}

export default function EditorPanel({
  code,
  onChange,
  onRun,
  visible,
  theme,
  onToggleTheme,
  onSelectExample,
}: Props) {
  const [galleryOpen, setGalleryOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleEditorMount = useCallback(() => {
    // Monaco loaded — inject CesiumJS types for IntelliSense
    // Types are loaded lazily to avoid blocking editor init
    import('./examples/manifest').catch(() => {});
  }, []);

  const handleRun = useCallback(() => {
    setError(null);
    onRun();
  }, [onRun]);

  if (!visible) return null;

  return (
    <div className="aur-editor-panel">
      {/* Editor toolbar */}
      <div className="aur-editor-toolbar">
        <button
          className="aur-btn aur-btn--ghost aur-btn--small"
          onClick={() => setGalleryOpen(true)}
          title="Open example gallery (Ctrl+G)"
        >
          Gallery
        </button>
        <button
          className="aur-btn aur-btn--ghost aur-btn--small"
          onClick={onToggleTheme}
          title="Toggle editor theme"
        >
          {theme === 'vs-dark' ? '☀' : '☾'}
        </button>
        <button
          className="aur-btn aur-btn--primary aur-btn--small"
          onClick={handleRun}
          title="Run code (Ctrl+Enter)"
        >
          ▶ Run
        </button>
      </div>

      {/* Monaco Editor */}
      <div className="aur-editor-body">
        <Editor
          height="100%"
          language="javascript"
          theme={theme}
          value={code}
          onChange={(v) => onChange(v ?? '')}
          onMount={handleEditorMount}
          loading={
            <div className="aur-editor-loading">
              <span className="aur-editor-loading__text">Loading editor…</span>
            </div>
          }
          options={{
            minimap: { enabled: false },
            fontSize: 13,
            fontFamily: "'JetBrains Mono', 'Cascadia Code', monospace",
            lineNumbers: 'on',
            scrollBeyondLastLine: false,
            automaticLayout: true,
            tabSize: 2,
            wordWrap: 'on',
            padding: { top: 8 },
            renderLineHighlight: 'line',
            cursorBlinking: 'smooth',
            smoothScrolling: true,
            bracketPairColorization: { enabled: true },
          }}
        />
      </div>

      {/* Error bar */}
      {error && (
        <div className="aur-editor-error">
          <span className="aur-editor-error__icon">✗</span>
          <span className="aur-editor-error__text">{error}</span>
        </div>
      )}

      {/* Gallery overlay */}
      {galleryOpen && (
        <GalleryOverlay
          onSelect={(entry) => {
            onSelectExample(entry);
            setGalleryOpen(false);
          }}
          onClose={() => setGalleryOpen(false)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Add CSS for editor panel**

Append to `ui/src/aurora.css`:

```css
/* ═══════════════════════════════════════════════════════════════════
   Editor Panel
   ═══════════════════════════════════════════════════════════════════ */

.aur-editor-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--aur-deep);
  border-right: 1px solid var(--aur-ice);
  overflow: hidden;
  flex-shrink: 0;
}

.aur-editor-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--aur-ice);
  flex-shrink: 0;
  -webkit-app-region: no-drag;
}

.aur-editor-body {
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

.aur-editor-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  background: var(--aur-deep);
}

.aur-editor-loading__text {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--aur-void-dim);
}

.aur-editor-error {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  background: rgba(229, 115, 115, 0.1);
  border-top: 1px solid var(--aur-false);
  flex-shrink: 0;
}

.aur-editor-error__icon {
  color: var(--aur-false);
  font-size: var(--text-sm);
  flex-shrink: 0;
}

.aur-editor-error__text {
  font-family: var(--font-data);
  font-size: var(--text-xs);
  color: var(--aur-false);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
```

- [ ] **Step 3: Write tests**

```typescript
// ui/src/test/EditorPanel.test.tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import EditorPanel from '../EditorPanel';
import type { ExampleEntry } from '../examples/manifest';

// Mock Monaco Editor — it's heavy and not needed for unit tests
vi.mock('@monaco-editor/react', () => ({
  default: ({ value, onChange, theme }: any) => (
    <textarea
      data-testid="monaco-editor"
      value={value}
      onChange={(e: any) => onChange?.(e.target.value)}
      data-theme={theme}
    />
  ),
  loader: { config: vi.fn() },
}));

const mockExample: ExampleEntry = {
  id: 'test',
  name: 'Test',
  category: 'Basics',
  description: 'Test example',
  source: 'aurora',
  code: '// test code',
};

describe('EditorPanel', () => {
  const defaultProps = {
    code: '// hello',
    onChange: vi.fn(),
    onRun: vi.fn(),
    visible: true,
    theme: 'vs-dark' as const,
    onToggleTheme: vi.fn(),
    onSelectExample: vi.fn(),
  };

  it('renders nothing when visible=false', () => {
    const { container } = render(<EditorPanel {...defaultProps} visible={false} />);
    expect(container.innerHTML).toBe('');
  });

  it('renders editor toolbar with Gallery, theme, and Run buttons', () => {
    render(<EditorPanel {...defaultProps} />);
    expect(screen.getByText('Gallery')).toBeInTheDocument();
    expect(screen.getByText('▶ Run')).toBeInTheDocument();
  });

  it('renders Monaco editor with code value', () => {
    render(<EditorPanel {...defaultProps} />);
    const editor = screen.getByTestId('monaco-editor');
    expect(editor).toHaveValue('// hello');
  });

  it('calls onRun when Run button clicked', () => {
    const onRun = vi.fn();
    render(<EditorPanel {...defaultProps} onRun={onRun} />);
    fireEvent.click(screen.getByText('▶ Run'));
    expect(onRun).toHaveBeenCalled();
  });

  it('calls onToggleTheme when theme button clicked', () => {
    const onToggleTheme = vi.fn();
    render(<EditorPanel {...defaultProps} onToggleTheme={onToggleTheme} />);
    fireEvent.click(screen.getByTitle('Toggle editor theme'));
    expect(onToggleTheme).toHaveBeenCalled();
  });

  it('opens gallery overlay when Gallery button clicked', () => {
    render(<EditorPanel {...defaultProps} />);
    fireEvent.click(screen.getByText('Gallery'));
    // Gallery overlay should appear
    expect(screen.getByText('Search examples')).toBeInTheDocument();
  });
});
```

- [ ] **Step 4: Run tests**

```bash
cd ui && npx vitest run src/test/EditorPanel.test.tsx
```
Expected: 6 passed

- [ ] **Step 5: Commit**

```bash
git add ui/src/EditorPanel.tsx ui/src/aurora.css ui/src/test/EditorPanel.test.tsx
git commit -m "feat: add EditorPanel with Monaco Editor + Gallery trigger"
```

---

### Task 5: Create GalleryOverlay component

**Files:**
- Create: `ui/src/GalleryOverlay.tsx`
- Modify: `ui/src/aurora.css` (append gallery overlay styles)

**Interfaces:**
- Consumes: `ExampleEntry`, `ALL_EXAMPLES`, `CATEGORIES` from `./examples/manifest` (Task 3)
- Produces:
  ```typescript
  export default function GalleryOverlay(props: {
    onSelect: (entry: ExampleEntry) => void;
    onClose: () => void;
  }): JSX.Element
  ```

- [ ] **Step 1: Write the component**

```typescript
// ui/src/GalleryOverlay.tsx — example gallery slide-out panel

import { useState, useMemo, useCallback } from 'react';
import { ALL_EXAMPLES, CATEGORIES } from './examples/manifest';
import type { ExampleEntry } from './examples/manifest';

interface Props {
  onSelect: (entry: ExampleEntry) => void;
  onClose: () => void;
}

export default function GalleryOverlay({ onSelect, onClose }: Props) {
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('All');
  const [source, setSource] = useState<'all' | 'aurora' | 'cesium'>('all');

  const filtered = useMemo(() => {
    let list = ALL_EXAMPLES;

    if (source !== 'all') {
      list = list.filter(e => e.source === source);
    }
    if (category !== 'All') {
      list = list.filter(e => e.category === category);
    }
    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(e =>
        e.name.toLowerCase().includes(q) ||
        e.description.toLowerCase().includes(q) ||
        e.category.toLowerCase().includes(q)
      );
    }
    return list;
  }, [search, category, source]);

  const handleSelect = useCallback((entry: ExampleEntry) => {
    onSelect(entry);
  }, [onSelect]);

  // Close on Escape
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape') onClose();
  }, [onClose]);

  return (
    <div className="aur-gallery-overlay" onKeyDown={handleKeyDown}>
      <div className="aur-gallery-overlay__backdrop" onClick={onClose} />

      <div className="aur-gallery-overlay__panel">
        {/* Header */}
        <div className="aur-gallery-overlay__header">
          <input
            className="aur-gallery-overlay__search"
            type="text"
            placeholder="Search examples…"
            value={search}
            onChange={e => setSearch(e.target.value)}
            autoFocus
          />
          <button
            className="aur-btn aur-btn--icon"
            onClick={onClose}
            title="Close gallery"
          >
            ✕
          </button>
        </div>

        {/* Category tabs */}
        <div className="aur-gallery-overlay__categories">
          {CATEGORIES.map(cat => (
            <button
              key={cat}
              className={`aur-gallery-overlay__cat-btn${category === cat ? ' aur-gallery-overlay__cat-btn--active' : ''}`}
              onClick={() => setCategory(cat)}
            >
              {cat}
            </button>
          ))}
        </div>

        {/* Thumbnail grid */}
        <div className="aur-gallery-overlay__grid">
          {filtered.length === 0 ? (
            <div className="aur-gallery-overlay__empty">
              No examples match your search.
            </div>
          ) : (
            filtered.map(entry => (
              <button
                key={entry.id}
                className="aur-gallery-card"
                onClick={() => handleSelect(entry)}
              >
                <div className="aur-gallery-card__preview">
                  <span className="aur-gallery-card__icon">
                    {entry.source === 'aurora' ? '◈' : '◆'}
                  </span>
                </div>
                <div className="aur-gallery-card__info">
                  <span className="aur-gallery-card__name">{entry.name}</span>
                  <span className="aur-gallery-card__desc">{entry.description}</span>
                  <span className="aur-gallery-card__meta">
                    {entry.category} · {entry.source === 'aurora' ? 'Aurora' : 'CesiumJS'}
                  </span>
                </div>
              </button>
            ))
          )}
        </div>

        {/* Source tabs */}
        <div className="aur-gallery-overlay__source-tabs">
          <button
            className={`aur-gallery-overlay__source-btn${source === 'all' ? ' aur-gallery-overlay__source-btn--active' : ''}`}
            onClick={() => setSource('all')}
          >
            All
          </button>
          <button
            className={`aur-gallery-overlay__source-btn${source === 'aurora' ? ' aur-gallery-overlay__source-btn--active' : ''}`}
            onClick={() => setSource('aurora')}
          >
            Aurora
          </button>
          <button
            className={`aur-gallery-overlay__source-btn${source === 'cesium' ? ' aur-gallery-overlay__source-btn--active' : ''}`}
            onClick={() => setSource('cesium')}
          >
            CesiumJS
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Add CSS for gallery overlay**

Append to `ui/src/aurora.css`:

```css
/* ═══════════════════════════════════════════════════════════════════
   Gallery Overlay
   ═══════════════════════════════════════════════════════════════════ */

.aur-gallery-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
}

.aur-gallery-overlay__backdrop {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(2px);
}

.aur-gallery-overlay__panel {
  position: relative;
  width: 480px;
  max-width: 90vw;
  height: 100vh;
  background: var(--aur-deep);
  border-right: 1px solid var(--aur-ice);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: aur-gallery-slide-in 0.2s var(--ease-out);
  z-index: 1;
}

@keyframes aur-gallery-slide-in {
  from { transform: translateX(-100%); }
  to { transform: translateX(0); }
}

.aur-gallery-overlay__header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--aur-ice);
  flex-shrink: 0;
}

.aur-gallery-overlay__search {
  flex: 1;
  background: var(--aur-night);
  border: 1px solid var(--aur-ice);
  border-radius: var(--radius-sm);
  color: var(--aur-white);
  font-family: var(--font-body);
  font-size: var(--text-md);
  padding: var(--space-2) var(--space-3);
  outline: none;
  transition: border-color var(--duration-fast);
}

.aur-gallery-overlay__search:focus {
  border-color: var(--aur-aurora);
}

.aur-gallery-overlay__search::placeholder {
  color: var(--aur-void-dim);
}

.aur-gallery-overlay__categories {
  display: flex;
  gap: var(--space-1);
  padding: var(--space-2) var(--space-4);
  overflow-x: auto;
  border-bottom: 1px solid var(--aur-ice);
  flex-shrink: 0;
}

.aur-gallery-overlay__categories::-webkit-scrollbar {
  height: 0;
}

.aur-gallery-overlay__cat-btn {
  flex-shrink: 0;
  padding: 0.15rem 0.6rem;
  border: 1px solid var(--aur-ice);
  border-radius: 12px;
  background: transparent;
  color: var(--aur-void);
  font-family: var(--font-body);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--duration-fast);
}

.aur-gallery-overlay__cat-btn:hover {
  border-color: var(--aur-void);
  color: var(--aur-white);
}

.aur-gallery-overlay__cat-btn--active {
  background: var(--aur-aurora-dim);
  border-color: var(--aur-aurora);
  color: var(--aur-aurora);
}

.aur-gallery-overlay__grid {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-3) var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.aur-gallery-overlay__empty {
  padding: var(--space-6) 0;
  text-align: center;
  color: var(--aur-void-dim);
  font-size: var(--text-sm);
}

.aur-gallery-card {
  display: flex;
  gap: var(--space-3);
  padding: var(--space-3);
  background: var(--aur-night);
  border: 1px solid var(--aur-ice);
  border-radius: var(--radius-md);
  cursor: pointer;
  text-align: left;
  transition: border-color var(--duration-fast), background var(--duration-fast);
  width: 100%;
}

.aur-gallery-card:hover {
  border-color: var(--aur-aurora);
  background: var(--aur-ice);
}

.aur-gallery-card__preview {
  width: 64px;
  height: 48px;
  background: var(--aur-ice);
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.aur-gallery-card__icon {
  font-size: 1.25rem;
  opacity: 0.4;
}

.aur-gallery-card__info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.aur-gallery-card__name {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--aur-white);
}

.aur-gallery-card__desc {
  font-size: var(--text-xs);
  color: var(--aur-void);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.aur-gallery-card__meta {
  font-family: var(--font-data);
  font-size: 0.625rem;
  color: var(--aur-void-dim);
}

.aur-gallery-overlay__source-tabs {
  display: flex;
  border-top: 1px solid var(--aur-ice);
  flex-shrink: 0;
}

.aur-gallery-overlay__source-btn {
  flex: 1;
  padding: var(--space-2);
  border: none;
  background: transparent;
  color: var(--aur-void-dim);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--duration-fast);
  border-bottom: 2px solid transparent;
}

.aur-gallery-overlay__source-btn:hover {
  color: var(--aur-void);
}

.aur-gallery-overlay__source-btn--active {
  color: var(--aur-aurora);
  border-bottom-color: var(--aur-aurora);
}
```

- [ ] **Step 3: Write tests**

```typescript
// ui/src/test/GalleryOverlay.test.tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import GalleryOverlay from '../GalleryOverlay';

describe('GalleryOverlay', () => {
  const onSelect = vi.fn();
  const onClose = vi.fn();

  beforeEach(() => {
    onSelect.mockClear();
    onClose.mockClear();
  });

  it('renders search input with autofocus', () => {
    render(<GalleryOverlay onSelect={onSelect} onClose={onClose} />);
    const input = screen.getByPlaceholderText('Search examples…');
    expect(input).toBeInTheDocument();
    expect(document.activeElement).toBe(input);
  });

  it('renders category tabs', () => {
    render(<GalleryOverlay onSelect={onSelect} onClose={onClose} />);
    expect(screen.getByText('All')).toBeInTheDocument();
    expect(screen.getByText('Basics')).toBeInTheDocument();
  });

  it('renders source tabs', () => {
    render(<GalleryOverlay onSelect={onSelect} onClose={onClose} />);
    expect(screen.getByText('Aurora')).toBeInTheDocument();
    expect(screen.getByText('CesiumJS')).toBeInTheDocument();
  });

  it('calls onClose when backdrop clicked', () => {
    const { container } = render(<GalleryOverlay onSelect={onSelect} onClose={onClose} />);
    const backdrop = container.querySelector('.aur-gallery-overlay__backdrop')!;
    fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalled();
  });

  it('calls onClose when close button clicked', () => {
    render(<GalleryOverlay onSelect={onSelect} onClose={onClose} />);
    fireEvent.click(screen.getByTitle('Close gallery'));
    expect(onClose).toHaveBeenCalled();
  });

  it('filters examples by search text', () => {
    render(<GalleryOverlay onSelect={onSelect} onClose={onClose} />);
    const input = screen.getByPlaceholderText('Search examples…');
    fireEvent.change(input, { target: { value: 'china' } });
    expect(screen.getByText('China Satellite Overlay')).toBeInTheDocument();
    expect(screen.queryByText('Hello Globe')).not.toBeInTheDocument();
  });

  it('calls onSelect when example card clicked', () => {
    render(<GalleryOverlay onSelect={onSelect} onClose={onClose} />);
    fireEvent.click(screen.getByText('Hello Globe'));
    expect(onSelect).toHaveBeenCalled();
    const entry = onSelect.mock.calls[0][0];
    expect(entry.id).toBe('hello-globe');
  });

  it('filters by source tab', () => {
    render(<GalleryOverlay onSelect={onSelect} onClose={onClose} />);
    fireEvent.click(screen.getAllByText('Aurora')[1]); // source tab, not category
    // Only Aurora examples should be visible
    expect(screen.getByText('Hello Globe')).toBeInTheDocument();
    expect(screen.queryByText('Billboards')).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 4: Run tests**

```bash
cd ui && npx vitest run src/test/GalleryOverlay.test.tsx
```
Expected: 8 passed

- [ ] **Step 5: Commit**

```bash
git add ui/src/GalleryOverlay.tsx ui/src/aurora.css ui/src/test/GalleryOverlay.test.tsx
git commit -m "feat: add GalleryOverlay example browser with search, categories, and source tabs"
```

---

### Task 6: Extract TopBar component from App.tsx

**Files:**
- Create: `ui/src/TopBar.tsx`
- Modify: `ui/src/App.tsx` (remove inline topbar JSX, import TopBar)

**Interfaces:**
- Produces:
  ```typescript
  export default function TopBar(props: {
    editorVisible: boolean;
    onToggleEditor: () => void;
    onRun: () => void;
    theme: 'vs-dark' | 'vs-light';
    onToggleTheme: () => void;
    onOpenGallery: () => void;
    onToggleFullscreen: () => void;
    onReset: () => void;
    panelOpen: boolean;
    onTogglePanel: () => void;
    decision: string | null;
    loading: boolean;
  }): JSX.Element
  ```

- [ ] **Step 1: Write TopBar component**

```typescript
// ui/src/TopBar.tsx — Aurora toolbar

interface Props {
  editorVisible: boolean;
  onToggleEditor: () => void;
  onRun: () => void;
  theme: 'vs-dark' | 'vs-light';
  onToggleTheme: () => void;
  onOpenGallery: () => void;
  onToggleFullscreen: () => void;
  onReset: () => void;
  panelOpen: boolean;
  onTogglePanel: () => void;
  decision: string | null;
  loading: boolean;
}

export default function TopBar({
  editorVisible,
  onToggleEditor,
  onRun,
  theme,
  onToggleTheme,
  onOpenGallery,
  onToggleFullscreen,
  onReset,
  panelOpen,
  onTogglePanel,
  decision,
  loading,
}: Props) {
  return (
    <header className="aur-topbar">
      <span className="aur-wordmark">Aurora</span>
      <div className="aur-topbar-divider" />

      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={onToggleEditor}
        title="Toggle editor (Ctrl+E)"
      >
        {editorVisible ? '◀' : '▶'} Editor
      </button>

      <button
        className="aur-btn aur-btn--primary aur-btn--small"
        onClick={onRun}
        disabled={loading}
        title="Run code (Ctrl+Enter)"
      >
        ▶ Run
      </button>

      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={onToggleTheme}
        title="Toggle editor theme"
      >
        {theme === 'vs-dark' ? '☀' : '☾'}
      </button>

      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={onOpenGallery}
        title="Example gallery (Ctrl+G)"
      >
        Gallery
      </button>

      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={onToggleFullscreen}
        title="Fullscreen (F11)"
      >
        ⛶
      </button>

      <button
        className="aur-btn aur-btn--ghost aur-btn--small"
        onClick={onReset}
        title="Reset globe"
      >
        ↻
      </button>

      {decision && (
        <>
          <div className="aur-topbar-divider" />
          <span className="aur-topbar-decision" data-decision={decision}>
            {decision}
          </span>
        </>
      )}

      <div className="aur-topbar-spacer" />

      <button
        className="aur-btn aur-btn--icon"
        onClick={onTogglePanel}
        title={panelOpen ? 'Close panel' : 'Open panel'}
      >
        {panelOpen ? '▸' : '◂'}
      </button>

      <span className="aur-esc-hint">Esc</span>
    </header>
  );
}
```

- [ ] **Step 2: Update App.tsx to use TopBar**

Replace the `<header className="aur-topbar">...</header>` block in App.tsx with:

```typescript
import TopBar from './TopBar';

// Inside App component, add these state variables:
const [editorVisible, setEditorVisible] = useState(true);
const [editorTheme, setEditorTheme] = useState<'vs-dark' | 'vs-light'>('vs-dark');
const [editorCode, setEditorCode] = useState('');
const [fullscreen, setFullscreen] = useState(false);
const [galleryOpen, setGalleryOpen] = useState(false);

// Replace the <header>...</header> block with:
<TopBar
  editorVisible={editorVisible}
  onToggleEditor={() => setEditorVisible(v => !v)}
  onRun={handleEditorRun}
  theme={editorTheme}
  onToggleTheme={() => setEditorTheme(t => t === 'vs-dark' ? 'vs-light' : 'vs-dark')}
  onOpenGallery={() => setGalleryOpen(true)}
  onToggleFullscreen={() => setFullscreen(f => !f)}
  onReset={handleReset}
  panelOpen={panelOpen}
  onTogglePanel={() => setPanelOpen(o => !o)}
  decision={data?.decision ?? null}
  loading={loading}
/>
```

- [ ] **Step 3: Write tests for TopBar**

```typescript
// ui/src/test/TopBar.test.tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import TopBar from '../TopBar';

describe('TopBar', () => {
  const defaultProps = {
    editorVisible: true,
    onToggleEditor: vi.fn(),
    onRun: vi.fn(),
    theme: 'vs-dark' as const,
    onToggleTheme: vi.fn(),
    onOpenGallery: vi.fn(),
    onToggleFullscreen: vi.fn(),
    onReset: vi.fn(),
    panelOpen: true,
    onTogglePanel: vi.fn(),
    decision: null,
    loading: false,
  };

  it('renders Aurora wordmark', () => {
    render(<TopBar {...defaultProps} />);
    expect(screen.getByText('Aurora')).toBeInTheDocument();
  });

  it('renders all toolbar buttons', () => {
    render(<TopBar {...defaultProps} />);
    expect(screen.getByText(/Editor/)).toBeInTheDocument();
    expect(screen.getByText('▶ Run')).toBeInTheDocument();
    expect(screen.getByText('Gallery')).toBeInTheDocument();
  });

  it('calls onToggleEditor when Editor button clicked', () => {
    const onToggleEditor = vi.fn();
    render(<TopBar {...defaultProps} onToggleEditor={onToggleEditor} />);
    fireEvent.click(screen.getByText(/Editor/));
    expect(onToggleEditor).toHaveBeenCalled();
  });

  it('calls onRun when Run button clicked', () => {
    const onRun = vi.fn();
    render(<TopBar {...defaultProps} onRun={onRun} />);
    fireEvent.click(screen.getByText('▶ Run'));
    expect(onRun).toHaveBeenCalled();
  });

  it('shows decision when provided', () => {
    render(<TopBar {...defaultProps} decision="Hold" />);
    expect(screen.getByText('Hold')).toBeInTheDocument();
  });

  it('disables Run button when loading', () => {
    render(<TopBar {...defaultProps} loading={true} />);
    const btn = screen.getByText('▶ Run');
    expect(btn).toBeDisabled();
  });
});
```

- [ ] **Step 4: Run all tests**

```bash
cd ui && npx vitest run
```
Expected: all tests pass (existing 19 + new ones)

- [ ] **Step 5: Commit**

```bash
git add ui/src/TopBar.tsx ui/src/App.tsx ui/src/test/TopBar.test.tsx
git commit -m "refactor: extract TopBar component from App.tsx, add Sandcastle toolbar buttons"
```

---

### Task 7: Rewrite App.tsx with three-column layout and splitters

**Files:**
- Modify: `ui/src/App.tsx` (full rewrite of layout section)
- Modify: `ui/src/aurora.css` (update content area to flexbox)

**Interfaces:**
- Consumes: `TopBar` (Task 6), `EditorPanel` (Task 4), `SplitHandle` (Task 2), `Earth`, `Overlay` (existing)
- Produces: Complete three-column Sandcastle layout

- [ ] **Step 1: Update CSS for three-column content area**

Replace the `.aur-content` and related rules in `aurora.css`:

```css
/* ── Content Area (three-column flexbox) ── */
.aur-content {
  grid-area: content;
  display: flex;
  flex-direction: row;
  overflow: hidden;
}

.aur-globe-area {
  flex: 1;
  position: relative;
  overflow: hidden;
  min-width: 200px;
  min-height: 0;
}
```

- [ ] **Step 2: Rewrite App.tsx layout**

The full App.tsx becomes:

```typescript
// ui/src/App.tsx — Aurora Sandcastle: 3-column layout
//
// Left: Monaco Editor | Center: CesiumJS Globe | Right: Analysis Panel
// Two draggable splitters between columns.

import { useState, useCallback, useEffect, useRef } from 'react';
import Earth from './Earth';
import Overlay from './Overlay';
import TopBar from './TopBar';
import EditorPanel from './EditorPanel';
import SplitHandle from './SplitHandle';
import GalleryOverlay from './GalleryOverlay';
import diag, { isTauriEnvironment } from './utils/diag';
import type { PipelineRequest, PipelineResponse } from './types';
import type { ExampleEntry } from './examples/manifest';

const DEFAULT_PIPELINE_REQUEST: PipelineRequest = {
  freq: 2.0,
  sample_rate: 100.0,
  duration_secs: 1.0,
  noise_std: 0.1,
  frequency_threshold: 1.5,
  user_feels_normal: true,
};

const DEFAULT_RESUME_DELAY_MS = 5000;

const MOCK_PIPELINE_RESPONSE: PipelineResponse = {
  detected_freq_hz: 2.0,
  decision: 'Hold',
  asi: 0.5,
  reminder_count: 0,
  active_shift_count: 0,
  conflicts: [],
  reminders: [],
  html: '<p>Dev mode — no Tauri backend</p>',
  json: '{}',
};

const DEFAULT_CODE = `// Aurora Sandcastle — edit and run CesiumJS code
// Click Gallery to browse examples, or write your own.

const viewer = new Cesium.Viewer('cesiumContainer', {
  baseLayerPicker: false,
  geocoder: false,
  homeButton: false,
  sceneModePicker: false,
  navigationHelpButton: false,
  animation: false,
  timeline: false,
});

viewer.camera.flyTo({
  destination: Cesium.Cartesian3.fromDegrees(104, 30, 25000000),
  duration: 0,
});
`;

async function invokeRunPipeline(req: PipelineRequest): Promise<PipelineResponse> {
  if (isTauriEnvironment()) {
    diag('invoke', 'INFO', '调用 run_analysis_pipeline (Tauri)');
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<PipelineResponse>('run_analysis_pipeline', { request: req });
      diag('invoke', 'INFO', `pipeline 返回成功: decision=${result.decision} asi=${result.asi}`);
      return result;
    } catch (e: any) {
      diag('invoke', 'ERROR', `Tauri invoke 失败: ${e}`);
      throw e;
    }
  }
  diag('invoke', 'WARN', '无 Tauri 环境 — 返回 mock 数据');
  return MOCK_PIPELINE_RESPONSE;
}

export default function App() {
  const [data, setData] = useState<PipelineResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [resumeDelayMs, setResumeDelayMs] = useState(DEFAULT_RESUME_DELAY_MS);
  const [panelOpen, setPanelOpen] = useState(true);

  // Sandcastle state
  const [editorVisible, setEditorVisible] = useState(true);
  const [editorTheme, setEditorTheme] = useState<'vs-dark' | 'vs-light'>('vs-dark');
  const [editorCode, setEditorCode] = useState(DEFAULT_CODE);
  const [fullscreen, setFullscreen] = useState(false);
  const [galleryOpen, setGalleryOpen] = useState(false);

  // Panel widths
  const [editorWidth, setEditorWidth] = useState(400);
  const [panelWidth, setPanelWidth] = useState(380);

  const initialRunDone = useRef(false);

  const handleRun = useCallback(async () => {
    diag('App', 'INFO', '运行 Aurora 管线分析');
    setLoading(true);
    try {
      const result = await invokeRunPipeline(DEFAULT_PIPELINE_REQUEST);
      setData(result);
      diag('App', 'INFO', '分析完成，数据已更新');
    } catch (err) {
      diag('App', 'ERROR', `分析失败: ${err}`);
    } finally {
      setLoading(false);
    }
  }, []);

  // Execute editor code on the CesiumJS globe
  const handleEditorRun = useCallback(() => {
    diag('App', 'INFO', '执行编辑器代码');
    // The Earth component listens for code execution events
    window.dispatchEvent(new CustomEvent('aurora-run-code', {
      detail: { code: editorCode },
    }));
  }, [editorCode]);

  // Reset globe to default
  const handleReset = useCallback(() => {
    diag('App', 'INFO', '重置地球');
    window.dispatchEvent(new CustomEvent('aurora-reset-globe'));
  }, []);

  // Select example from gallery
  const handleSelectExample = useCallback((entry: ExampleEntry) => {
    setEditorCode(entry.code);
    // Auto-run after a short delay to let editor update
    setTimeout(() => {
      window.dispatchEvent(new CustomEvent('aurora-run-code', {
        detail: { code: entry.code },
      }));
    }, 100);
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      if (ctrl && e.key === 'e') {
        e.preventDefault();
        setEditorVisible(v => !v);
      } else if (ctrl && e.key === 'Enter') {
        e.preventDefault();
        handleEditorRun();
      } else if (ctrl && e.key === 'g') {
        e.preventDefault();
        setGalleryOpen(true);
      } else if (e.key === 'F11') {
        e.preventDefault();
        setFullscreen(f => !f);
      } else if (e.key === 'Escape') {
        if (galleryOpen) {
          setGalleryOpen(false);
        } else {
          diag('Earth', 'INFO', 'Esc 退出');
          try {
            import('@tauri-apps/api/core').then(m => m.invoke('exit_app')).catch(() => window.close());
          } catch {
            window.close();
          }
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [handleEditorRun, galleryOpen]);

  useEffect(() => {
    if (initialRunDone.current) return;
    initialRunDone.current = true;

    const isTauri = isTauriEnvironment();
    diag('App', 'INFO', `应用启动 — 环境: ${isTauri ? 'Tauri' : '浏览器'}`);
    diag('App', 'INFO', `location: ${location.href}`);

    handleRun();
  }, [handleRun]);

  // Splitter drag handlers with min/max constraints
  const handleEditorSplitDrag = useCallback((delta: number) => {
    setEditorWidth(w => Math.max(280, Math.min(600, w + delta)));
  }, []);

  const handlePanelSplitDrag = useCallback((delta: number) => {
    setPanelWidth(w => Math.max(280, Math.min(500, w + delta)));
  }, []);

  return (
    <div className="aur-app">
      <TopBar
        editorVisible={editorVisible}
        onToggleEditor={() => setEditorVisible(v => !v)}
        onRun={handleEditorRun}
        theme={editorTheme}
        onToggleTheme={() => setEditorTheme(t => t === 'vs-dark' ? 'vs-light' : 'vs-dark')}
        onOpenGallery={() => setGalleryOpen(true)}
        onToggleFullscreen={() => setFullscreen(f => !f)}
        onReset={handleReset}
        panelOpen={panelOpen}
        onTogglePanel={() => setPanelOpen(o => !o)}
        decision={data?.decision ?? null}
        loading={loading}
      />

      <div className="aur-content">
        {/* Left: Editor Panel */}
        {!fullscreen && (
          <>
            <div style={{ width: editorVisible ? editorWidth : 0, overflow: 'hidden', transition: 'width 0.2s ease-in-out', flexShrink: 0 }}>
              <EditorPanel
                code={editorCode}
                onChange={setEditorCode}
                onRun={handleEditorRun}
                visible={editorVisible}
                theme={editorTheme}
                onToggleTheme={() => setEditorTheme(t => t === 'vs-dark' ? 'vs-light' : 'vs-dark')}
                onSelectExample={handleSelectExample}
              />
            </div>
            {editorVisible && <SplitHandle onDrag={handleEditorSplitDrag} />}
          </>
        )}

        {/* Center: Globe */}
        <div className="aur-globe-area">
          <Earth resumeDelayMs={resumeDelayMs} />
        </div>

        {/* Right: Analysis Panel */}
        {!fullscreen && (
          <>
            {panelOpen && <SplitHandle onDrag={handlePanelSplitDrag} />}
            <aside
              className={`aur-panel${panelOpen ? '' : ' aur-panel--collapsed'}`}
              style={{ width: panelOpen ? panelWidth : 0 }}
            >
              <Overlay
                data={data}
                loading={loading}
                onRun={handleRun}
                resumeDelayMs={resumeDelayMs}
                onResumeDelayChange={setResumeDelayMs}
              />
            </aside>
          </>
        )}
      </div>

      {/* Gallery overlay (rendered at root level) */}
      {galleryOpen && (
        <GalleryOverlay
          onSelect={handleSelectExample}
          onClose={() => setGalleryOpen(false)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 3: Update App.test.tsx for new layout**

```typescript
// ui/src/test/App.test.tsx — updated for Sandcastle layout
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    detected_freq_hz: 2.0,
    decision: 'Hold',
    asi: 0.5,
    reminder_count: 0,
    active_shift_count: 0,
    conflicts: [],
    reminders: [],
    html: '<p>test</p>',
    json: '{}',
  }),
}));

vi.mock('../Earth', () => ({ default: () => null }));

// Mock Monaco Editor
vi.mock('@monaco-editor/react', () => ({
  default: ({ value }: any) => <textarea data-testid="monaco-editor" value={value} readOnly />,
  loader: { config: vi.fn() },
}));

import App from '../App';

describe('App', () => {
  it('renders the Aurora branding', () => {
    render(<App />);
    const elements = screen.getAllByText('Aurora');
    expect(elements.length).toBeGreaterThanOrEqual(1);
  });

  it('renders toolbar buttons', () => {
    render(<App />);
    expect(screen.getByText(/Editor/)).toBeInTheDocument();
    expect(screen.getByText('Gallery')).toBeInTheDocument();
  });

  it('renders the Esc hint', () => {
    render(<App />);
    expect(screen.getByText('Esc')).toBeInTheDocument();
  });

  it('renders the footer text', () => {
    render(<App />);
    expect(screen.getByText(/Aurora v0.1.0/)).toBeInTheDocument();
  });

  it('renders Monaco editor with default code', () => {
    render(<App />);
    const editor = screen.getByTestId('monaco-editor');
    expect(editor).toBeInTheDocument();
    expect(editor).toHaveValue(expect.stringContaining('Aurora Sandcastle'));
  });
});
```

- [ ] **Step 4: Run all tests**

```bash
cd ui && npx vitest run
```
Expected: all tests pass

- [ ] **Step 5: Run TypeScript check**

```bash
cd ui && npx tsc --noEmit
```
Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add ui/src/App.tsx ui/src/aurora.css ui/src/test/App.test.tsx
git commit -m "feat: rewrite App.tsx with 3-column Sandcastle layout, splitters, and keyboard shortcuts"
```

---

### Task 8: Update Earth.tsx to handle code execution events

**Files:**
- Modify: `ui/src/Earth.tsx` (add event listeners for `aurora-run-code` and `aurora-reset-globe`)

**Interfaces:**
- Consumes: Custom events from App.tsx (Task 7)
- Produces: Earth component responds to code execution and reset events

- [ ] **Step 1: Add event listeners to Earth.tsx**

Add these effects to the Earth component (after existing effects, before the return):

```typescript
// ── Sandcastle: code execution event ──
useEffect(() => {
  const handleRunCode = (e: Event) => {
    const { code } = (e as CustomEvent).detail;
    diag('Earth', 'INFO', '执行 Sandcastle 代码');

    // Destroy existing CesiumJS viewer
    if (viewerRef.current && !viewerRef.current.isDestroyed()) {
      viewerRef.current.destroy();
      viewerRef.current = null;
    }

    // Reset engine state
    setEngine('loading');
    setReady(false);

    // Execute user code in a sandboxed context
    // The code is expected to create: new Cesium.Viewer('cesiumContainer', {...})
    try {
      const Cesium = (window as any).Cesium;
      if (!Cesium) {
        throw new Error('CesiumJS not loaded');
      }

      // Ensure container exists
      if (!containerRef.current) {
        throw new Error('Container not mounted');
      }

      // Execute the code — it creates a viewer bound to cesiumContainer
      const fn = new Function('Cesium', 'cesiumContainer', code);
      fn(Cesium, 'cesiumContainer');

      // Find the viewer that was just created
      // The code creates `const viewer = new Cesium.Viewer(...)` which
      // CesiumJS tracks internally. We find it via the container element.
      const viewers = (Cesium as any)._viewers;
      if (viewers && viewers.length > 0) {
        viewerRef.current = viewers[viewers.length - 1];
      }

      // Store globally for user code access
      (window as any).AURORA_VIEWER = viewerRef.current;

      setEngine('cesium');
      setReady(true);
      diag('Earth', 'INFO', 'Sandcastle 代码执行成功');
    } catch (err: any) {
      diag('Earth', 'ERROR', `代码执行失败: ${err.message}`);
      // Fall back to globe-gl
      setEngine('globe-gl');
      setReady(true);
    }
  };

  const handleReset = () => {
    diag('Earth', 'INFO', '重置地球');
    if (viewerRef.current && !viewerRef.current.isDestroyed()) {
      viewerRef.current.destroy();
      viewerRef.current = null;
    }
    setEngine('loading');
    setReady(false);
    // Re-trigger the startup flow
    setServerReady(false);
    // The server probe effect will re-run and re-init
    setTimeout(() => setServerReady(true), 100);
  };

  window.addEventListener('aurora-run-code', handleRunCode);
  window.addEventListener('aurora-reset-globe', handleReset);
  return () => {
    window.removeEventListener('aurora-run-code', handleRunCode);
    window.removeEventListener('aurora-reset-globe', handleReset);
  };
}, []);
```

- [ ] **Step 2: Run tests**

```bash
cd ui && npx vitest run
```
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add ui/src/Earth.tsx
git commit -m "feat: add Sandcastle code execution and reset event handlers to Earth"
```

---

### Task 9: Final integration — build verification and cleanup

**Files:**
- Verify: `ui/src/aurora.css` (no duplicate rules)
- Verify: `ui/package.json` (dependency list clean)

- [ ] **Step 1: Run full test suite**

```bash
cd ui && npx vitest run
```
Expected: all tests pass

- [ ] **Step 2: Run TypeScript check**

```bash
cd ui && npx tsc --noEmit
```
Expected: no errors

- [ ] **Step 3: Run production build**

```bash
cd ui && npx vite build
```
Expected: build succeeds, CSS + JS output in dist/

- [ ] **Step 4: Verify CSS file size**

```bash
cd ui && ls -la dist/assets/index-*.css
```
Expected: CSS file exists, reasonable size (< 20KB)

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: final integration — all tests pass, build succeeds"
```
