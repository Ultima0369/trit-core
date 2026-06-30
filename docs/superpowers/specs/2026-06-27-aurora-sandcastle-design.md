# Aurora Sandcastle — Design Spec

**Date:** 2026-06-27
**Status:** Approved

## Overview

Upgrade Aurora's `Earth.tsx` to a Sandcastle-level interactive experience: add Monaco Editor code panel, Gallery example browser, and toolbar — while keeping the Aurora analysis panel intact.

## Layout

Three-column horizontal layout with two draggable splitters:

```
┌──────────────────────────────────────────────────────────┐
│  TopBar (42px): Aurora │ ◀▶ Editor │ ▶ Run │ Theme │     │
│  Gallery │ Fullscreen │ Reset │ Esc                      │
├────────────┬────────────────────────┬────────────────────┤
│ Monaco     │ CesiumJS Globe        │ Aurora Panel       │
│ Editor     │ (flex: 1)             │ (380px, collapsible)│
│ (400px,    │                       │                    │
│ collapsible│                       │ Decision / ASI /   │
│            │                       │ Conflicts /        │
│            │                       │ Reminders          │
├────────────┴────────────────────────┴────────────────────┤
│  SplitHandle (editor↔globe)  SplitHandle (globe↔panel)  │
└──────────────────────────────────────────────────────────┘
```

- Editor panel: default 400px, range 280–600px, collapsible
- Globe area: flex: 1, fills remaining space
- Analysis panel: 380px, range 280–500px, collapsible

## TopBar Toolbar

| Button | Action | Shortcut |
|--------|--------|----------|
| ◀▶ Editor | Toggle editor visibility | Ctrl+E |
| ▶ Run | Execute editor code, update globe | Ctrl+Enter |
| Theme | Toggle Monaco light/dark theme | — |
| Gallery | Open example gallery overlay | Ctrl+G |
| Fullscreen | Hide editor + panel, globe only | F11 |
| Reset | Reset globe to default state | — |

Status indicators: current example name, unsaved dot, error count.

## Gallery Overlay

Slide-out panel from left side when Gallery button clicked:

- **Search bar**: real-time filter by name/description/tags
- **Category tabs**: horizontal scroll, click to filter (All, Basics, Imagery, Terrain, Entities, Camera, DataSources, Particles, Analysis, Aurora)
- **Thumbnail grid**: 3-column, each card = thumbnail + name + short description
- **Source tabs**: bottom tabs switch between "Aurora" and "CesiumJS Official" example sets
- Click card → load code into editor → auto-run

### Aurora Examples (8–12 built-in)

- Hello Globe — basic globe + Aurora signal overlay
- Signal Hotspots — ternary decision signal heatmap
- Conflict Zones — attention conflict zone visualization
- Attention Flow — attention shift flight-line animation
- Frame Perspective — globe annotations per Frame
- Frequency Rings — frequency detection ring overlay
- China Satellite Overlay — China satellite imagery
- Night Sky Transition — day/night + cognitive state metaphor

### CesiumJS Official Examples (30+)

Extracted from CesiumJS npm package `Apps/Sandcastle/gallery/` directory.
Organized by official categories. Code auto-adapted to Aurora's CesiumJS loading path.

## Monaco Editor

- Package: `@monaco-editor/react`
- Language: JavaScript
- Theme: `vs-dark` (default), toggle to `vs-light`
- Options: no minimap, fontSize 13, automaticLayout, tabSize 2, wordWrap on
- CesiumJS IntelliSense: inject CesiumJS `.d.ts` via `addExtraLib()`
- Lazy initialization: only load when editor panel first opens

## Code Execution

- Run button → destroy current Viewer → execute code → create new Viewer
- Error handling: try/catch around `new Function()`, errors shown in editor status bar
- Inject `window.AURORA_VIEWER` global for user code access

## Component Tree

```
App.tsx
├── TopBar
│   ├── Aurora wordmark
│   ├── EditorToggle, RunButton, ThemeToggle
│   ├── GalleryButton, FullscreenButton, ResetButton
│   ├── Spacer, PanelToggle, EscHint
├── Content (flex row)
│   ├── EditorPanel (collapsible)
│   │   ├── Monaco Editor
│   │   └── GalleryOverlay (slide-out)
│   │       ├── SearchBar
│   │       ├── CategoryTabs
│   │       ├── ThumbnailGrid
│   │       └── SourceTabs
│   ├── SplitHandle (editor↔globe)
│   ├── GlobeArea
│   │   └── Earth.tsx
│   ├── SplitHandle (globe↔panel)
│   └── AnalysisPanel (collapsible)
│       └── Overlay.tsx
```

## Files

| File | Action | Description |
|------|--------|-------------|
| `ui/src/App.tsx` | Rewrite | Three-column layout + splitters + toolbar |
| `ui/src/TopBar.tsx` | **New** | Toolbar extracted from App.tsx |
| `ui/src/EditorPanel.tsx` | **New** | Monaco Editor wrapper |
| `ui/src/GalleryOverlay.tsx` | **New** | Example gallery slide-out panel |
| `ui/src/SplitHandle.tsx` | **New** | Draggable split handle component |
| `ui/src/examples/aurora/` | **New** | Aurora-specific examples (8–12 .js files) |
| `ui/src/examples/cesium/` | **New** | CesiumJS official examples (30+ .js files) |
| `ui/src/examples/manifest.ts` | **New** | Example index (name/category/description/thumbnail/file) |
| `ui/src/aurora.css` | Extend | Editor, gallery, splitter, toolbar styles |
| `ui/package.json` | Modify | Add `@monaco-editor/react` dependency |

## Technical Notes

- **Splitter**: pointerdown → pointermove → pointerup, `cursor: col-resize`, `user-select: none` on body during drag, min/max width constraints
- **Gallery thumbnails**: lazy load (IntersectionObserver), render on scroll into viewport
- **Monaco**: lazy init on first editor panel open, `automaticLayout: true` for resize
- **CesiumJS execution**: `new Function()` sandbox, inject `CESIUM_BASE_URL` and `AURORA_VIEWER`
- **Error display**: small status bar below editor showing last error message and line number
