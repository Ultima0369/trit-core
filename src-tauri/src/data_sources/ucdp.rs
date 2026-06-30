//! UCDP 地缘冲突事件采集 — UCDP GED API（公开、无 key）。
//!
//! 借鉴 worldmonitor/scripts/seed-ucdp-events.mjs：从 UCDP GED API 拉取
//! 地缘冲突事件（坐标 + 暴力类型 + 估计死伤）。trit-core 改为本地直采 +
//! L2 缓存，喂 geoEvents 图层。
//!
//! API: https://ucdpapi.pcr.uu.se/api/gedevents/{version}?pagesize=N&page=P
//! version 探测：worldmonitor 用 (本年.1, 去年.1, 25.1, 24.1) 顺序尝试。
//!
//! ponytail: 采集失败返回空 Vec；限制事件数防止 payload 爆。按暴力类型
//! 分 1=state-based / 2=non-state / 3=one-sided（与 worldmonitor 一致）。

use serde::{Deserialize, Serialize};

use super::{http_client, read_cache, write_cache, UCDP_CACHE_KEY};
use crate::l2_cache::L2Cache;

const PAGE_SIZE: usize = 500;
const MAX_EVENTS: usize = 500; // 本地缓存 guard，防止 payload 过大。
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Aurora/0.1";

/// 一个地缘冲突事件（喂 geoEvents 图层）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoEvent {
    pub lat: f64,
    pub lng: f64,
    /// "state-based" | "non-state" | "one-sided"（与 worldmonitor 对齐）。
    pub violence_type: String,
    /// 估计死伤（半径缩放依据，借鉴 worldmonitor getRadius=sqrt(deaths)*scale）。
    pub deaths: i64,
    pub country: String,
    pub date: String,
}

/// UCDP API 单条事件（仅取需要的字段）。
#[derive(Debug, Deserialize)]
struct UcdpRow {
    latitude: Option<f64>,
    longitude: Option<f64>,
    #[serde(rename = "type_of_violence")]
    type_code: i64,
    #[serde(default)]
    best: Option<i64>,
    country: Option<String>,
    date_start: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UcdpPage {
    #[serde(default, rename = "Result")]
    result: Vec<UcdpRow>,
}

fn violence_type(code: i64) -> &'static str {
    match code {
        1 => "state-based",
        2 => "non-state",
        3 => "one-sided",
        _ => "state-based",
    }
}

/// 采集 UCDP 事件。失败返回空 Vec（fail-safe）。
pub async fn fetch_geo_events(l2: &L2Cache) -> Vec<GeoEvent> {
    if let Some(cached) = read_cache::<Vec<GeoEvent>>(l2, UCDP_CACHE_KEY) {
        if !cached.stale {
            return cached.data;
        }
        crate::logger::log("ucdp", "INFO", "UCDP 缓存过期，后台刷新");
    }

    let client = http_client();
    // 版本探测：顺序尝试近年版本，首个返回非空数据的版本即用。
    let year = current_year();
    let versions: [String; 4] = [
        format!("{}.1", year),
        format!("{}.1", year - 1),
        "25.1".into(),
        "24.1".into(),
    ];

    let mut events = Vec::new();
    'outer: for version in &versions {
        for page in 0..3 {
            match fetch_page(&client, version, page).await {
                Some(rows) if !rows.is_empty() => {
                    for r in rows {
                        if events.len() >= MAX_EVENTS {
                            break 'outer;
                        }
                        if let (Some(lat), Some(lng)) = (r.latitude, r.longitude) {
                            events.push(GeoEvent {
                                lat,
                                lng,
                                violence_type: violence_type(r.type_code).into(),
                                deaths: r.best.unwrap_or(0),
                                country: r.country.unwrap_or_default(),
                                date: r.date_start.unwrap_or_default(),
                            });
                        }
                    }
                }
                _ => break, // 该版本无更多页 → 换下个版本
            }
        }
        if !events.is_empty() {
            break; // 已从某版本拿到数据，不再探测
        }
    }

    if !events.is_empty() {
        let json = serde_json::to_vec(&events).unwrap_or_default();
        write_cache(l2, UCDP_CACHE_KEY, &json);
        crate::logger::log("ucdp", "INFO", &format!("采集 {} 个冲突事件", events.len()));
    } else {
        crate::logger::log("ucdp", "WARN", "UCDP 采集无数据（可能离线或 API 不可达）");
    }
    events
}

async fn fetch_page(
    client: &reqwest::Client,
    version: &str,
    page: usize,
) -> Option<Vec<UcdpRow>> {
    let url = format!(
        "https://ucdpapi.pcr.uu.se/api/gedevents/{}?pagesize={}&page={}",
        version, PAGE_SIZE, page
    );
    let resp = client
        .get(&url)
        .header("User-Agent", UA)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let parsed: UcdpPage = resp.json().await.ok()?;
    Some(parsed.result)
}

fn current_year() -> i64 {
    // ponytail: 从系统时钟取年份。SystemTime 在测试外可用。
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_700_000_000); // 2023 兜底
    1970 + (secs / 31_557_600) as i64 // 粗略：秒/年
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violence_type_mapping() {
        assert_eq!(violence_type(1), "state-based");
        assert_eq!(violence_type(2), "non-state");
        assert_eq!(violence_type(3), "one-sided");
        assert_eq!(violence_type(99), "state-based"); // 兜底
    }

    #[test]
    fn ucdp_row_parses_minimal() {
        let json = r#"{"Result":[{"latitude":5.0,"longitude":20.0,"type_of_violence":1,"best":42,"country":"X","date_start":"2024-01-01"}]}"#;
        let p: UcdpPage = serde_json::from_str(json).unwrap();
        assert_eq!(p.result.len(), 1);
        assert_eq!(p.result[0].best, Some(42));
    }
}
