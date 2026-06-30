# ADR-005：Rust 作为主要实现语言

**状态**：已接受
**日期**：2026-06-20
**分类**：05_adr — 架构决策记录

---

## 背景

Aurora 需要高性能（实时小波分析）、高安全（处理敏感数据）、本地优先（跨平台桌面应用）。需要选择主要实现语言。

## 决策

**采用 Rust 作为 Aurora 的主要实现语言，Python 作为算法原型和可选桥接。**

## 考虑的选项

### 选项A：Python

**优势**：
- 生态丰富：NumPy、SciPy、PyWavelets、Pandas、Matplotlib
- 开发快速：动态类型，快速迭代
- 机器学习：TensorFlow、PyTorch、scikit-learn
- 数据科学：Jupyter、数据可视化

**劣势**：
- 性能：解释执行，小波分析在大数据量时慢
- 部署：需要 Python 运行时，打包困难
- 安全：动态类型，运行时错误多
- 内存：GC 不可控，内存占用不稳定
- 本地优先：打包成独立桌面应用困难（PyInstaller 不稳定）

**结论**：不可作为主语言。可作为算法原型和可选桥接。

### 选项B：TypeScript / JavaScript

**优势**：
- 前端生态：React、Vue、D3.js
- 桌面应用：Electron 成熟
- 开发快速：全栈 JavaScript

**劣势**：
- 性能：V8 引擎，数值计算慢（小波分析需要大量浮点运算）
- 内存：V8 内存管理不可控
- 安全：npm 依赖供应链风险
- 本地优先：Electron 打包体积大（>100MB），启动慢
- 已有 Trit-Core：Trit-Core 是 Rust，用 JS 需要 FFI 或 WASM

**结论**：不可作为主语言。可用于前端 UI（Tauri 的 Web 前端）。

### 选项C：Go

**优势**：
- 性能：编译型，比 Python/JS 快
- 并发：goroutine 适合并发数据采集
- 部署：单二进制文件，易于部署
- 生态：丰富的网络/数据库库

**劣势**：
- 数值计算：没有 NumPy/SciPy 级别的数值库
- 小波分析：没有成熟的小波库（Go 生态弱于 Python/Rust）
- 桌面应用：没有成熟的桌面 UI 框架
- 安全：虽然比 Python 好，但不如 Rust 的内存安全

**结论**：不可作为主语言。数值计算和桌面 UI 生态不足。

### 选项D：Rust ✅

**优势**：
- 性能：编译型，与 C/C++ 同级，适合小波分析
- 安全：内存安全、零 unsafe（Trit-Core 已有 `#![forbid(unsafe_code)]`）
- 并发： fearless concurrency，适合并发数据采集
- 部署：单二进制文件，易于打包
- 桌面应用：Tauri 成熟（Rust 后端 + Web 前端）
- 已有 Trit-Core：Trit-Core 是 Rust，天然集成
- 数值计算：`rustfft`、`nalgebra` 等库逐渐成熟
- 本地优先：单二进制，无运行时依赖

**劣势**：
- 学习曲线：所有权系统、生命周期、类型系统复杂
- 开发速度：比 Python/JS 慢，编译时间长
- 生态：数值计算库不如 Python 丰富
- 小波分析：没有 PyWavelets 级别的成熟库，需要自研或桥接

**缓解劣势**：
- 学习曲线：项目维护者已有 Rust 经验（Trit-Core）
- 开发速度：用 Python 做算法原型，验证后用 Rust 重写热路径
- 生态：小波分析热路径自研，其他用成熟库
- 小波分析：自研 CWT（核心），DWT 用 `rustfft`，Python 桥接用于快速原型

**结论**：可接受。劣势可通过工程策略缓解，优势不可妥协。

## 决策矩阵

| 维度 | Python | TypeScript | Go | Rust |
|------|--------|------------|-----|------|
| 性能 | ❌ | ❌ | ⚠️ | ✅ |
| 安全 | ❌ | ❌ | ⚠️ | ✅ |
| 桌面应用 | ❌ | ⚠️ | ❌ | ✅ |
| 小波分析 | ✅ | ❌ | ❌ | ⚠️ |
| 本地优先 | ❌ | ❌ | ⚠️ | ✅ |
| Trit-Core 集成 | ❌ | ❌ | ❌ | ✅ |
| 开发速度 | ✅ | ✅ | ⚠️ | ⚠️ |
| 学习曲线 | ✅ | ✅ | ⚠️ | ❌ |
| 生态丰富 | ✅ | ✅ | ⚠️ | ⚠️ |

## 使用策略

| 场景 | 语言 | 说明 |
|------|------|------|
| 核心协议（Trit-Core） | Rust | 已有实现，向后兼容扩展 |
| 小波分析热路径 | Rust | 自研 CWT，性能关键 |
| 小波分析原型 | Python | PyWavelets 快速验证 |
| 数据采集 | Rust | 并发、安全、本地优先 |
| 桌面应用后端 | Rust | Tauri 后端 |
| 桌面应用前端 | TypeScript | Tauri 的 Web 前端 |
| 数据可视化 | TypeScript | D3.js / ECharts |
| 机器学习（未来） | Python | 可选桥接，模型推理 Rust 化 |

## 影响

- **技术栈**：Rust 为核心，TypeScript 为前端，Python 为原型
- **团队要求**：核心开发者需要 Rust 能力
- **开发流程**：Python 原型 → Rust 实现 → 性能优化
- **CI/CD**：Rust 编译时间较长，需要缓存优化

## 验证

- Trit-Core 在 Rust 中稳定运行（已有验证）
- Rust 小波分析性能满足实时要求（< 1秒）
- Tauri 桌面应用可正常打包和运行

## 相关 ADR

- ADR-001：本地优先架构（Rust 支持本地高性能）
- ADR-006：Tauri 作为桌面框架（Rust 后端）
- ADR-002：小波变换 vs 傅里叶变换（Rust 实现小波分析）

## 状态

**已接受**。作为 Aurora 的核心实现语言，后续可在特定模块使用 Python/TS 作为辅助，但核心协议和性能路径必须是 Rust。

---

*本 ADR 记录 Aurora 主要实现语言的选择过程。*
