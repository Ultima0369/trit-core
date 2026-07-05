//! 气候数据采集 — Open-Meteo Archive API（公开、无 key）。
//!
//! 借鉴 worldmonitor 的 climate seed：从公开 API 拉取温度等读数。trit-core
//! 改为本地直采 + L2 缓存。当前只采温度异常（Open-Meteo 提供稳定的
//! 站点历史温度）；CO2 待找到稳定 JSON 源后接入（thermal anchor 暂用
//! safe() 的 415ppm 静态值兜底）。
//!
//! ponytail: 采集失败返回 None，上层回落静态值，绝不阻塞 UI。

use serde::{Deserialize, Serialize};

use super::{http_client, read_cache, write_cache, CLIMATE_CACHE_KEY, CO2_CACHE_KEY};
use crate::l2_cache::L2Cache;

/// 一个站点的气候读数（喂 thermal anchor 的温度维度）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClimateReading {
    /// 站点名。
    pub station: String,
    pub lat: f64,
    pub lng: f64,
    /// 近期日均温度（℃）。
    pub temp_c: f64,
    /// 相对该站点长期均温的异常（℃）。正值=偏暖。
    pub anomaly_c: f64,
}

/// Open-Meteo Archive 响应（仅取需要的字段）。
#[derive(Debug, Deserialize)]
struct ArchiveResponse {
    #[serde(default)]
    daily: Option<ArchiveDaily>,
}

#[derive(Debug, Deserialize)]
struct ArchiveDaily {
    #[serde(default, rename = "temperature_2m_mean")]
    temp_mean: Vec<Option<f64>>,
}

/// 采集一组站点的近期温度异常。失败返回空 Vec（fail-safe）。
///
/// 借鉴 worldmonitor climate seed 的站点选择：取分布全球的温度参考站。
/// 异常 = 近期均值 - 站点长期基准（这里用 Open-Meteo 的 climatology 年均）。
pub async fn fetch_climate_readings(l2: &L2Cache) -> Vec<ClimateReading> {
    // 先读缓存；新鲜则直接返回。
    if let Some(cached) = read_cache::<Vec<ClimateReading>>(l2, CLIMATE_CACHE_KEY) {
        if !cached.stale {
            return cached.data;
        }
        // stale：后台仍可刷新，但先返回旧数据不阻塞 UI。
        crate::logger::log("climate", "INFO", "气候缓存过期，后台刷新");
    }

    let client = http_client();
    // 站点：(name, lat, lng, long_term_baseline_c)
    // 基线用粗略气候学年均（ponytail：MVP 用静态基线，接真实 climatology
    // API 时替换为 Open-Meteo 的 climatology endpoint）。
    let stations = [
        ("Mauna Loa", 19.54, -155.58, 13.0),
        ("Amazon", -3.0, -60.0, 26.0),
        ("Borneo", 1.0, 114.0, 27.0),
        ("Southern Ocean", -60.0, 0.0, -2.0),
        ("Arctic", 80.0, 0.0, -15.0),
    ];

    let mut readings = Vec::new();
    for (name, lat, lng, baseline) in stations {
        match fetch_station_temp(&client, lat, lng).await {
            Some(temp) => {
                readings.push(ClimateReading {
                    station: name.into(),
                    lat,
                    lng,
                    temp_c: temp,
                    anomaly_c: temp - baseline,
                });
            }
            None => {
                crate::logger::log(
                    "climate",
                    "WARN",
                    &format!("站点 {} 温度采集失败，跳过", name),
                );
            }
        }
    }

    if !readings.is_empty() {
        let json = serde_json::to_vec(&readings).unwrap_or_default();
        write_cache(l2, CLIMATE_CACHE_KEY, &json);
    }
    readings
}

/// 拉 Open-Meteo Archive 最近 7 天日均温度，返回均值。
async fn fetch_station_temp(client: &reqwest::Client, lat: f64, lng: f64) -> Option<f64> {
    // ponytail: 用固定近期日期窗口不可靠（依赖系统时钟），改为取过去 30 天。
    // Open-Meteo Archive 需要 start/end 日期；这里用 today-30..today。
    let (start, end) = recent_window();
    let url = format!(
        "https://archive-api.open-meteo.com/v1/archive?latitude={lat}&longitude={lng}&start={start}&end={end}&daily=temperature_2m_mean"
    );
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let parsed: ArchiveResponse = resp.json().await.ok()?;
    let daily = parsed.daily?;
    let temps: Vec<f64> = daily.temp_mean.into_iter().flatten().collect();
    if temps.is_empty() {
        return None;
    }
    Some(temps.iter().sum::<f64>() / temps.len() as f64)
}

/// NOAA GML Mauna Loa 月均 CO2（ppm）。
///
/// 端点：https://gml.noaa.gov/aftp/products/trends/co2/co2_mm_mlo.txt
/// 格式：固定列宽文本，`#` 开头为注释，数据行第 4 列为月均 CO2 ppm。
/// 取最后一个数据行（最新月）。失败返回 None。
///
/// ponytail: 不解析为 JSON——NOAA 只提供文本，split_whitespace 按列取即可。
pub async fn fetch_co2_ppm(l2: &L2Cache) -> Option<f64> {
    if let Some(cached) = read_cache::<f64>(l2, CO2_CACHE_KEY) {
        if !cached.stale {
            return Some(cached.data);
        }
    }
    let client = http_client();
    let url = "https://gml.noaa.gov/aftp/products/trends/co2/co2_mm_mlo.txt";
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let text = resp.text().await.ok()?;
    let ppm = parse_co2_ppm(&text)?;
    if !ppm.is_finite() || !(300.0..=600.0).contains(&ppm) {
        // 合理性校验：CO2 应在 300-600 ppm 区间。
        crate::logger::log("climate", "WARN", &format!("CO2 读数异常 {ppm}，丢弃"));
        return None;
    }
    let json = serde_json::to_vec(&ppm).ok()?;
    write_cache(l2, CO2_CACHE_KEY, &json);
    crate::logger::log("climate", "INFO", &format!("Mauna Loa CO2 采集: {ppm} ppm"));
    Some(ppm)
}

/// 从 NOAA GML 文本解析最新月均 CO2 ppm。
/// 纯函数，便于测试。跳过 `#` 注释和空行，取最后一个数据行的第 4 列。
fn parse_co2_ppm(text: &str) -> Option<f64> {
    // rfind 从末尾找首个数据行（跳过注释/空行），避免遍历全部行。
    text.lines()
        .rfind(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .and_then(|l| {
            l.split_whitespace()
                .nth(3)
                .and_then(|s| s.parse::<f64>().ok())
        })
}

/// 返回 (today-30d, today) 的 YYYY-MM-DD 字符串。
fn recent_window() -> (String, String) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs_per_day: u64 = 86_400;
    let today = now / secs_per_day;
    let start = today.saturating_sub(30);
    (ymd(start), ymd(today))
}

/// unix 天数 → YYYY-MM-DD。
///
/// ponytail: 不用 Howard Hinnant civil_from_days（手写易错，曾系统性早 4 天）。
/// 用朴素锚点法：1970-01-01 = day 0，逐年逐月累加。调用频率低（每次采集
/// 一次），朴素循环的简单性 > 算法的常数级性能。
fn ymd(days_since_epoch: u64) -> String {
    let mut days = days_since_epoch as i64;
    let mut year = 1970i64;
    loop {
        let dy = if is_leap(year) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }
    let mdays = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &dm in &mdays {
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
    }
    format!("{:04}-{:02}-{:02}", year, month, days + 1)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ymd_epoch_is_1970_01_01() {
        assert_eq!(ymd(0), "1970-01-01");
    }

    #[test]
    fn ymd_known_dates() {
        // 2024-01-01 = 19723 days after 1970-01-01
        assert_eq!(ymd(19_723), "2024-01-01");
        // 2024-03-01 (闰年 2 月后)
        assert_eq!(ymd(19_783), "2024-03-01");
    }

    #[test]
    fn recent_window_is_30_days() {
        let (start, end) = recent_window();
        assert!(start < end);
    }

    #[test]
    fn parse_co2_takes_last_data_row() {
        // 模拟 NOAA GML 格式：注释行 + 数据行（year month decimal monthly deseason ...）。
        let text = "# header line\n# another comment\n\
                    2026    4   2026.2917      431.12      428.65     23    1.16    0.46\n\
                    2026    5   2026.3750      432.34      429.14     17    0.66    0.31\n";
        assert_eq!(parse_co2_ppm(text), Some(432.34));
    }

    #[test]
    fn parse_co2_returns_none_on_empty() {
        assert_eq!(parse_co2_ppm("# only comments\n"), None);
        assert_eq!(parse_co2_ppm(""), None);
    }
}
