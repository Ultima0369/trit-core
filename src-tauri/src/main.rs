// 隐藏 Windows 控制台窗口 (release 模式)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 日志由 lib.rs::run() 初始化，这里只需调用
    aurora_desktop_lib::run();
}
