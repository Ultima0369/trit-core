# ARCHITECTURE — Trit-Core 系统架构

本文档描述 Trit-Core 的分层架构、数据流和关键设计决策。

---

## 1. 分层架构

```
┌──────────────────────────────────────────────────┐
│  应用层 (CLI / API)                               │
│  trit-sandbox, trit-node 二进制                   │
├──────────────────────────────────────────────────┤
│  沙箱引擎 (sandbox/)                               │
│  场景解析 → TAND 级联 → 策略仲裁 → JSON 输出        │
├──────────────────────────────────────────────────┤
│  策略引擎 (meta/)                                  │
│  Domain → ResolutionPolicy → SafeFallback          │
│  FrameMask (O(1) 位掩码)                           │
├──────────────────────────────────────────────────┤
│  三元 ALU (trit/)                                  │
│  TernaryAlgebra: TAND, TOR, TNOT, THOLD, TSENSE    │
│  Phase 算术: mean, complement, quantize            │
├──────────────────────────────────────────────────┤
│  帧注册表 (frame/)                                  │
│  Frame 枚举 + FrameRegistry                        │
├──────────────────────────────────────────────────┤
│  数据模型 (trit/)                                   │
│  TritWord { value: TritValue, phase: Phase,        │
│             frame: Frame }                         │
└──────────────────────────────────────────────────┘
```

---

## 2. 热路径 vs 冷路径

这是 Trit-Core 最关键的微架构设计。

### 2.1 热路径（Hot Path）

**条件**：两个 TritWord 共享同一个 Frame。

**行为**：
- 标准三值真值表（TAND/TOR）
- Phase 取算术均值
- 不分配 MetaInterrupt
- 不触发 MetaMonitor

**性能**：约 3ns/操作（分支无关 LUT + 一次浮点加法 + 一次除法）

**占比**：典型决策中约 80% 的操作走热路径。

### 2.2 冷路径（Cold Path）

**条件**：两个 TritWord 的 Frame 不同。

**行为**：
- 返回 Hold + MetaInterrupt
- 记录冲突类型、原因、时间戳
- 触发后续策略仲裁

**性能**：约 95ns/操作（包含 String 分配和时间戳获取）

**占比**：约 20% 的操作走冷路径。跨域冲突在现实场景中不是常态，但必须被检测。

### 2.3 为什么这样设计？

热/冷路径分离遵循一个核心洞察：**大多数决策发生在同一个参考系内，不需要元认知开销。** 当两个科学家讨论同一个实验数据时，他们不需要先协商"我们是否在同一个认知框架中"——他们直接计算。只有当经济学家和生态学家讨论同一个政策时，才需要元层面的仲裁。

---

## 3. 数据流

```
JSON 场景文件
      │
      ▼
ScenarioInput (serde 反序列化)
      │
      ▼
SignalInput[] → TritWord[]
      │
      ▼
TAND 级联（从左到右折叠）
  ├── 同帧 → 热路径：真值表 + Phase 均值
  └── 跨帧 → 冷路径：Hold + MetaInterrupt
      │
      ▼
MetaInterrupt[] 收集
      │
      ▼
ResolutionPolicy::arbitrate()
  ├── FrameMask 位掩码 O(1) 帧检测
  ├── Domain 特定规则
  └── 返回 ArbitrationResult
      │
      ▼
SafeFallback::guard()
  ├── 检查域是否危险
  ├── 检查结果是否为 Hold/Unknown
  ├── 检查中断计数 > 0
  └── 必要时强制 False
      │
      ▼
SandboxOutput (JSON 序列化)
```

---

## 4. FrameMask — O(1) 帧检测

```rust
pub(crate) struct FrameMask(u8);
```

5 个 Frame 各占一个 bit：

| Frame | 位 |
|---|---|
| Science | bit 0 |
| Individual | bit 1 |
| Consensus | bit 2 |
| Absolute | bit 3 |
| Meta | bit 4 |

操作：
- `from_inputs()`：一次遍历，O(n)，设置对应位
- `has(frame)`：位与运算，O(1)
- `count()`：`popcount` 指令，O(1)

当所有 5 位都设置时（`mask == 0b11111`），提前退出遍历。

---

## 5. ResolutionPolicy — 域仲裁

```rust
pub fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult
```

仲裁结果：

| 结果 | 含义 |
|---|---|
| `Commit(TritWord)` | 提交到特定 TritWord |
| `Preserve(TritWord)` | 保留特定 TritWord（MedicalEthics） |
| `ForceCollapse` | 强制安全坍缩（交由 SafeFallback 处理） |
| `Hold` | 有意暂停判断 |
| `Negotiate` | 尝试多轮协商 |

### 5.1 仲裁逻辑

```
Physical/Engineering:
  Science 存在 → Commit(Science 信号)
  Science 不存在 → ForceCollapse

MedicalEthics:
  Individual 存在 → Preserve(Individual 信号)
  Individual 不存在 → Negotiate

ValueJudgment:
  无条件 → Hold

General:
  单一帧 → Commit(第一个信号)
  多帧 → Negotiate

Custom(name):
  无条件 → Negotiate（由外部 RuleLoader 覆盖）
```

---

## 6. SafeFallback — IEC 61508 安全原则

### 6.1 触发条件（三个条件必须同时满足）

1. Domain 是危险的（Physical、Engineering、chemistry、genetics、structural、nuclear、pharmaceutical）
2. 仲裁结果是 Hold 或 Unknown
3. 存在至少一个 MetaInterrupt

### 6.2 行为

强制将结果改为 `False`，并生成 `OutOfScope` 类型的 MetaInterrupt。

### 6.3 为什么 MedicalEthics 不是危险的？

患者自主权（Individual frame）本身就是安全默认。在医疗场景中，"不做"（不治疗）可能比"做"（强制治疗）更危险。因此 MedicalEthics 不触发 SafeFallback——它通过 Preserve(Individual) 机制来保护患者自主权。

---

## 7. 分布式协议（M4–M8）

### 7.1 Node 状态机

```
Sovereign → Coupling → Coupled → Hold
    ↑                      │
    └────── 解耦 ──────────┘
```

### 7.2 消息类型

| 操作码 | 方向 | 含义 |
|---|---|---|
| RESONATE_REQ | 请求方→目标 | 请求相位耦合 |
| RESONATE_ACK | 目标→请求方 | 确认耦合（含干扰类型） |
| DECOUPLE_REQ | 任意→目标 | 请求解耦 |
| DECOUPLE_ACK | 目标→请求方 | 确认解耦（恢复主权相位） |
| NEGOTIATE | 总线广播 | 多节点协商结果 |
| HEARTBEAT | 节点→总线 | 存活信号 |

### 7.3 PLL 锁相环

软件锁相环用于耦合节点的相位同步：

- 比例增益 `kp = 0.3`
- 死区 `deadband = 0.05`（忽略微小相位差，防止振荡）
- 最大单步校正 `max_correction = 0.1`
- 冲突相位差阈值：`|phase_a - phase_b| > 0.3`

### 7.4 TCP 传输层（M5）

Trit-Core 支持通过 TCP 进行真实网络通信，替代内存中的 ResonanceBus。

**帧协议**：
```
| 0..4         | 4..(4+len)   |
|--------------|--------------|
| len (u32 BE) | JSON 负载    |
```
- 最大帧大小：1 MiB（CWE-770 防护）
- 4 字节大端长度前缀 + JSON 负载
- 设计理由：二进制安全（JSON 可能包含换行符），允许零拷贝预分配

**组件**：
- `TcpNodeServer`：监听 TCP 端口，接受连接，分发消息到本地 ResonanceBus
- `TcpClient`：连接远程节点，发送 REQ 并接收 ACK
- `frame_codec`：`read_frame()` / `write_frame()` 底层帧编解码

**架构**：
```
Node A (TcpNodeServer)          Node B (TcpClient)
  ├── ResonanceBus                 │
  ├── TcpListener                  │
  │    ├── conn 1 ←──────────── TcpStream ──→ connect()
  │    │    ├── read_frame()                   ├── write_frame(REQ)
  │    │    ├── dispatch_message()             ├── read_frame(ACK)
  │    │    └── write_frame(ACK)               └── ...
  │    └── conn 2 ← ...
  └── ...
```
节点可以同时作为服务端（接受其他节点的连接）和客户端（主动连接其他节点）。

### 7.5 从 M4 到 M6 的演进

| 维度 | M4（内存总线） | M5（TCP 传输层） | M6（种子发现） | M7（分区容错） | M8（拜占庭容错） |
|---|---|---|---|---|---|
| 通信方式 | ResonanceBus（内存） | TCP（网络） | TCP + 自动拓扑 | TCP + 心跳监控 | TCP + 门卫验证 |
| 节点发现 | 手动注册 | TCP 连接 | 种子引导 + HEARTBEAT 交换 | 种子 + 心跳超时检测 | 已知节点白名单 |
| 并发模型 | 单线程 | tokio 多线程 | tokio 多线程 | tokio 多线程 | tokio 多线程 |
| 适用范围 | 本地模拟、测试 | 真实分布式部署 | 动态集群、Docker Compose | 不可靠网络 | 对抗性网络 |
| 内存上限 | 256 节点、10K 消息 | 无硬性上限 | 无硬性上限 | 无硬性上限 | 无硬性上限 |
| 容错能力 | 无 | 连接超时 | 种子不可达时优雅降级 | 心跳超时 + 脑裂检测 | 拜占庭节点过滤 |
| 安全模型 | 无 | TCP 帧大小限制 | 已知节点集合 | 分区恢复 | 7 重验证 + 速率限制 |

### 7.6 种子节点发现（M6）

M6 引入了自动对等节点发现机制，使节点无需事先知道彼此地址即可形成网格。

**启动流程**：
1. 节点启动时解析 `--peers` 标志或 `TRIT_PEERS` 环境变量（逗号分隔的 `host:port` 列表）
2. 对每个种子地址发起 TCP 连接，交换 HEARTBEAT 消息
3. 成功联系的种子节点被注册到本地 `ResonanceBus`
4. 如果所有种子均不可达，节点以优雅独立模式运行

**Docker Compose 全网格**：
```
Node A (Science, :9000) ←→ Node B (Individual, :9001)
    ↕                           ↕
Node C (Consensus, :9002) ←──────┘
```
每个节点以其他两个节点为种子启动，形成 3 节点全连接 TCP 网格。

**关键函数**：
- `parse_seeds("host1:9000,host2:9001")` → `Vec<String>` 解析种子列表
- `bootstrap(&bus, local_id, &seeds)` → 返回成功联系的对等节点数

### 7.7 网络分区容错（M7）

M7 引入了心跳监控和分区检测机制，使集群在网络不可靠时仍能安全运行。

**心跳超时**：
- 每个节点定时发送 HEARTBEAT 消息
- 30 秒无心跳视为对等节点失联（`HEARTBEAT_TIMEOUT_SECS`）
- 60 秒无响应触发脑裂检测（`SPLIT_BRAIN_TIMEOUT_SECS`）

**关键函数**：
- `stale_peers()` → 返回超时未心跳的对等节点列表
- `purge_stale_peers()` → 移除失联节点
- `detect_split_brain()` → 检测网络分区（脑裂）

**TcpClient 增强**：
- 连接超时（5s）、读超时（30s）、写超时（10s）
- BufReader/BufWriter 重写，支持多消息会话

### 7.8 拜占庭容错（M8）

M8 在 TCP 反序列化与总线分发之间插入了 `ByzantineGatekeeper` 验证层。

**7 重安全检查**：
1. 发送者 ID 验证（非空、非空白、≤128 字符）
2. 已知节点检查（可选，按节点集合白名单）
3. 相位值范围验证（[0.0, 1.0]，有限，非 NaN）
4. 帧名称验证（必须匹配已知 Frame 变体）
5. 负载一致性验证（数组长度匹配、非空参与者）
6. 速率限制（每对等节点每窗口 100 条消息）
7. 每对等节点日志上限（1000 条）

**架构**：
```
TCP 反序列化 → ByzantineGatekeeper::validate() → ResonanceBus::dispatch()
                   ↓ 拒绝
              REJECTED 响应
```

**设计原则**：
- 门卫是可选的（`ResonanceBus` 持有 `Option<ByzantineGatekeeper>`）
- 禁用时零开销（保持向后兼容）
- 速率限制窗口过期后自动重置
- `register_node()` / `unregister_node()` 管理已知节点白名单

---

## 8. 关键设计约束

1. **`#![forbid(unsafe_code)]`** — 零 unsafe 代码
2. **`#![deny(warnings)]`** — 警告即错误
3. **核心代数冻结** — `trit/` 模块的 TAND/TOR/TNOT 语义在 0.1.x 中不可变
4. **跨帧操作不强制二元决策** — 始终产生 Hold + MetaInterrupt
5. **Absolute 帧必须永远 Hold** — 由 MetaMonitor::inspect() 强制执行
6. **Phase 构造时钳制 NaN/Inf** — 防止浮点异常传播
