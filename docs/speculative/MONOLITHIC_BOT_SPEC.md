# Monolithic Bot：具身智能的裸金属工程规范

> **英文标题**：Monolithic Bot: A Bare-Metal Engineering Specification for Embodied Intelligence
> **子系统的**：trit-core 认知架构的物理具身化验证平台
> **架构关系**：trit-core 提供认知框架 → Monolithic Bot 提供物理体素环境与五感管道
> **文档版本**：v0.1.0 | 2026-07-02

---

## 0. 前置说明

### 0.1 本文件在 trit-core 文档树中的位置

```
trit-core/
├── docs/
│   ├── explanation/
│   │   ├── ARCHITECTURE.md      ← trit-core 自身的认知架构
│   │   ├── CONCEPTS.md          ← trit-core 核心概念（TritValue, Frame, Phase）
│   │   ├── PHILOSOPHY.md        ← trit-core 设计哲学
│   │   └── embodiment/          ← [本目录] 具身化技术规范
│   │       └── MONOLITHIC_BOT_SPEC.md
│   ├── reference/
│   │   └── api.md               ← trit-core 公共 API
│   └── adr/                     ← trit-core 架构决策记录
└── aurora/                      ← Aurora 认知应用层
```

### 0.2 概念对齐

| trit-core 概念 | Monolithic Bot 对应 | 映射说明 |
|---------------|---------------------|---------|
| TritValue {−1, 0, +1} | Voxel::material_id (u16) | 三值逻辑 vs. 多值材质 |
| Frame | PerceptionPacket | 认知帧 ↔ 感知数据包 |
| Phase | MotorCommand | 相位状态 ↔ 运动指令 |
| MetaInterrupt | InterruptController | 元中断 ↔ 传感器中断 |
| Domain | SensorType | 领域 ↔ 传感模态 |
| EcologicalAnchor | Vec3::gravity | 生态锚 ↔ 重力方向 |
| Word | Voxel | 基本语义单元 ↔ 基本空间单元 |

---

## 1. 系统概述

### 1.1 问题陈述

当前的具身智能研究面临一个根本性的张力：**算法需要大量物理交互数据才能收敛，但物理交互的成本高、速度慢、不可回放**。Monolithic Bot 的解决方案是：在部署到物理世界之前，先在一个**体素化的元世界**中进行完整的感知-决策-行动循环训练，确保策略在模拟环境中经过充分验证。

### 1.2 设计原则

1. **因果优先于统计**：每个传感器读数都有确定的物理成因（体素材质、光照、声源），而非从概率分布中采样。
2. **管道即架构**：感知 → 决策 → 行动是硬编码的因果链，不存在分支旁路。
3. **可观测性内建**：每个组件都暴露结构化状态，支持运行时监控与事后回放。
4. **层间契约显式化**：层与层之间通过类型化的消息通道通信，不存在隐式共享状态。

---

## 2. 系统架构

### 2.1 分层结构

```
┌─────────────────────────────────────────────────────────────┐
│  L4  元世界层 (Metaverse Layer)                              │
│  • VoxelWorld (256×256×64 体素)                            │
│  • PhysicsEngine (刚体/碰撞/光线传播)                       │
│  • WorldGenerator (场景程序化生成)                          │
│  • 运行时: C++ (高性能物理) + Rust (逻辑编排)               │
├─────────────────────────────────────────────────────────────┤
│  L3  认知层 (Cognitive Layer)                                │
│  • PerceptionFusion (五感数据融合 → PerceptionPacket)       │
│  • DecisionEngine (RL Agent + SymbolicPlanner)              │
│  • MemorySystem (Working + Episodic Memory)                │
│  • 运行时: Rust (纯所有权语义, 无GC)                         │
├─────────────────────────────────────────────────────────────┤
│  L2  驱动层 (Driver Layer)                                   │
│  • SensorDriver trait (统一传感器接口)                       │
│  • ActuatorDriver trait (统一执行器接口)                     │
│  • DriverRegistry (热插拔注册表)                            │
│  • 运行时: Rust (Trait Object 多态分发)                     │
├─────────────────────────────────────────────────────────────┤
│  L1  内核层 (Kernel Layer) ←── 借 Linux 核心理念            │
│  • Scheduler (3-Pipeline CFS 调度器)                        │
│  • InterruptController (传感器事件中断分发)                 │
│  • MessageBus (MPSC 消息通道)                              │
│  • MemoryPool (固定块预分配器)                             │
│  • 运行时: Rust (no_std? 当前依赖 std)                      │
├─────────────────────────────────────────────────────────────┤
│  L0  物理层 (Physical Layer)  ←── 在仿真中由体素世界替代     │
│  • 真实传感器硬件 (部署阶段)                                │
│  • 真实执行器硬件 (部署阶段)                                │
│  • 运行时: 物理硬件                                        │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 数据流

```
[体素世界]
    │
    ├──→ VisionSensor::render_view() ──→ [RGB + Depth + Segmentation]
    ├──→ AudioSensor::capture_audio() ──→ [Waveform + MFCC]
    ├──→ TactileSensor::sample_tactile() ──→ [PressureMap + Force]
    ├──→ OlfactorySensor::sniff() ──→ [ConcentrationVector]
    └──→ TasteSensor::sample_taste() ──→ [TasteVector + pH]
                            │
                            ▼
                   PerceptionPacket
                    (五感融合)
                            │
                            ▼
                   DecisionEngine::decide()
                    (RL Agent + Planner)
                            │
                            ▼
                      Decision { Action }
                            │
                            ▼
                   MotorController::resolve_action()
                            │
                            ▼
                    MotorCommand → ActuatorDriver
                            │
                            ▼
                   [体素世界状态更新]
                            │
                            └──→ PhysicsEngine::step()
                                      │
                                      ▼
                                 下一帧
```

---

## 3. 核心子系统规范

### 3.1 裸金属内核 (Bare-Metal Kernel)

#### 3.1.1 调度器 (Scheduler)

调度器采用三管道固定优先级调度，类似于 Linux CFS 的简化实现：

| 管道 | 周期 | 时间片 | 优先级 | 功能 |
|------|------|--------|--------|------|
| Perception | 每 tick | 5ms | 1 (最高) | 传感器采样 + 数据融合 |
| Decision | 每 2 ticks | 10ms | 2 | RL 推理 + 符号规划 |
| Action | 每 tick | 5ms | 3 | 运动控制 + 执行器驱动 |

调度器保证：**在单个 physics tick (16.67ms @ 60Hz) 内，所有管道合计执行时间不超过 20ms 时不会发生帧丢失**。

```rust
// 调度器核心接口 (伪代码)
impl Scheduler {
    fn execute_tick(
        perception_fn: FnMut() -> Duration,  // L3 感知阶段
        decision_fn:   FnMut() -> Duration,  // L3 决策阶段
        action_fn:     FnMut() -> Duration   // L3 行动阶段
    ) -> u64;  // 返回本次 tick 总耗时 (微秒)
}
```

#### 3.1.2 中断控制器 (Interrupt Controller)

中断是消息驱动的传感器事件通知机制，非硬件中断。支持 9 种中断源：

| 中断源 | 触发条件 | 优先级 | 典型延迟 |
|--------|---------|--------|---------|
| VisionFrame | 视觉帧就绪 | Critical | <1ms |
| AudioFrame | 音频缓冲满 | High | <2ms |
| Collision | 体素碰撞检测 | Critical | <0.5ms |
| ActuatorDone | 执行器到位 | Normal | <5ms |
| SystemError | 异常状态 | Critical | 立即 |

#### 3.1.3 消息总线 (MessageBus)

采用 Rust 标准库 `std::sync::mpsc` 通道，支持四种消息优先级：

```
Message {
    priority: MsgPriority { Critical, High, Normal, Low },
    payload:  SensorFrame | Decision | MotorCommand | SystemEvent
}
```

### 3.2 体素世界 (VoxelWorld)

#### 3.2.1 体素规范

| 属性 | 类型 | 范围 | 物理对应 |
|------|------|------|---------|
| material_id | u16 | 0-65535 | 材质种类（当前 10 种预定义） |
| density | f32 | 0-10000 kg/m³ | 材质密度 |
| hardness | f32 | 0-10 (Mohs) | 材质硬度 |
| temperature | f32 | 0-5000 K | 热力学状态 |
| color | [u8; 3] | 0-255 RGB | 光学属性 |
| flags | u8 | 位掩码 | 状态标识（固态/液态/气态） |

#### 3.2.2 空间分区

```
世界:         256 × 256 × 64 = 4,194,304 体素
块 (Chunk):   16 × 16 × 16 = 4,096 体素/块
              → 4096 块总容量 (惰性分配)
分辨率:       0.1 m/体素 → 25.6m × 25.6m × 6.4m 物理空间
```

#### 3.2.3 材质体系 (当前 10 种)

| ID | 名称 | 密度 kg/m³ | 硬度 Mohs | 主要用途 |
|----|------|-----------|-----------|---------|
| 0 | Air | 1.225 | 0.0 | 空体素（默认） |
| 1 | Dirt | 1600 | 1.5 | 地面底层 |
| 2 | Grass | 1500 | 1.0 | 地表层 |
| 3 | Stone | 2500 | 6.0 | 障碍物/建筑 |
| 4 | Wood | 700 | 2.0 | 可操作物体 |
| 5 | Water | 1000 | 0.0 | 流体(液体状态) |
| 6 | Metal | 7800 | 5.0 | 坚固结构 |
| 7 | Glass | 2500 | 5.5 | 透明结构 |
| 8 | Sand | 1600 | 0.5 | 松散颗粒 |
| 9 | Brick | 2000 | 3.5 | 建筑结构 |

### 3.3 五感传感器系统

#### 3.3.1 👁 视觉传感器 (Vision)

- **分辨率**: 640 × 480
- **视场角**: 70°(H) × 55°(V)
- **最大距离**: 50m
- **输出**: RGB 图 + 深度图 + 语义分割图
- **工作原理**: 光线步进 (Ray Marching)，步进 0.1m
- **采样率**: 30 Hz

#### 3.3.2 👂 听觉传感器 (Audio)

- **采样率**: 44.1 kHz
- **通道数**: 2 (立体声)
- **帧大小**: 1024 采样点
- **最大距离**: 30m
- **输出**: 波形 + 梅尔频谱（可选）
- **采样率**: 匹配音频采样率

#### 3.3.3 ✋ 触觉传感器 (Tactile)

- **压力阵列**: 32 × 32
- **最大压力**: 100 kPa
- **附加**: 力/力矩 3 轴 + 温度
- **采样率**: 100 Hz

#### 3.3.4 👃 嗅觉传感器 (Olfactory)

- **气味类型**: 10 种（中性/甜/酸/烧焦/花香/果香/腐臭/薄荷/化学/烟熏）
- **最大距离**: 20m
- **灵敏度**: 0.1 ppm
- **采样率**: 10 Hz

#### 3.3.5 👅 味觉传感器 (Taste)

- **基本味觉**: [甜, 酸, 苦, 咸, 鲜] 5 维向量
- **附加**: pH, 导电率, TDS, 温度
- **采样率**: 5 Hz

### 3.4 决策引擎

#### 3.4.1 强化学习代理 (RL Agent)

| 参数 | 值 | 说明 |
|------|-----|------|
| 算法 | DQN (线性近似) | 当前实现，后续可替换为 PPO |
| 状态维度 | 512 | 感知编码后向量 |
| 动作维度 | 32 | 离散动作空间 |
| 学习率 | 0.001 | Adam 默认 |
| 折扣因子 γ | 0.99 | 长期回报权重 |
| 探索率 ε | 0.1 → 退火 | ε-贪婪策略 |
| 回放缓冲区 | 100,000 | 经验回放容量 |

#### 3.4.2 动作空间

| 索引 | 动作 | 说明 |
|------|------|------|
| 0 | Idle | 等待 1s |
| 1 | MoveTo | 移动到 (x, y) 位置 |
| 2 | Explore | 随机探索 |
| 3 | LookAt | 旋转传感器朝向 |
| 4 | Grasp | 抓取物体 |
| 5 | Release | 释放物体 |
| 6 | Stop | 急停 |
| 7 | Speak | 语音输出 |

### 3.5 运动控制系统

#### 3.5.1 躯体配置

| 属性 | 值 |
|------|-----|
| 躯体名称 | HumanoidBot |
| 自由度 | 20 (关节) |
| 质量 | 60 kg |
| 身高 | 1.75 m |
| 关节限位 | ±3.14 rad (各轴) |
| 最大关节速度 | 3.0 rad/s |
| 最大关节力矩 | 100 Nm |

---

## 4. 与 trit-core 的集成接口

### 4.1 认知层适配器

Monolithic Bot 的 `PerceptionPacket` 可以直接映射到 trit-core 的 `Frame` 类型：

```rust
// trit-core 视角下的 Monolithic Bot 适配器 (伪代码)
trait EmbodiedFrame {
    fn to_trit_frame(&self) -> trit_core::Frame;  // 五感 → 三值逻辑帧
    fn from_trit_action(&self, action: trit_core::Action) -> monolithic_bot::Action;
}

// 域映射
impl EmbodiedDomain for VisionSensor {
    fn domain_id() -> DomainId { "vision::raycast" }
    fn to_trinary(&self, data: &VisionData) -> TritVector {
        // 将 RGB 深度映射到 {−1, 0, +1}
    }
}
```

### 4.2 相位集成

Monolithic Bot 的 `MotorCommand` 对应 trit-core 的 `Phase`。trit-core 的相位算术（Phase Arithmetic）可以直接用于计算运动轨迹的中间状态：

```
Phase(t) = Phase(0) + t * (Phase(1) − Phase(0))
         → MotorCommand 插值
```

### 4.3 元中断映射

| trit-core MetaInterrupt | Monolithic Bot InterruptSource | 触发条件 |
|------------------------|-------------------------------|---------|
| DomainOverflow | VisionFrame | 视觉数据溢出 |
| RuleConflict | Collision | 碰撞检测冲突 |
| SafeFallback | SystemError | 系统异常 |
| FrameMaskTrigger | (可扩展) | 感知帧掩码 |

---

## 5. 验证与基准

### 5.1 系统规模

| 指标 | 值 |
|------|-----|
| 总代码行数 | 4,505 行 |
| Rust 源码 | ~3,800 行 (28 文件) |
| C++ 头文件 | ~150 行 (1 文件) |
| 文档 | ~440 行 (2 文件) |
| 模块数 | 8 主模块 |
| 编译检查状态 | ✅ 零错误通过 |

### 5.2 运行时性能指标 (开发模式)

| 操作 | 平均耗时 | 测量条件 |
|------|---------|---------|
| 调度器单 tick | <50 μs | debug build |
| 视觉渲染 (640×480) | ~15 ms | 全光线步进 |
| 决策推理 (DQN) | <100 μs | 线性 Q 网络 |
| 经验回放训练 | <50 μs/batch | batch=32 |
| 体素世界步进 | <5 μs | 无物理(当前) |

---

## 6. 演进路线图

| 阶段 | 目标 | 与 trit-core 的关系 |
|------|------|-------------------|
| M0 当前 | 裸金属核心 + 体素世界 + 五感 + 决策管道 | 概念验证 |
| M1 | C++ 物理引擎集成 (碰撞/刚体) | 为 trit-core 提供物理真实感 |
| M2 | ONNX Runtime 集成 (GPU 推理) | 替换线性 Q 为深度网络 |
| M3 | 训练场景自动化 + 评估套件 | 作为 trit-core 的验证平台 |
| M4 | 物理机器人部署桥接 | trit-core 认知 → 物理行动 |

---

## 附录 A：关键数据结构形式化定义

### A.1 Voxel (64 位编码)

```
Bit:  63 .. 48    47 .. 32    31 .. 16    15 . 12    11 ..  0
     ┌──────────┬──────────┬──────────┬─────────┬───────────┐
     │ material │ density  │ hardness │  flags  │ temp(K)   │
     │   id     │  (u16)   │  (u16)   │  (u8)   │  (i16)    │
     │  (u16)   │          │          │         │           │
     └──────────┴──────────┴──────────┴─────────┴───────────┘
      + 24 bits RGB color (3 × u8)
      Total: 64 bits
```

### A.2 PerceptionPacket 时序

```
[ t=0    ] VisionFrame 触发
[ t+1ms  ] AudioFrame 触发
[ t+2ms  ] TactileFrame 触发
[ t+3ms  ] PerceptionPacket 组装完成
[ t+3.5ms] DecisionEngine::decide() 开始
[ t+5ms  ] Decision 输出
[ t+5.5ms] MotorController::execute() 开始
[ t+6ms  ] MotorCommand 发送至执行器
           ↓
[ t+16.67ms] 下一个 tick 开始
```

---

*本文档已纳入 trit-core 双螺旋知识库。交叉引用：[`explanation/ARCHITECTURE.md`](../ARCHITECTURE.md) · [`explanation/CONCEPTS.md`](../CONCEPTS.md) · [`adr/003-domain-conflict.md`](../../adr/003-domain-conflict.md)*
