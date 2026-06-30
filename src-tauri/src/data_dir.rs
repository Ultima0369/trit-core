//! 统一数据目录解析。
//!
//! 优先级: AURORA_DATA_DIR 环境变量 > 项目根 aurora-data/ > exe 同目录
//!
//! 项目根通过编译期 CARGO_MANIFEST_DIR 定位（src-tauri 的 parent），
//! dev 下稳定指向工作区根（如 C:\trit-core\aurora-data）。
//! release 跨机器分发时应通过 AURORA_DATA_DIR 环境变量覆盖，
//! 因 CARGO_MANIFEST_DIR 是编译机路径。

use std::path::PathBuf;

/// 返回 Aurora 数据根目录。
/// 确保目录存在且可写，否则回退。
pub fn aurora_data_dir() -> PathBuf {
    // 1. 环境变量覆盖
    if let Ok(dir) = std::env::var("AURORA_DATA_DIR") {
        let p = PathBuf::from(&dir);
        if ensure_writable(&p) {
            crate::logger::log(
                "data_dir",
                "INFO",
                &format!("使用 AURORA_DATA_DIR: {}", p.display()),
            );
            return p;
        }
        crate::logger::log(
            "data_dir",
            "WARN",
            &format!("AURORA_DATA_DIR 不可写: {}", p.display()),
        );
    }

    // 2. 项目根 aurora-data/（编译期定位 src-tauri，parent 即项目根）
    //    ponytail: CARGO_MANIFEST_DIR 是编译期常量，dev 下指向工作区根下的 src-tauri
    if let Some(proj_root) = project_root() {
        let p = proj_root.join("aurora-data");
        if ensure_writable(&p) {
            crate::logger::log(
                "data_dir",
                "INFO",
                &format!("使用项目根数据目录: {}", p.display()),
            );
            return p;
        }
        crate::logger::log(
            "data_dir",
            "WARN",
            &format!("项目根数据目录不可写: {}", p.display()),
        );
    }

    // 3. 回退：exe 同目录
    let fallback = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
        .join("aurora-data");

    crate::logger::log(
        "data_dir",
        "WARN",
        &format!("回退到 exe 同目录: {}", fallback.display()),
    );
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// 编译期定位项目根：CARGO_MANIFEST_DIR 指向 src-tauri，其 parent 即项目根。
fn project_root() -> Option<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
}

/// 确保目录存在且可写。
fn ensure_writable(dir: &std::path::Path) -> bool {
    match std::fs::create_dir_all(dir) {
        Ok(()) => {
            let test_file = dir.join(".write_test");
            std::fs::write(&test_file, b"test").is_ok_and(|_| {
                let _ = std::fs::remove_file(&test_file);
                true
            })
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aurora_data_dir_returns_some_path() {
        let dir = aurora_data_dir();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn test_project_root_is_trit_core() {
        // dev 编译期 CARGO_MANIFEST_DIR = .../src-tauri，parent 应为项目根
        let root = project_root().expect("project_root should resolve at compile time");
        assert!(
            root.join("src-tauri").exists(),
            "项目根下应含 src-tauri: {}",
            root.display()
        );
    }
}
