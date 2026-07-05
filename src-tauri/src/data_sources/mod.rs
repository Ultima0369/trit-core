//! 公开数据采集模块 — 借鉴 worldmonitor 的公开数据链路。
//!
//! worldmonitor 用 Railway→Redis→Vercel 架构；trit-core 是本地桌面工具，
//! 改为：Rust 侧 reqwest 直采公开 API + 复用已有 L2 本地缓存。命令调用时
//! 读缓存（过期则同步拉取，失败 fail-safe 返回空），后台线程定时刷新。
//!
//! 当前两个数据源（均公开、无 API key）：
//! - climate: Open-Meteo Archive API（站点温度异常）+ 喂 thermal anchor
//! - ucdp: UCDP GED API（地缘冲突事件）→ geoEvents 图层
//!
//! ponytail: 采集失败绝不阻塞 UI —— 所有 fetch_xxx 失败返回空 Vec / None，
//! 上层回落 safe() 静态值或空图层。

pub mod climate;
pub mod ucdp;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use crate::l2_cache::L2Cache;

/// 缓存有效期：1 小时（公开数据日级更新，1h 足够新鲜且省请求）。
pub const CACHE_TTL: Duration = Duration::from_secs(3600);

/// 缓存键。
pub const CLIMATE_CACHE_KEY: &str = "data-sources/climate.json";
pub const CO2_CACHE_KEY: &str = "data-sources/co2.json";
pub const UCDP_CACHE_KEY: &str = "data-sources/ucdp-events.json";

const HEADER_LEN: usize = 8;

/// 缓存读取结果：数据 + 是否过期（过期仍返回数据，标记 stale 供后台刷新）。
pub struct Cached<T> {
    pub data: T,
    pub stale: bool,
}

/// 读缓存。无缓存返回 None；有缓存返回数据 + stale 标记。
pub fn read_cache<T: serde::de::DeserializeOwned>(l2: &L2Cache, key: &str) -> Option<Cached<T>> {
    let bytes = l2.get(key)?;
    if bytes.len() < HEADER_LEN {
        return None;
    }
    let mut ts = [0u8; HEADER_LEN];
    ts.copy_from_slice(&bytes[..HEADER_LEN]);
    let cached_at = u64::from_le_bytes(ts);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let stale = now.saturating_sub(cached_at) > CACHE_TTL.as_secs();
    let data = serde_json::from_slice::<T>(&bytes[HEADER_LEN..]).ok()?;
    Some(Cached { data, stale })
}

/// 写缓存：[8 字节 unix 秒][json 数据]。
pub fn write_cache(l2: &L2Cache, key: &str, data: &[u8]) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut out = Vec::with_capacity(HEADER_LEN + data.len());
    out.extend_from_slice(&now.to_le_bytes());
    out.extend_from_slice(data);
    let _ = l2.put(key, &out);
}

/// 共享 HTTP 客户端：10s 超时，Chrome UA（部分公开 API 拒绝默认 UA）。
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Aurora/0.1")
        .build()
        .expect("failed to build reqwest client")
}

/// 顺序刷新所有数据源缓存（后台线程调用）。
///
/// ponytail: 顺序而非并发——公开 API 限流更友好，且任一源失败不影响其他
/// （每个 fetch_xxx 内部 fail-safe 返回空）。失败仅记日志，不传播。
pub async fn refresh_all(l2: &L2Cache) {
    crate::logger::log("data-sources", "INFO", "后台刷新开始");
    let climate = climate::fetch_climate_readings(l2).await;
    crate::logger::log(
        "data-sources",
        "INFO",
        &format!("气候缓存: {} 条站点读数", climate.len()),
    );
    let co2 = climate::fetch_co2_ppm(l2).await;
    crate::logger::log(
        "data-sources",
        "INFO",
        &format!(
            "CO2 缓存: {}",
            co2.map(|v| format!("{v} ppm"))
                .unwrap_or_else(|| "采集失败".into())
        ),
    );
    let ucdp = ucdp::fetch_geo_events(l2).await;
    crate::logger::log(
        "data-sources",
        "INFO",
        &format!("UCDP 缓存: {} 个事件", ucdp.len()),
    );
    crate::logger::log("data-sources", "INFO", "后台刷新完成");
}

/// 后台刷新循环：启动后立即刷一次，之后每 CACHE_TTL 周期重复。
/// 在独立 tokio runtime 上跑（不依赖 Tauri runtime 可用性）。
/// 收到 shutdown 信号即退出。
pub fn spawn_refresh_loop(l2: Arc<L2Cache>, shutdown: Arc<AtomicBool>) {
    std::thread::Builder::new()
        .name("data-sources-refresher".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    crate::logger::log(
                        "data-sources",
                        "ERROR",
                        &format!("tokio runtime 构建失败: {e}"),
                    );
                    return;
                }
            };
            // ponytail: 立即刷一次（首屏数据预热），之后周期循环。
            while !shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                rt.block_on(refresh_all(&l2));
                // 分段 sleep 以便响应 shutdown。
                let mut slept = 0u64;
                while slept < CACHE_TTL.as_secs()
                    && !shutdown.load(std::sync::atomic::Ordering::SeqCst)
                {
                    std::thread::sleep(Duration::from_secs(5));
                    slept += 5;
                }
            }
            crate::logger::log("data-sources", "INFO", "后台刷新线程退出");
        })
        .expect("failed to spawn data-sources-refresher");
}
