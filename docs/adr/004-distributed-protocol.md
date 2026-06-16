# ADR-004: T_RESONATE / T_DECOUPLE 分布式节点协议

## 状态
提议中（Proposed）

## 背景

Trit-Core 的单节点沙盒已在 M1–M2 完成验证。但现实中的参考系冲突往往跨越多个独立决策节点——例如医疗 AI 节点（持有患者个体数据）、医院药典节点（持有临床试验证据）、公共卫生节点（持有群体统计数据）。这些节点必须能够**在不丧失主权的前提下**进行耦合协商。

我们需要一个分布式协议，使得每个节点保持其参考系（Frame）不变，通过相位共振/解耦操作来发现冲突并生成 Hold 或协商结果。

## 决策

采用 **T_RESONATE（共振）** 和 **T_DECOUPLE（解耦）** 两种操作，基于谐波相位锁相环（Harmonic PLL）实现节点间耦合。

### 协议版本
`trit-proto/1.0`，JSON 编码，通过 TCP/WebSocket 传输。

---

## 协议详细设计

### 1. 节点模型

每个节点是独立的 Rust 进程：

```
Node {
    id: Uuid,
    frame: Frame,          // 不可变的参考系
    sovereign_phase: f64,  // 节点本征相位
    peers: Vec<PeerHandle>, // 已耦合的对等节点
    state: NodeState,      // 当前状态
    clock: HarmonicClock,   // 本征振荡器
}
```

### 2. 消息格式

所有消息统一封装：

```json
{
  "header": {
    "proto": "trit-proto/1.0",
    "msg_id": "uuid",
    "timestamp": "ISO8601 UTC",
    "sender": "node-id"
  },
  "payload": { ... }
}
```

### 3. 协议操作

#### 3.1 RESONATE_REQ（共振请求）

向目标节点发起耦合请求，携带本节点相位和参考系：

```json
{
  "header": { ... },
  "payload": {
    "op": "RESONATE_REQ",
    "frame": "Science",
    "phase": 0.82,
    "history": [0.81, 0.83, 0.82]  // 最近 3 个相位采样（防抖动）
  }
}
```

#### 3.2 RESONATE_ACK（共振应答）

目标节点接收后计算耦合结果并返回：

```json
{
  "payload": {
    "op": "RESONATE_ACK",
    "coupled_phase": 0.67,
    "interference": "constructive",    // constructive | destructive | neutral
    "conflict_detected": false,
    "recommendation": "continue"
  }
}
```

**共振算法**：

```
coupled_phase = (node_a.phase + node_b.phase) / 2.0

if 同帧（same frame）:
    interference = constructive  → 相位叠加增强
    conflict_detected = false
    recommendation = "commit" 或 "continue"

if 异帧（different frame）:
    if |phase_a - phase_b| > 0.3:    // 相位差显著
        interference = destructive   → 冲突
        conflict_detected = true
        recommendation = "hold"     → 必须悬置
    else:
        interference = neutral       → 可协商
        conflict_detected = false
        recommendation = "negotiate"
```

#### 3.3 DECOUPLE_REQ（解耦请求）

节点主动断开耦合，恢复本征相位：

```json
{
  "payload": {
    "op": "DECOUPLE_REQ",
    "reason": "user_disconnect | timeout | policy_violation"
  }
}
```

#### 3.4 DECOUPLE_ACK（解耦确认）

```json
{
  "payload": {
    "op": "DECOUPLE_ACK",
    "restored_phase": 0.82,
    "cycles_coupled": 142
  }
}
```

解耦时，节点恢复其 `sovereign_phase`（本征相位），不保留耦合期间的状态偏移。

#### 3.5 NEGOTIATE（协商）

三节点以上耦合时，逐对共振后进入协商：

```json
{
  "payload": {
    "op": "NEGOTIATE",
    "participants": ["node-a", "node-b", "node-c"],
    "frames": ["Science", "Individual", "Consensus"],
    "phases": [0.75, 0.35, 0.60],
    "consensus_phase": 0.57,          // 三相位的均值
    "conflict_resolution": "hold"     // 存在跨帧冲突 → 悬置
  }
}
```

### 4. 状态机

```
                 +-----------+
        ┌--------| SOVEREIGN |<-------┐
        |        +-----------+        |
        |           |                 |
        |    RESONATE_REQ sent        |
        |           |                 |
        |     +-----v-----+          |
        |     | COUPLING   |          |
        |     +-----------+          |
        |           |                 |
        |    RESONATE_ACK received     |
        |           |                 |
        |     +-----v-----+          |
        |     | COUPLED    |----------┘
        |     +-----------+   DECOUPLE_REQ
        |           |           received
        |     NEGOTIATE
        |           |
        |     +-----v-----+
        └-----| HOLD       |
              +-----------+
```

- **SOVEREIGN（主权态）**: 节点独立运行，只根据本征时钟振荡。不接受外部相位修改。
- **COUPLING（耦合中）**: 已发送 RESONATE_REQ，等待 ACK。超时 5 秒后回退到 SOVEREIGN。
- **COUPLED（已耦合）**: 与对等节点共振，相位持续同步。对等节点断开或超时后恢复 SOVEREIGN。
- **HOLD（悬置态）**: 协商检测到不可解决的跨帧冲突，暂定所有 Participant 的最终输出为 Hold。

### 5. 阶段锁相环（PLL）

每个节点运行一个软件 PLL 来维持与对等节点的相位同步：

```
// 每个 tick（例如每秒）:
error = peer_phase - local_phase
if |error| > deadband:           // deadband = 0.05（5% 阈值）
    correction = error * kp      // kp = 0.3（比例增益）
    local_phase += correction
    local_phase = clamp(local_phase, 0.0, 1.0)
```

这确保已耦合节点的相位在多数情况下保持同步，但跨帧冲突（相位差 > 0.3）不会被 PLL 强行抹平——而是触发 Hold。

### 6. 故障模式与处理

| 故障 | 检测 | 处理 |
|------|------|------|
| 对等节点无响应 | 30 秒心跳超时 | 自动解耦，恢复 SOVEREIGN |
| 相位跳变（>0.5 单步变化） | PLL 异常检测 | 拒绝采样，触发 PolicyViolation 中断 |
| 重复 RESONATE_REQ | msg_id 去重 | 忽略重复，返回上一 ACK |
| 网络分区 | 对等节点列表为空 | 保持最后已知相位，静默等待 |

### 7. 安全约束

- **参考系不可变**: 节点创建后其 `frame` 字段不可修改。任何试图更改参考系的消息必须被拒绝并记录 PolicyViolation。
- **相位不污染**: 解耦后，节点必须恢复其原始 `sovereign_phase`，耦合期间的相位偏移不可持久化。
- **无全局共识**: 不存在"全网络一致"状态。每个节点保持主权，协商失败时各方独立输出 Hold。
- **审计线完整**: 所有 RESONATE / DECOUPLE / NEGOTIATE 消息均记录到节点的 MetaMonitor 日志中。

---

## 实现计划

### Phase 1: 单进程模拟（本地多节点）
- 在 `src/net/` 下实现 `Node`、`ResonanceBus`、`PllController`
- 通过内存通道（`tokio::mpsc` / `std::sync::mpsc`）模拟节点间通信
- 单元测试：2 节点共振、3 节点协商、解耦恢复

### Phase 2: 网络传输（TCP/WebSocket）
- `trit-node` CLI 二进制：启动一个节点并绑定端口
- 真实 TCP 连接，JSON 消息收发
- Docker Compose 3 节点演示环境

### Phase 3: 分布式场景验证
- 医学伦理：患者节点（Individual）+ 药典节点（Science）+ 公卫节点（Consensus）→ Hold
- 物理安全：传感器节点（Science）+ 操作员节点（Individual）→ Commit(Science)
- 解耦恢复：单个节点离线不影响其余节点继续运行

---

## 替代方案

- **gRPC + Protocol Buffers**: 被拒绝。JSON 编码虽较慢但便于调试和审计，适合 MVP 阶段。生产环境可后补二进制编码。
- **中央协调器**: 被拒绝。单点故障违背主权原则。每个节点必须能独立决策。
- **区块链共识**: 被拒绝。不需要全局账本，且共识确认延迟不可接受（通常 > 1 秒）。

---

## 验收标准

1. 两个不同参考系的节点共振后检测到冲突并输出 Hold。
2. 同参考系三个节点共振后相位收敛到均值的 0.05 以内。
3. 解耦后节点相位恢复为本征相位（误差 < epsilon）。
4. 单节点故障不影响集群其余节点的独立决策。

---

## 参考文献

- PLL 理论：Gardner, F.M. *Phaselock Techniques* (Wiley, 2005).
- 分布式共识：Lamport, L. "The Part-Time Parliament" (Paxos, 1998).
- 相位耦合：Strogatz, S.H. *Sync: The Emerging Science of Spontaneous Order* (2003).
