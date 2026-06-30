// ui/src/utils/diag.ts — 统一诊断日志工具
//
// console 输出 + 转发到 Rust 日志文件（Tauri 环境）。
// Rust 日志文件是唯一持久化来源；前端不再维护 localStorage 副本。
//
// ponytail: 不在模块顶层静态 import @tauri-apps/api/core——否则整个 tauri
// runtime 进主 bundle，且让所有动态 import 该包的模块退化成静态。改为
// 调用时再动态 import。

/// 写入前端诊断日志：console + 转发到 Rust 日志文件
export default function diag(module: string, level: string, message: string) {
  const ts = new Date().toISOString().substr(11, 12);
  const line = `[${ts}] [${level}] [${module}] ${message}`;
  console.log(line);

  // 转发到 Rust 日志文件（使用官方 @tauri-apps/api）
  if ((window as any).__TAURI_INTERNALS__) {
    import('@tauri-apps/api/core')
      .then(({ invoke }) => invoke('frontend_log', { level, module, message }))
      .catch(() => {});
  }
}

/// 检测当前是否运行在 Tauri 桌面环境
export function isTauriEnvironment(): boolean {
  return !!(window as any).__TAURI_INTERNALS__;
}
