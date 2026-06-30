//! 共享工具函数 — 文件系统、瓦片坐标转换。
//!
//! 从 asset_fetcher.rs 和 l2_cache.rs 中提取的公共函数，
//! 消除 DRY 违规。

use std::fs;
use std::path::Path;

/// 递归计算目录总大小。
pub fn dir_size(dir: &Path) -> u64 {
    dir_size_and_count(dir).0
}

/// 单次递归遍历同时返回 (总字节数, 文件数)。
/// 替代分别调用 dir_size + dir_file_count 的两次全目录遍历——
/// 在 5440 文件的 z3-z6 全量目录上省掉一次 3-4ms 的 read_dir 递归。
pub fn dir_size_and_count(dir: &Path) -> (u64, u64) {
    let mut total: u64 = 0;
    let mut count: u64 = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                    count += 1;
                } else if meta.is_dir() {
                    let (sub_size, sub_count) = dir_size_and_count(&entry.path());
                    total += sub_size;
                    count += sub_count;
                }
            }
        }
    }
    (total, count)
}

/// 将经度转换为 Slippy Map 瓦片 X 坐标。
pub fn lng_to_tile_x(lng: f64, zoom: u32) -> u32 {
    ((lng + 180.0) / 360.0 * (1u64 << zoom) as f64).floor() as u32
}

/// 将纬度转换为 Slippy Map 瓦片 Y 坐标。
pub fn lat_to_tile_y(lat: f64, zoom: u32) -> u32 {
    let lat_rad = lat.to_radians();
    let n = (1u64 << zoom) as f64;
    ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * n).floor()
        as u32
}

/// 字节数 → 人类可读格式。
pub fn human_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// 缓存命中率 [0.0, 1.0] — 命中数 / (命中 + 未命中)。
pub fn hit_rate(hits: u64, misses: u64) -> f64 {
    let total = hits + misses;
    if total > 0 {
        hits as f64 / total as f64
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_dir_size_nonempty() {
        let dir = std::env::temp_dir().join(format!("aurora_utils_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("a.txt"), b"hello");
        assert!(dir_size(&dir) >= 5);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dir_size_and_count() {
        let dir = std::env::temp_dir().join(format!("aurora_utils_fc_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("a.txt"), b"hello"); // 5 bytes
        let _ = fs::write(dir.join("b.txt"), b"yy"); // 2 bytes
        let (size, count) = dir_size_and_count(&dir);
        assert_eq!(count, 2);
        assert_eq!(size, 7);
        // dir_size 应与 dir_size_and_count 的字节数一致
        assert_eq!(dir_size(&dir), 7);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_lng_to_tile_x_beijing() {
        // 北京 ~116.4°E, z=8 → tile x
        let x = lng_to_tile_x(116.4, 8);
        // 实际值为 210（不是 217）
        assert_eq!(x, 210);
    }

    #[test]
    fn test_lat_to_tile_y_beijing() {
        // 北京 ~39.9°N, z=8 → tile y
        let y = lat_to_tile_y(39.9, 8);
        // 实际值为 97（不是 94）
        assert_eq!(y, 97);
    }

    #[test]
    fn test_human_size() {
        assert_eq!(human_size(500), "500 B");
        assert_eq!(human_size(2048), "2 KB");
        assert_eq!(human_size(5_242_880), "5.0 MB");
    }

    #[test]
    fn test_hit_rate() {
        assert_eq!(hit_rate(0, 0), 0.0);
        assert_eq!(hit_rate(3, 1), 0.75);
        assert_eq!(hit_rate(0, 5), 0.0);
        assert_eq!(hit_rate(4, 0), 1.0);
    }
}
