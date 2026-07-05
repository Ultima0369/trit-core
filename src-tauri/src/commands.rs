//! Tauri command handlers — bridge between frontend invoke() and Aurora pipeline.

use aurora::app::{AnalysisInput, AuroraApp};
use aurora::pipeline::analysis::SignalSpec;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::l1_cache::L1Cache;
use crate::l2_cache::L2Cache;
use crate::tile_downloader::TileDownloader;

/// Serializable input from the frontend for running the pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineRequest {
    /// Signal frequency in Hz (e.g., 2.0).
    pub freq: f64,
    /// Sample rate in Hz (e.g., 100.0).
    pub sample_rate: f64,
    /// Signal duration in seconds (e.g., 1.0).
    pub duration_secs: f64,
    /// Noise standard deviation (e.g., 0.1).
    pub noise_std: f64,
    /// Frequency threshold for embodied detection.
    pub frequency_threshold: f64,
    /// Whether the user reports feeling normal.
    pub user_feels_normal: bool,
}

/// Single input signal — for the Frame tension visualization in the HUD.
/// Maps 1:1 from `truncore::TritWord` (frame + value + phase).
#[derive(Debug, Clone, Serialize)]
pub struct SignalResponse {
    /// Frame name, e.g. "Science", "Embodied", "GeoEco" (Debug repr, matches `decision`).
    pub frame: String,
    /// Value: "True", "Hold", "False", or "Unknown" (Debug repr).
    pub value: String,
    /// Phase in [0.0, 1.0] (0.5 = neutral).
    pub phase: f64,
}

/// Serializable output returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineResponse {
    /// Detected fundamental frequency in Hz.
    pub detected_freq_hz: f64,
    /// Decision value: "True", "Hold", or "False".
    pub decision: String,
    /// Final decision Phase in [0.0, 1.0] (from `result.phase()`).
    pub phase: f64,
    /// Final Frame name (from `result.frame()`), e.g. "Meta" for cross-frame Holds.
    pub final_frame: String,
    /// Per-input signals for the Frame tension visualization.
    pub signals: Vec<SignalResponse>,
    /// Attention Sovereignty Index [0.0, 1.0].
    pub asi: f64,
    /// Number of reminders in this session.
    pub reminder_count: usize,
    /// Active shift count.
    pub active_shift_count: usize,
    /// Conflict cards.
    pub conflicts: Vec<ConflictResponse>,
    /// Reminder history entries.
    pub reminders: Vec<ReminderResponse>,
    /// Full HTML report.
    pub html: String,
    /// JSON report string.
    pub json: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictResponse {
    pub conflict_type: String,
    pub reason: String,
    pub frame_a: String,
    pub frame_b: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReminderResponse {
    pub timestamp: String,
    pub action: String,
    pub target: String,
    pub response: Option<String>,
}

/// Tauri-managed application state.
pub struct AppState {
    pub app: Mutex<AuroraApp>,
    pub l1: Arc<L1Cache>,
    pub l2: Arc<L2Cache>,
    pub downloader: Arc<TileDownloader>,
}

/// 将 UserResponse 枚举映射为用户可读字符串，避免暴露 Debug 表示。
use aurora::bc::attention_guidance::UserResponse;

fn format_user_response(ur: &UserResponse) -> String {
    match ur {
        UserResponse::ShiftedTo(target) => format!("已切换注意力 → {}", target),
        UserResponse::OverrodeHold { chosen_frame } => format!("覆盖 Hold → {}", chosen_frame),
        UserResponse::Ignored => "已忽略".into(),
        UserResponse::Dismissed => "已关闭".into(),
    }
}

/// Run the full Aurora analysis + attention pipeline.
///
/// Called from the frontend via:
///   invoke('run_analysis_pipeline', { request: PipelineRequest })
#[tauri::command]
pub fn run_analysis_pipeline(
    request: PipelineRequest,
    state: State<AppState>,
) -> Result<PipelineResponse, String> {
    // Validate input before passing to the pipeline
    if !request.freq.is_finite() || request.freq <= 0.0 || request.freq > 1000.0 {
        return Err(format!(
            "invalid freq: {} (expected 0 < freq <= 1000)",
            request.freq
        ));
    }
    if !request.sample_rate.is_finite()
        || request.sample_rate <= 0.0
        || request.sample_rate > 100_000.0
    {
        return Err(format!(
            "invalid sample_rate: {} (expected 0 < sample_rate <= 100000)",
            request.sample_rate
        ));
    }
    if !request.duration_secs.is_finite()
        || request.duration_secs <= 0.0
        || request.duration_secs > 3600.0
    {
        return Err(format!(
            "invalid duration_secs: {} (expected 0 < duration <= 3600)",
            request.duration_secs
        ));
    }
    if !request.noise_std.is_finite() || request.noise_std < 0.0 || request.noise_std > 100.0 {
        return Err(format!(
            "invalid noise_std: {} (expected 0 <= noise_std <= 100)",
            request.noise_std
        ));
    }
    if !request.frequency_threshold.is_finite()
        || request.frequency_threshold <= 0.0
        || request.frequency_threshold > 1000.0
    {
        return Err(format!(
            "invalid frequency_threshold: {} (expected 0 < threshold <= 1000)",
            request.frequency_threshold
        ));
    }

    crate::logger::log(
        "command",
        "INFO",
        &format!(
            "run_analysis_pipeline 请求: freq={} sr={} dur={} noise={} thresh={} normal={}",
            request.freq,
            request.sample_rate,
            request.duration_secs,
            request.noise_std,
            request.frequency_threshold,
            request.user_feels_normal
        ),
    );

    let app = state.app.lock().map_err(|e| {
        crate::logger::log("command", "ERROR", &format!("AppState lock 失败: {e}"));
        format!("lock error: {e}")
    })?;

    let input = AnalysisInput {
        spec: SignalSpec {
            freq: request.freq,
            sample_rate: request.sample_rate,
            duration_secs: request.duration_secs,
            noise_std: request.noise_std,
        },
        frequency_threshold: request.frequency_threshold,
        user_feels_normal: request.user_feels_normal,
    };

    crate::logger::log("command", "INFO", "调用 AuroraApp::run_pipeline...");
    let output = app.run_pipeline(input).map_err(|e| {
        crate::logger::log("command", "ERROR", &format!("pipeline 错误: {e}"));
        format!("pipeline error: {e}")
    })?;

    crate::logger::log(
        "command",
        "INFO",
        &format!(
            "pipeline 完成: freq={:.3}Hz decision={:?} asi={:.3} conflicts={} reminders={}",
            output.analysis_report.spectrum.fundamental_hz,
            output.analysis_report.decision.result.value(),
            output.attention_outcome.asi,
            output.analysis_report.decision.interrupts.len(),
            output.attention_outcome.reminder_count,
        ),
    );

    let conflicts: Vec<ConflictResponse> = output
        .analysis_report
        .decision
        .interrupts
        .iter()
        .map(|i| {
            let (frame_a, frame_b) = i.frames();
            ConflictResponse {
                conflict_type: format!("{:?}", i.conflict),
                reason: i.reason.clone(),
                frame_a,
                frame_b,
            }
        })
        .collect();

    // Per-input signals for the Frame tension visualization.
    // value/frame use Debug repr (consistent with the `decision` field); phase via inner().
    let signals: Vec<SignalResponse> = output
        .analysis_report
        .decision
        .input_signals
        .iter()
        .map(|w| SignalResponse {
            frame: format!("{:?}", w.frame()),
            value: format!("{:?}", w.value()),
            phase: w.phase().inner(),
        })
        .collect();

    let reminders: Vec<ReminderResponse> = output
        .attention_outcome
        .session
        .reminders()
        .iter()
        .map(|r| ReminderResponse {
            timestamp: r.timestamp.format("%H:%M:%S").to_string(),
            action: r.action.clone(),
            target: r.target.clone(),
            response: r.user_response.as_ref().map(format_user_response),
        })
        .collect();

    Ok(PipelineResponse {
        detected_freq_hz: output.analysis_report.spectrum.fundamental_hz,
        decision: format!("{:?}", output.analysis_report.decision.result.value()),
        phase: output.analysis_report.decision.result.phase().inner(),
        final_frame: format!("{:?}", output.analysis_report.decision.result.frame()),
        signals,
        asi: output.attention_outcome.asi,
        reminder_count: output.attention_outcome.reminder_count,
        active_shift_count: output.attention_outcome.session.user_active_shift_count(),
        conflicts,
        reminders,
        html: output.html,
        json: output.json,
    })
}

// ══════════════════════════════════════════════════════════════════
// 缓存管理命令
// ══════════════════════════════════════════════════════════════════

/// 缓存统计报告。
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    /// L1 命中率 [0.0, 1.0]
    pub l1_hit_rate: f64,
    /// L1 条目数
    pub l1_entries: u64,
    /// L1 使用量 (bytes, 近似)
    pub l1_bytes: u64,
    /// L1 最大容量
    pub l1_max_bytes: u64,
    /// L2 命中率 [0.0, 1.0]
    pub l2_hit_rate: f64,
    /// L2 文件数
    pub l2_files: u64,
    /// L2 总字节数
    pub l2_bytes: u64,
    /// L2 最大容量
    pub l2_max_bytes: u64,
    /// 下载器成功次数
    pub downloads_ok: u64,
    /// 下载器失败次数
    pub downloads_fail: u64,
}

/// 查询缓存统计。
#[tauri::command]
pub fn cache_stats(state: State<AppState>) -> CacheStats {
    let (l2_bytes, l2_files) = state.l2.size_and_count();
    CacheStats {
        l1_hit_rate: state.l1.hit_rate(),
        l1_entries: state.l1.entry_count(),
        l1_bytes: state.l1.size_bytes(),
        l1_max_bytes: state.l1.max_bytes(),
        l2_hit_rate: state.l2.hit_rate(),
        l2_files,
        l2_bytes,
        l2_max_bytes: state.l2.max_bytes(),
        downloads_ok: state.downloader.downloaded_count(),
        downloads_fail: state.downloader.failed_count(),
    }
}

/// 设置 L2 缓存上限。max_gb=0 表示无限制。
#[tauri::command]
pub fn set_cache_limit(max_gb: u64, state: State<AppState>) -> Result<String, String> {
    state
        .l2
        .set_max_bytes(max_gb.saturating_mul(1024 * 1024 * 1024));
    crate::logger::log("cache", "INFO", &format!("L2 上限已设为 {} GB", max_gb));
    Ok(format!("已设为 {} GB", max_gb))
}

/// 清空所有缓存。
#[tauri::command]
pub fn clear_cache(state: State<AppState>) -> Result<String, String> {
    state.l1.clear();
    state.l2.clear().map_err(|e| format!("clear error: {e}"))?;
    crate::logger::log("cache", "INFO", "缓存已清空");
    Ok("缓存已清空".into())
}

/// 强制重新下载瓦片（"刷新地球"用）。
/// 后台线程执行，立即返回；前端轮询 get_asset_status 看进度。
#[tauri::command]
pub fn refresh_tiles() -> Result<String, String> {
    crate::asset_fetcher::refresh_tiles();
    Ok("瓦片刷新已启动".into())
}

/// 获取服务器健康状态。
#[tauri::command]
pub fn server_health() -> String {
    "OK".into()
}

/// 获取地图图层监测站坐标。
///
/// 返回 thermal / ecological 两类站点的真实地理参考点，供 MapPanel
/// 图层叠加渲染。坐标取自 anchor 模块对应监测网络的真实站点：
/// - thermal: CERES 辐射 + Mauna Loa CO2 + 拉尼尼亚能量失衡参考点
/// - ecological: BII / 碳汇 / 海洋 pH 监测点（亚马逊、婆罗洲、夏威夷海洋酸化站等）
///
/// ponytail: 静态站点表——坐标 + 名称足够渲染点位；接入实时传感器读数
/// 时，在此结构加 `value` 字段并由 anchor 模块填充。
#[derive(Debug, Clone, Serialize)]
pub struct MonitoringStation {
    /// "thermal" | "ecological" —对应 MapPanel 的 layerKey 后缀。
    pub kind: String,
    /// 站点人类可读名。
    pub name: String,
    pub lat: f64,
    pub lng: f64,
}

/// 地缘冲突事件（与 data_sources::ucdp::GeoEvent 对齐）。
#[derive(Debug, Clone, Serialize)]
pub struct GeoEventResponse {
    pub lat: f64,
    pub lng: f64,
    /// "state-based" | "non-state" | "one-sided"。
    pub violence_type: String,
    pub deaths: i64,
    pub country: String,
    pub date: String,
}

#[tauri::command]
pub fn get_monitoring_stations() -> Vec<MonitoringStation> {
    let thermal = [
        // CERES FM 扫描参考点 (非洲赤道带, ITCZ)
        ("CERES ITCZ Scan", 5.0, 20.0),
        // Mauna Loa CO2 观测站 (Scripps/NOAA, 450 ppm 阈值的基准源)
        ("Mauna Loa CO2", 19.536, -155.576),
        // 拉尼尼亚监测: 赤道太平洋 TOA 能量失衡参考点
        ("Equatorial Pacific TOA", 0.0, -170.0),
        // 北极放大: OLR 异常关键区
        ("Arctic OLR Reference", 80.0, 0.0),
    ];
    let ecological = [
        // Amazon — 全球最大陆地碳汇 + BII 基准区
        ("Amazon Carbon Sink", -3.0, -60.0),
        // Borneo — 东南亚生物多样性热点 (BII 退化监测)
        ("Borneo BII Site", 1.0, 114.0),
        // Hawaii Ocean Time-series (HOT) — 海洋表层 pH 长期监测 (8.17→7.95 阈值源)
        ("HOT Ocean pH", 22.75, -158.0),
        // Southern Ocean — 南大洋碳汇 (最大海洋碳吸收区)
        ("Southern Ocean Sink", -60.0, 0.0),
    ];
    thermal
        .iter()
        .map(|(n, lat, lng)| MonitoringStation {
            kind: "thermal".into(),
            name: (*n).into(),
            lat: *lat,
            lng: *lng,
        })
        .chain(ecological.iter().map(|(n, lat, lng)| MonitoringStation {
            kind: "ecological".into(),
            name: (*n).into(),
            lat: *lat,
            lng: *lng,
        }))
        .collect()
}

/// 单个传感器读数（与 trit-core anchor::SensorReading 对应）。
#[derive(Debug, Clone, Serialize)]
pub struct SensorReadingResponse {
    pub name: String,
    pub value: f64,
    pub threshold: f64,
    pub violated: bool,
    pub unit: String,
}

/// 一个 anchor 的状态快照（thermal / ecological）。
#[derive(Debug, Clone, Serialize)]
pub struct AnchorStatusResponse {
    /// "thermal" | "ecological" —与 MonitoringStation.kind 对齐。
    pub kind: String,
    pub readings: Vec<SensorReadingResponse>,
    /// 是否存在任何违例。
    pub has_violations: bool,
}

/// 获取 anchor 状态快照，供监测站 popup 展示读数 vs 阈值。
///
/// thermal anchor 的 OLR 异常维度接真实气候数据（Open-Meteo 温度异常，
/// 经 data_sources::climate 采集 + L2 缓存）；CO2/能量失衡暂用 safe 静态
/// 基线（待稳定 CO2 JSON 源接入）。ecological 用 safe/degraded 静态。
///
/// `degraded=true` 用 degraded/exceeded 构造演示违例态；默认读真实气候。
/// 采集失败 fail-safe 回落 safe()。
///
/// ponytail: 接实时传感器时，让 AuroraApp 持有 anchor 实例，从这里取。
#[tauri::command]
pub async fn get_anchor_status(
    degraded: bool,
    state: State<'_, AppState>,
) -> Result<Vec<AnchorStatusResponse>, String> {
    use aurora::anchor::{ecological_base::EcologicalBase, thermal_baseline::ThermalBaseline};

    // 真实气候读数（温度异常 + Mauna Loa CO2）；失败回落空/静态。
    let climate = if degraded {
        Vec::new()
    } else {
        crate::data_sources::climate::fetch_climate_readings(&state.l2).await
    };
    // Mauna Loa 温度异常作为 OLR 异常代理（物理相关：地表偏暖 ↔ OLR 偏移）。
    let olr_anomaly = climate
        .iter()
        .find(|c| c.station == "Mauna Loa")
        .map(|c| c.anomaly_c)
        .unwrap_or(0.0);
    // 真实 Mauna Loa CO2 ppm（NOAA GML）；采集失败回落 safe 基线 415ppm。
    let co2_ppm = if degraded {
        415.0
    } else {
        crate::data_sources::climate::fetch_co2_ppm(&state.l2)
            .await
            .unwrap_or(415.0)
    };

    let thermal = if degraded {
        ThermalBaseline::exceeded()
    } else {
        // OLR 维度接真实温度异常；CO2 接真实 Mauna Loa 读数；imbalance 用 safe 基线。
        ThermalBaseline::with_static_values(
            olr_anomaly,
            co2_ppm,
            0.5,
            aurora::anchor::thermal_baseline::ThermalBaselineConfig::default(),
        )
    };
    let ecological = if degraded {
        EcologicalBase::degraded()
    } else {
        EcologicalBase::safe()
    };

    let to_resp = |kind: &str, readings: Vec<aurora::anchor::SensorReading>| AnchorStatusResponse {
        kind: kind.into(),
        has_violations: readings.iter().any(|r| r.violated),
        readings: readings
            .iter()
            .map(|r| SensorReadingResponse {
                name: r.name.into(),
                value: r.value,
                threshold: r.threshold,
                violated: r.violated,
                unit: r.unit.into(),
            })
            .collect(),
    };

    Ok(vec![
        to_resp("thermal", thermal.snapshot()),
        to_resp("ecological", ecological.snapshot()),
    ])
}

/// 获取地缘冲突事件（UCDP GED），供 geoEvents 图层渲染。
///
/// 借鉴 worldmonitor 的 UCDP 链路：坐标 + 暴力类型 + 死伤，前端按类型分色、
/// 按死伤缩放半径。trit-core 本地直采 + L2 缓存，失败返回空。
#[tauri::command]
pub async fn get_geo_events(state: State<'_, AppState>) -> Result<Vec<GeoEventResponse>, String> {
    let events = crate::data_sources::ucdp::fetch_geo_events(&state.l2).await;
    Ok(events
        .iter()
        .map(|e| GeoEventResponse {
            lat: e.lat,
            lng: e.lng,
            violence_type: e.violence_type.clone(),
            deaths: e.deaths,
            country: e.country.clone(),
            date: e.date.clone(),
        })
        .collect())
}

/// 导出全部用户数据为 JSON 字符串。
///
/// M1 Exit Criteria "数据导出" + CHARTER "不剥夺"：用户可带走自己的数据。
/// 前端拿到字符串后用 Blob + a[download] 触发下载（不依赖 fs/dialog 插件）。
/// ponytail: 返回字符串而非写文件——纯 web 标准下载，零新依赖。
#[tauri::command]
pub fn export_user_data(state: State<AppState>) -> Result<String, String> {
    let app = state.app.lock().map_err(|e| format!("lock error: {e}"))?;
    app.export_data_json()
        .map_err(|e| format!("export error: {e}"))
}

/// 预取指定区域的瓦片（后台队列）。
///
/// 参数校验：
/// - 纬度范围 [-90, 90]，lng 范围 [-180, 180]
/// - 缩放级别 [0, 18]
/// - 最大瓦片数 50000（防止资源耗尽）
#[tauri::command]
pub async fn prefetch_tiles(
    lat_min: f64,
    lng_min: f64,
    lat_max: f64,
    lng_max: f64,
    z_min: u32,
    z_max: u32,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // ── 输入白名单校验 ──
    if !(-90.0..=90.0).contains(&lat_min) || !(-90.0..=90.0).contains(&lat_max) {
        return Err(format!(
            "纬度必须在 [-90, 90] 范围内，收到: lat_min={lat_min}, lat_max={lat_max}"
        ));
    }
    if !(-180.0..=180.0).contains(&lng_min) || !(-180.0..=180.0).contains(&lng_max) {
        return Err(format!(
            "经度必须在 [-180, 180] 范围内，收到: lng_min={lng_min}, lng_max={lng_max}"
        ));
    }
    if lat_min > lat_max {
        return Err(format!("lat_min ({lat_min}) 必须 ≤ lat_max ({lat_max})"));
    }
    if lng_min > lng_max {
        return Err(format!("lng_min ({lng_min}) 必须 ≤ lng_max ({lng_max})"));
    }
    if z_min > 18 || z_max > 18 {
        return Err(format!(
            "缩放级别最大为 18，收到: z_min={z_min}, z_max={z_max}"
        ));
    }
    if z_min > z_max {
        return Err(format!("z_min ({z_min}) 必须 ≤ z_max ({z_max})"));
    }

    let total_tiles: usize = (z_min..=z_max)
        .map(|z| {
            let x_min = crate::utils::lng_to_tile_x(lng_min, z);
            let x_max = crate::utils::lng_to_tile_x(lng_max, z);
            let y_min = crate::utils::lat_to_tile_y(lat_max, z);
            let y_max = crate::utils::lat_to_tile_y(lat_min, z);
            (x_max.saturating_sub(x_min) + 1) as usize * (y_max.saturating_sub(y_min) + 1) as usize
        })
        .sum();

    const MAX_TILES: usize = 50000;
    if total_tiles > MAX_TILES {
        return Err(format!(
            "瓦片数 ({total_tiles}) 超过上限 ({MAX_TILES})，请缩小区域或降低缩放级别"
        ));
    }

    crate::logger::log("prefetch", "INFO", &format!(
        "预取请求: bbox({lat_min},{lng_min},{lat_max},{lng_max}) z{z_min}-z{z_max}, 约 {total_tiles} 个瓦片"
    ));

    // 后台异步处理（不等待完成）
    let downloader = Arc::clone(&state.downloader);
    let l2 = Arc::clone(&state.l2);
    let l1 = Arc::clone(&state.l1);

    tokio::spawn(async move {
        for z in z_min..=z_max {
            let x_min = crate::utils::lng_to_tile_x(lng_min, z);
            let x_max = crate::utils::lng_to_tile_x(lng_max, z);
            let y_min = crate::utils::lat_to_tile_y(lat_max, z);
            let y_max = crate::utils::lat_to_tile_y(lat_min, z);

            for x in x_min..=x_max {
                for y in y_min..=y_max {
                    let key = format!("china-tiles/{}/{}/{}.jpg", z, x, y);

                    // 跳过已缓存的
                    if l2.exists(&key) {
                        continue;
                    }

                    if let Some(data) = downloader.download(z, x, y).await {
                        let _ = l2.put(&key, &data);
                        l1.put(&key, data);
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
            }
        }
        crate::logger::log("prefetch", "INFO", "预取完成");
    });

    Ok(format!("预取任务已启动 (~{} 个瓦片)", total_tiles))
}
