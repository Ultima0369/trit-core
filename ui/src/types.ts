// ui/src/types.ts — TypeScript types matching Tauri PipelineResponse

export interface PipelineRequest {
  freq: number;
  sample_rate: number;
  duration_secs: number;
  noise_std: number;
  frequency_threshold: number;
  user_feels_normal: boolean;
}

export interface ConflictResponse {
  conflict_type: string;
  reason: string;
  frame_a: string;
  frame_b: string;
}

/** 单个输入信号 — 用于 Frame 张力可视化（与 Rust SignalResponse 对应）。 */
export interface SignalWord {
  /** Frame 名，如 "Science"、"Embodied"、"GeoEco"（Debug repr） */
  frame: string;
  /** 值："True" | "Hold" | "False" | "Unknown" */
  value: string;
  /** Phase [0.0, 1.0]，0.5 = 中性 */
  phase: number;
}

export interface ReminderResponse {
  timestamp: string;
  action: string;
  target: string;
  response: string | null;
}

export interface PipelineResponse {
  detected_freq_hz: number;
  decision: string;
  /** 最终决策 Phase [0.0, 1.0] */
  phase: number;
  /** 最终 Frame 名（跨帧 Hold 时为 "Meta"） */
  final_frame: string;
  /** 逐输入信号，供 Frame 张力图 */
  signals: SignalWord[];
  asi: number;
  reminder_count: number;
  active_shift_count: number;
  conflicts: ConflictResponse[];
  reminders: ReminderResponse[];
  html: string;
  json: string;
}

/** 缓存统计（与 Rust CacheStats 对应） */
export interface CacheStats {
  l1_hit_rate: number;
  l1_entries: number;
  l1_bytes: number;
  l1_max_bytes: number;
  l2_hit_rate: number;
  l2_files: number;
  l2_bytes: number;
  l2_max_bytes: number;
  downloads_ok: number;
  downloads_fail: number;
}

/** 地图/地球视角坐标（MapPanel + Earth 统一传给 StatusBar）。 */
export interface MapCoord {
  /** 鼠标或视角中心经度。null = 无数据（如鼠标移出地图）。 */
  lng: number | null;
  /** 鼠标或视角中心纬度。 */
  lat: number | null;
  /** 缩放级别（2D zoom / 3D 由 altitude 换算）。 */
  zoom: number | null;
  /** 比例尺：米/像素。 */
  scale: number | null;
}
export interface MonitoringStation {
  /** "thermal" | "ecological" */
  kind: string;
  name: string;
  lat: number;
  lng: number;
}

/** 单个传感器读数（与 Rust SensorReadingResponse 对应）。 */
export interface SensorReading {
  name: string;
  value: number;
  threshold: number;
  violated: boolean;
  unit: string;
}

/** 一个 anchor 的状态快照（与 Rust AnchorStatusResponse 对应）。 */
export interface AnchorStatus {
  /** "thermal" | "ecological" */
  kind: string;
  readings: SensorReading[];
  has_violations: boolean;
}

/** 地缘冲突事件（与 Rust GeoEventResponse 对应）。 */
export interface GeoEvent {
  lat: number;
  lng: number;
  /** "state-based" | "non-state" | "one-sided" */
  violence_type: string;
  deaths: number;
  country: string;
  date: string;
}

// ── Globe ──────────────────────────────────────────────────

export type GlobeTexture = 'blue-marble' | 'topographic';

// ── Custom event names (shared between Earth / TopBar / App) ──

export const EVENTS = {
  SET_GLOBE_TEXTURE: 'aurora-set-globe-texture',
  GLOBE_TEXTURE_CHANGED: 'aurora-globe-texture-changed',
  RESET_GLOBE: 'aurora-reset-globe',
} as const;

export const CESIUM_CONTAINER_ID = 'cesiumContainer';
