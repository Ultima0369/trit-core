# Embodiment 具身化知识域 — 文档索引

> **域定位**：trit-core 抽象认知架构与物理世界的接口层
> **核心问题**：三值逻辑认知如何与连续物理世界对接？
> **文档版本**：v0.1.0

---

## 域概述

本目录 (`embodiment/`) 收录所有涉及"认知下凡"的文档——即 trit-core 的三值逻辑认知框架如何与物理世界、传感器、执行器和物理模拟交互的技术规范。

---

## 文档清单

| 文件 | 内容 | 状态 |
|------|------|------|
| [`MONOLITHIC_BOT_SPEC.md`](MONOLITHIC_BOT_SPEC.md) | Monolithic Bot 具身智能体素系统完整工程规范 | ✅ v0.1.0 |
| [`INDEX.md`](INDEX.md) (本文件) | 知识域索引 | ✅ v0.1.0 |

---

## 跨域引用

### 上游依赖 (trit-core 核心)

| 文档 | 引用内容 |
|------|---------|
| [`../ARCHITECTURE.md`](../ARCHITECTURE.md) | trit-core 认知架构总体设计 |
| [`../CONCEPTS.md`](../CONCEPTS.md) | Frame/Phase/Domain 概念定义 |
| [`../PHILOSOPHY.md`](../PHILOSOPHY.md) | 设计哲学（具身化动机） |
| [`../../adr/003-domain-conflict.md`](../../adr/003-domain-conflict.md) | 领域冲突检测算法（用于传感器融合） |

### 下游依赖 (Aurora 应用层)

| 文档 | 引用内容 |
|------|---------|
| [`../../../aurora/03_whitepaper/ARCHITECTURE.md`](../../../aurora/03_whitepaper/ARCHITECTURE.md) | Aurora 系统架构 |
| [`../../../aurora/07_specs/TRIT_CORE_INTEGRATION_SPEC.md`](../../../aurora/07_specs/TRIT_CORE_INTEGRATION_SPEC.md) | trit-core 集成规范 |

### 并行域

| 文档 | 关系 |
|------|------|
| [`../insights/EULER-HOMOLOGY.md`](../insights/EULER-HOMOLOGY.md) | 欧拉恒等式的认知架构类比（与体素世界的同源性） |
| [`../../../map/04_math.md`](../../../map/04_math.md) | 数学基础 MOC（体素坐标变换、光线投射几何） |

---

## 核心学术问题

本域试图回答以下问题：

1. **离散-连续接口**：三值逻辑 `{−1, 0, +1}` 如何编码连续传感器数据（如 640×480 图像）？
   → 当前方案：先降维再三值化。论文参考：*Temporal Abstraction in Reinforcement Learning*, Sutton 1999

2. **认知帧与物理同步**：trit-core 的 Frame 周期（约 100ms）与物理 tick (16.67ms) 如何对齐？
   → 当前方案：每 6 个 physics tick 触发一次 Frame 更新。论文参考：*Sensorimotor Coordination*, Kappen 2018

3. **相位空间覆盖**：运动控制相位空间 (`Phase`) 与体素世界的连续坐标 (`Vec3`) 如何映射？
   → 当前方案：Phase 作为 MotorCommand 的抽象，Vec3 作为底层坐标。论文参考：*Dynamical Systems Approach to Cognition*, Thelen & Smith 1994

---

## 路线图

| 里程碑 | 目标 | 预计 |
|--------|------|------|
| M0 | Monolithic Bot 工程规范发布 ✅ | 当前 |
| M1 | 物理引擎集成 + 碰撞检测 | Phase 1 |
| M2 | ONNX Runtime + 深度网络推理 | Phase 3 |
| M3 | 物理机器人部署桥接 | Phase 5 |
| M4 | trit-core 认知引擎直接驱动 | 待定 |

---

*维护者：Embodiment Domain Steward*
*对齐基准：trit-core v0.3.0*
