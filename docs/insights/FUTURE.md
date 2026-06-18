# FUTURE — 已知局限与未来方向

本文档诚实地记录 Trit-Core 当前的技术局限，以及每种局限的可能解决路径。这不是路线图——路线图见 [roadmap.md](../roadmap.md)。这是对"什么还不够好"的诚实评估。

---

## 1. 无形式化验证

### 现状

Trit-Core 通过 56 个属性测试（proptest）验证了核心不变性，但这不等同于形式化验证。proptest 是随机化采样，不是穷举证明。

### 局限

- 三值真值表的完备性未在定理证明器中验证
- 仲裁逻辑的公平性（如 MedicalEthics 是否在所有情况下都正确优先 Individual）依赖手工推理
- 分布式协议的状态机（Sovereign → Coupling → Coupled → Hold）未经过模型检查

### 可能路径

- Coq/Lean 4 形式化 TAND/TOR/TNOT 真值表
- TLA+ 模型检查 Node 状态机
- `kani`（Rust 验证器）用于证明特定不变性

---

## 2. 分布式协议：传输层与发现已完成

### 现状

`net/` 模块实现了完整的消息类型、状态机、PLL 控制器、TCP 传输层（M5）和种子节点发现（M6）。节点可通过 `TcpNodeServer`/`TcpClient` 进行 TCP 通信，通过 `--peers` 标志或 `TRIT_PEERS` 环境变量自动发现对等节点。Docker Compose 支持 3 节点全网格集群。

### 局限

- PLL 参数（kp=0.3、deadband=0.05）仅在本地和 TCP 环回测试中验证
- 无大规模集群测试（>3 节点）
- ~~无拜占庭容错~~ → M8 完成：`ByzantineGatekeeper` 提供消息验证、速率限制、per-peer 日志上限和已知节点强制
- 守门人剩余局限：无密码学签名验证、无 PBFT 式共识协议

---

## 3. Phase 浮点漂移

### 现状

Phase 使用 `f64` 存储，通过 `quantize()` 在每次运算后吸附到锚点（0.0、0.5、1.0）。这减轻了漂移，但不能完全消除。

### 局限

- 在极长级联中（>10^6 次操作），累积误差可能使 Phase 偏离真实值
- `quantize()` 的 epsilon 参数（当前 1e-6）是经验值，未经过严格数值分析

### 可能路径

- 使用有理数表示（`num::rational::Ratio`）替代 f64
- 使用定点数表示（如 0..1000 的 u16）
- 严格的误差传播分析，确定最大安全级联深度

---

## 4. 性能目标已初步验证

### 现状

性能目标为 10,000 TPS（每秒 TritWord 操作）。端到端基准测试（`docs/performance-validation.md`）已验证：
- 热路径微操作：~1.5ns/op（~667M ops/s 理论值）
- 端到端 MedicalEthics 管道：~658,000 TPS（65.8× 目标）
- 端到端 Physical 管道：~1,015,000 TPS（101.5× 目标）
- TCP 帧往返：~676,000 msg/s（67.6× 目标）

10,000 TPS 目标在所有层级均被大幅超额完成。

### 局限

- 无 dhat 堆分析验证热路径零分配声明
- 无多线程负载测试（当前基准测试为单线程）
- 无大规模集群性能数据（>3 节点）

### 可能路径

- `dhat` 堆分析验证零分配热路径
- 多线程下的 `ResonanceBus` 并发负载测试
- 大规模集群模拟（10+ 节点）的性能基准

---

## 5. Frame 类型有限

### 现状

Frame 只有 5 种：Science、Individual、Consensus、Absolute、Meta。FrameMask 使用 u8，最多支持 8 种。

### 局限

- 某些领域可能需要更细粒度的帧（如 Legal、Economic、Spiritual）
- u8 位掩码在 8 种帧时饱和

### 可能路径

- FrameMask 升级为 u16 或 u32
- 用户自定义 Frame（类似 Domain::Custom）
- 帧层次结构（子帧继承父帧的仲裁规则）

---

## 6. Domain::Custom 的字符串耦合

### 现状

`Domain::Custom("chemistry")` 和 `CustomRule { name: "chemistry" }` 通过字符串匹配关联。拼写不一致不会在编译时捕获。

### 局限

- 无编译时类型安全
- 规则名称和域名称的关联是隐式的

### 可能路径

- 编译时代码生成（`CustomRule` 的 name 字段和 `Domain::Custom` 的参数从同一个常量生成）
- 注册表模式：在运行时注册自定义域，返回句柄

---

## 7. 日志系统简单

### 现状

日志通过 `tracing` + `tracing-subscriber` 输出到 stdout。无持久化、无结构化查询、无日志轮转。

### 局限

- 长时间运行节点的日志会无限增长
- 无日志查询接口（如"显示所有 FrameMismatch 中断"）

### 可能路径

- 持久化到 SQLite（已计划用于 M5）
- OpenTelemetry 导出（用于生产可观测性）
- 日志级别动态调整（无需重启）

---

## 8. 无安全审计的独立验证

### 现状

安全审计（`docs/security-audit.md`）是自审计——由项目开发者执行，非独立第三方。

### 局限

- SafeFallback 的预置危险域列表是主观判断
- 仲裁规则的正确性未经外部专家验证
- 无渗透测试

### 可能路径

- 独立安全审计（第三方）
- 领域专家审查（医学伦理学家审查 MedicalEthics 规则，工程师审查 Physical/Engineering 规则）
- 形式化安全属性证明

---

## 9. 文档覆盖不完整

### 现状

本文档系统（2026-06）是首次系统化文档编写。之前文档是随开发过程累积的。

### 局限

- API 文档（`docs/api.md`）可能过时（需与源码同步）
- 缺少贡献者入门指南中的"良好第一个 Issue"标签
- 缺少视频/可视化教程

### 可能路径

- `cargo doc` 生成的 API 文档（docs.rs）
- 架构图的 Mermaid/Graphviz 源文件（可版本控制）
- 场景演练视频

---

## 总结

Trit-Core 在核心代数层是成熟的（冻结、测试充分、不变性验证）。在分布式协议、形式化验证、性能验证、工具链方面有明确的增长空间。每个局限都有已知的解决路径——问题不是"能否解决"，而是"优先级排序"。

如果你在某个局限领域有专长（形式化方法、分布式系统、安全审计），欢迎贡献。见 [CONTRIBUTING](../development/CONTRIBUTING.md)。
