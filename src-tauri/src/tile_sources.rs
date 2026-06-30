//! 多源瓦片配置与选择。
//!
//! 每个源定义 URL 模板、覆盖范围 (bbox)、缩放级别、优先级。
//! select_sources() 按优先级返回能覆盖给定瓦片的源列表。

/// 单个瓦片源配置。
#[derive(Debug, Clone)]
pub struct TileSource {
    pub name: &'static str,
    /// URL 模板，占位符: {s} (子域), {z}, {x}, {y}
    pub url_template: &'static str,
    /// 子域列表（如果 URL 含 {s}），否则 None
    pub subdomains: Option<&'static [&'static str]>,
    /// 覆盖范围 (lat_min, lng_min, lat_max, lng_max)，None = 全球
    pub bbox: Option<(f64, f64, f64, f64)>,
    pub min_zoom: u32,
    pub max_zoom: u32,
    /// 优先级，1 = 最高
    pub priority: u8,
    /// 是否使用 TMS Y 编号 (y=0 在南)
    pub tms_y: bool,
}

impl TileSource {
    /// 从 Slippy Map 坐标 (z, x, y) 构建此源的完整 URL。
    pub fn build_url(&self, z: u32, x: u32, y: u32) -> String {
        // TMS → Slippy Map Y 转换（如果需要）
        let y_val = if self.tms_y {
            let max_y = (1u32 << z).saturating_sub(1);
            max_y.saturating_sub(y)
        } else {
            y
        };

        let mut url = self
            .url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y_val.to_string());

        // 处理子域 {s}
        if url.contains("{s}") {
            if let Some(subdomains) = self.subdomains {
                let idx = (x.wrapping_add(y)) as usize % subdomains.len();
                url = url.replace("{s}", subdomains[idx]);
            } else {
                url = url.replace("{s}", "0");
            }
        }

        url
    }

    /// 检查此源是否覆盖给定的 Slippy Map 瓦片坐标。
    pub fn covers(&self, z: u32, x: u32, y: u32) -> bool {
        if z < self.min_zoom || z > self.max_zoom {
            return false;
        }
        if let Some((lat_min, lng_min, lat_max, lng_max)) = self.bbox {
            let (lat, lng) = tile_to_latlng(z, x, y);
            lat >= lat_min && lat <= lat_max && lng >= lng_min && lng <= lng_max
        } else {
            true // 全球覆盖
        }
    }
}

/// 预定义的瓦片源列表（按优先级排序）。
/// 仅 Esri World Imagery（全球统一），避免多源色调色差。
pub const TILE_SOURCES: &[TileSource] = &[
    TileSource {
        name: "Esri World Imagery",
        url_template:
            "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
        subdomains: None,
        bbox: None, // 全球
        min_zoom: 0,
        max_zoom: 18,
        priority: 2,
        tms_y: false, // Esri ArcGIS REST = Slippy Map (Google/OSM) grid, row 0 at north — NOT TMS
    },
];

/// 按优先级返回能覆盖给定瓦片的源列表。
pub fn select_sources(z: u32, x: u32, y: u32) -> Vec<&'static TileSource> {
    let mut sources: Vec<&TileSource> = TILE_SOURCES.iter().filter(|s| s.covers(z, x, y)).collect();
    sources.sort_by_key(|s| s.priority);
    sources
}

/// Slippy Map 瓦片坐标 → 瓦片中心经纬度
fn tile_to_latlng(z: u32, x: u32, y: u32) -> (f64, f64) {
    let n = (1u64 << z) as f64;
    // +0.5 → tile center; without it the formula yields the NW-corner (top edge).
    let lng = ((x as f64) + 0.5) / n * 360.0 - 180.0;
    let lat_rad = (std::f64::consts::PI * (1.0 - 2.0 * ((y as f64) + 0.5) / n))
        .sinh()
        .atan();
    let lat = lat_rad.to_degrees();
    (lat, lng)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esri_covers_global() {
        // Esri 是唯一源，全球覆盖（含中国和欧洲）
        let source = &TILE_SOURCES[0];
        assert!(source.covers(8, 217, 94), "应覆盖中国瓦片");
        assert!(source.covers(8, 129, 88), "应覆盖欧洲瓦片");
        assert!(source.covers(8, 0, 0), "应覆盖 (0,0)");
    }

    #[test]
    fn test_build_url_esri_slippy_no_flip() {
        // Esri ArcGIS REST cached tile services (World Imagery) use the standard
        // Web Mercator / Google-OSM tile grid: row 0 is at the NORTH and increases
        // southward — identical to Slippy Map Y. TMS (y=0 south) is a different spec.
        // So {y} must pass through unchanged; flipping it stores each tile at the
        // wrong latitude (vertically mirrored on disk).
        let source = &TILE_SOURCES[0];
        let url = source.build_url(8, 217, 94);
        assert!(url.contains("/94"), "Esri y should not be flipped: {url}");
        assert!(
            !url.contains("/161"),
            "Esri y must not be TMS-flipped: {url}"
        );
    }

    #[test]
    fn test_select_sources_returns_esri_for_china() {
        // 全球统一 Esri，中国区域也返回 Esri（无高德叠加，无色调色差）
        let sources = select_sources(8, 217, 94);
        assert!(!sources.is_empty());
        assert_eq!(sources[0].name, "Esri World Imagery");
    }

    #[test]
    fn test_select_sources_returns_esri_global() {
        let sources = select_sources(8, 129, 88);
        assert!(!sources.is_empty());
        assert!(sources.iter().all(|s| s.name == "Esri World Imagery"));
    }
}
