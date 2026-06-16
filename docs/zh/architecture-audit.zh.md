# 架构审计报告 v1.0

**审计日期**：2026-06-17  
**审计人**：P8/L7 系统架构师（碳硅协作）  
**被审计项目**：Trit-Core MVP  
**审计范围**：架构风格、限界上下文、接口契约、数据一致性、非功能性需求、风险评估

---

## 1. 架构风格选择

| 维度 | 评估结果 |
|------|---------|
| **当前风格** | 模块化单体（Modular Monolith） |
| **选择理由** | MVP 阶段无分布式通信需求。`src/trit/`、`src/meta/`、`src/frame/` 已呈现清晰模块边界，但运行于单一进程。避免了微服务在 MVP 阶段的网络开销、部署复杂度与分布式事务负担。 |
| **适用场景** | 单机决策引擎、CLI 工具、嵌入式库。 |
| **演进路径** | M0–M1：保持模块化单体。M2 引入多节点时，将 `sandbox/` 拆分为 REST API Gateway，`net/` 升级为独立微服务（Trit-Node），`trit/` 与 `meta/` 保持为共享核心库（crate 依赖）。 |
| **架构决策** | 当前不引入 Docker/Kubernetes。M3 规模化后再评估。 |

---

## 2. 系统架构图（C4 Model Level 2 — 容器级文字描述）

```
┌─────────────────────────────────────────────┐
│  外部用户 / 外部系统                           │
│  (CLI 调用者、Web 前端、下游 AI 服务)            │
│         │                                    │
│         │ HTTP / gRPC (M2+)                  │
│         ▼                                    │
│  ┌─────────────────────────────────────────┐ │
│  │  应用容器：Sandbox CLI / REST API        │ │
│  │  (src/bin/sandbox.rs, 未来扩展 REST)     │ │
│  │  职责：输入解析、场景编排、结果序列化       │ │
│  │         │                               │ │
│  │         │ 函数调用（进程内）               │ │
│  │         ▼                               │ │
│  │  ┌────────────────────────────────────┐ │ │
│  │  │  领域容器：决策引擎（Decision Engine）│ │
│  │  │  ┌──────────┐  ┌──────────┐      │ │ │
│  │  │  │ 三值代数  │  │ 策略仲裁  │      │ │ │
│  │  │  │ Trit ALU │  │ Meta-    │      │ │ │
│  │  │  │ (trit/)  │◄─┤ Monitor  │      │ │ │
│  │  │  └──────────┘  │ (meta/)  │      │ │ │
│  │  │       ▲        └──────────┘      │ │ │
│  │  │       │              ▲             │ │ │
│  │  │  ┌────┴────┐    ┌────┴────┐       │ │ │
│  │  │  │参考系注册│    │谐波时钟 │       │ │ │
│  │  │  │(frame/) │    │(clock/) │       │ │ │
│  │  │  └─────────┘    └─────────┘       │ │ │
│  │  └────────────────────────────────────┘ │ │
│  │         │                               │ │
│  │         │ 文件 I/O                       │ │
│  │         ▼                               │ │
│  │  ┌────────────────────────────────────┐ │ │
│  │  │ 数据容器：决策日志（JSONL）         │ │ │
│  │  │ 路径：logs/ or stdout             │ │ │
│  │  └────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

**关键边界**：`trit/` 与 `meta/` 是**核心域（Core Domain）**，不允许被外部直接调用。`sandbox/` 是**应用层（Application Layer）**，负责编排。

---

## 3. 限界上下文划分（Bounded Context）

| 上下文 | 英文标识 | 职责 | 聚合根 | 暴露接口 |
|--------|---------|------|--------|---------|
| **三值代数** | `trit-core.algebra` | 定义三态数据结构与运算规则 | `TritWord` | `TernaryAlgebra::t_and/or/not` |
| **参考系注册** | `trit-core.frame` | 管理决策域的上下文身份 | `FrameRegistry` | `register()`, `is_registered()` |
| **策略仲裁** | `trit-core.policy` | 检测冲突、执行域规则、产出仲裁 | `ResolutionPolicy` | `arbitrate()`, `MetaMonitor` |
| **场景应用** | `trit-core.sandbox` | 解析输入、编排运算、格式化输出 | `ScenarioInput` | `Sandbox::run()` |
| **谐波时钟** | `trit-core.clock` | 管理时间尺度与相位采样 | `HarmonicClock` | `tick()`, `phase_now()` |
| **分布式网络** | `trit-core.net` | M2+ 多节点耦合与解耦 | `Node` | `resonate()`, `decouple()` |

**上下文映射**：
- `algebra` → `frame`：依赖（运算时检查 frame 是否匹配）
- `policy` → `algebra`：依赖（调用运算后仲裁）
- `sandbox` → `policy` + `algebra`：依赖（编排层）
- `net` → `policy`：依赖（分布式节点策略）

**无循环依赖** ✅ 符合工程铁律 #5。

---

## 4. 接口契约（OpenAPI 风格 + 内部 Trait）

### 4.1 外部 REST API（M2 扩展建议）

```yaml
openapi: 3.0.0
info:
  title: Trit-Core Decision API
  version: 1.0.0
paths:
  /api/v1/decision:
    post:
      summary: 提交决策场景
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ScenarioInput'
      responses:
        '200':
          description: 决策成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/SandboxOutput'
        '409':
          description: 冲突无法仲裁（域规则拒绝）
        '422':
          description: 输入参数非法（如 phase 越界）
      x-trit-core-policy:
        domain: MedicalEthics  # 强制指定域，防止默认漂移
```

### 4.2 内部接口契约（Rust Trait）

```rust
// 核心域接口（不允许外部直接调用）
pub trait TernaryAlgebra {
    fn t_and(&self, a: &TritWord, b: &TritWord) -> Result<(TritWord, Option<MetaInterrupt>), AlgebraError>;
    fn t_or(&self, a: &TritWord, b: &TritWord) -> Result<(TritWord, Option<MetaInterrupt>), AlgebraError>;
    fn t_not(&self, a: &TritWord) -> TritWord;
}

// 策略引擎接口
pub trait ArbitrationPolicy {
    fn arbitrate(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError>;
    fn inspect(&self, word: &TritWord) -> Result<(), MetaInterrupt>;
}
```

**版本策略**：SemVer。0.1.x 内保持向后兼容；0.2.0 允许重构 `net/` 与 `sandbox/`。

### 4.3 错误模型

| 错误码 | 含义 | 场景 |
|--------|------|------|
| `AlgebraError::PhaseOutOfBounds` | 相位越界 [0,1] | 输入非法 |
| `AlgebraError::FrameNotRegistered` | 参考系未注册 | 域隔离漏洞 |
| `PolicyError::Incommensurable` | 不可通约 | ValueJudgment 域冲突 |
| `PolicyError::ForcedCollapseDenied` | 强制坍缩被域规则拒绝 | MedicalEthics 保护个体 |

---

## 5. 数据一致性方案

| 层级 | 一致性要求 | 方案 | 说明 |
|------|-----------|------|------|
| **运算状态** | 强一致性（内存级） | 单进程，无并发锁 | 当前架构天然满足 |
| **决策日志** | 最终一致性 | 追加写 JSONL（顺序 I/O） | 崩溃时可能丢失最后一条，MVP 可接受 |
| **参考系注册** | 强一致性 | 启动时加载，运行时只读 | 无运行时变更需求 |
| **域规则** | 强一致性 | 硬编码，编译期确定 | 防止运行时篡改 |
| **分布式状态（M2+）** | 最终一致性 | CRDT 或 Event Sourcing | 节点相位同步允许短暂漂移 |

**无分布式事务**：MVP 无跨进程通信。M2 引入分布式后，采用 **Saga 模式**（补偿机制：若分布式共振失败，回退为独立节点运行）。

---

## 6. 非功能性需求矩阵（NFR）

| 维度 | 量化目标 | 测量方法 | 当前状态 |
|------|---------|---------|---------|
| **性能** | 单核 10,000 TPS | `cargo bench` 基准测试 | 未实现 |
| **延迟** | P99 < 1ms | 集成测试计时 | 未实现 |
| **可用性** | 进程级 99.9%（无 HA） | 无外部依赖，天然可用 | ✅ 当前满足 |
| **内存** | 1000 并发 < 50MB RSS | `valgrind` / `heaptrack` | 未实现 |
| **安全** | 最小权限（无 unsafe） | `cargo geiger` / `cargo audit` | ✅ 已 enforce |
| **可观测性** | 结构化日志 + 指标 | `tracing` + `metrics` crate | 未实现 |
| **向后兼容** | 0.1.x 内 API 不 breaking | CI 检查 | 未实现 |
| **数据 retention** | 日志保留 30 天 | 磁盘轮转策略 | 未实现 |

**缺口**：当前缺少性能测试框架、可观测性埋点、安全审计（依赖漏洞扫描）。

---

## 7. 风险评估

### 7.1 单点故障（P1）

| 风险点 | 影响 | 缓解 | 优先级 |
|--------|------|------|--------|
| `sandbox.rs` 主进程崩溃 | 全部决策服务中断 | 进程级监控（systemd），M2 引入多副本 | P1 |
| 浮点相位精度漂移 | 多次运算后相位累积误差 | 定点数替代（future ADR），或 epsilon 容差 | P2 |
| JSONL 日志损坏 | 审计链断裂 | 每行校验和 + 定期归档 | P2 |

### 7.2 技术债务（P2）

| 债务项 | 原因 | 还款计划 |
|--------|------|---------|
| `phase: f64` | 精度问题、嵌入式不友好 | M3 引入定点数或 `u16` 映射 |
| `net/` 模块空实现 | 架构占位，无实际代码 | M2 填充或删除，防止腐化 |
| 无 CI/CD | 无法保证可复现性 | GitHub Actions 接入 `cargo test` + `clippy` |
| 无可观测性 | 无法线上排查 | 引入 `tracing` + `prometheus` 指标 |

### 7.3 架构腐化预警（S2）

| 信号 | 说明 | 应对措施 |
|------|------|---------|
| `net/` 被随意填充 | 可能破坏核心域边界 | 强制 code review：net/ 只能依赖 meta/，不能反向依赖 |
| 域规则硬编码僵化 | 无法适配新场景 | M3 引入 Domain Rule DSL（YAML 或 Lua） |
| 外部 API 直接调用 `trit/` | 绕过策略层 | 编译期限制：核心 crate 标记 `#[doc(hidden)]` 或 `pub(crate)` |

---

## 8. 工程铁律对照

| 铁律 | 当前状态 | 改进项 |
|------|---------|--------|
| 可复现性 | 场景 JSON 可复现，但无变更日志 | 增加 `CHANGELOG.md` + ADR 编号 |
| 测试先行 | 有集成测试，覆盖率未量化 | `cargo tarpaulin` 接入，目标 ≥ 80% |
| 无感知回归 | 无 CI/CD | GitHub Actions 跑 `cargo test` + `cargo bench` |
| 可观测性 | 无结构化日志 | 引入 `tracing` crate |
| 最小权限 | 无循环依赖 ✅ | 核心 crate 限制可见性 |
| 契约优先 | 已有 `docs/api.md` ✅ | 补充 OpenAPI YAML |
| 零技术债 | 有 phase float 债 | 排入 M3 还款计划 |

---

## 9. 架构师结论与行动建议

**整体评价**：Trit-Core 是一个**具有原始创新力的模块化单体**。核心域（trit/ + meta/）的边界清晰，符合 DDD 原则。但当前处于**工程成熟度 L1**（原型可运行），距离**工业级 L3**（可上线、可运维、可扩展）有明确差距。

**三个必须立即行动项**：

1. **接入 CI/CD**（GitHub Actions）：每次 PR 自动跑 `cargo test` + `cargo clippy` + `cargo deny`（安全检查）。这是可复现性的底线。
2. **引入可观测性框架**：`tracing` 用于结构化日志，`metrics` 用于计数/延迟指标。否则无法验证 NFR。
3. **定义性能基准**：`benches/` 目录 + `criterion` crate，测量当前 TPS 与延迟，建立回归基线。

**三个可延后项**（M2+）：

1. 外部 REST API（OpenAPI 规范）。
2. 分布式网络层（net/）实现。
3. 形式验证（Coq/Lean）。

---

**审计完成。项目骨架健康，但工程基础设施需要加固。**
