# N-of-1 实践协议：把第一人称经验变成可检验数据

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 草案
**分类**: 03_methodology — 实践方法
**来源**: 基于 dao-science/3_methodology/n_of_1_protocol.md，适配 trit-core 框架

---

## 一、核心命题

> **N-of-1 不是轶事。是系统化、可重复的因果推断设计。**
>
> 被试数量 N=1——研究者自己。通过在不同条件之间多次切换，个体可以判断某种干预对自己是否有效。

---

## 二、为什么 trit-core 需要 N-of-1

**当前 trit-core 的检验层只有第三方测试**（单元测试、集成测试、属性测试）。但 NRP（神经回路重塑协议）和 Aurora 的认知训练模块需要**第一人称检验**——用户需要知道训练对自己是否有效，而不是"对平均人是否有效"。

| 检验方式 | 问什么 | 适用场景 | 在 trit-core 中的位置 |
|---------|--------|---------|-------------------|
| 大样本 RCT | 对平均人有效吗？ | 药物审批、公共卫生 | 不适用（个体差异太大） |
| 第三方测试 | 代码正确吗？ | 软件工程 | TESTING_STRATEGY.md |
| **N-of-1** | **对我有效吗？** | **认知训练、注意力训练** | **本协议** |

---

## 三、最小可行 N-of-1 协议（Q-H-B-I-M-A-D）

### 3.1 七步框架

1. **Q（Question）**：我想回答什么问题？
2. **H（Hypothesis）**：我的可证伪预测是什么？
3. **B（Baseline）**：基线期测量多久？
4. **I（Intervention）**：具体做什么？剂量、频率、时长？
5. **M（Measurement）**：用什么指标？何时测量？
6. **A（Analysis）**：如何判断有效？
7. **D（Decision）**：结果出来后做什么？

### 3.2 设计类型

| 类型 | 结构 | 优点 | 缺点 | 适用场景 |
|------|------|------|------|----------|
| A-B | 基线 → 干预 | 易执行 | 无法排除历史效应 | 快速初步测试 |
| A-B-A-B | 基线 → 干预 → 撤除 → 干预 | 因果推断强 | 需要更长时间 | 确认效果稳定 |
| 多基线 | 多个行为/情境同时基线，错开引入 | 控制个体变异 | 设计复杂 | 多症状同时干预 |

---

## 四、针对三法一门的 N-of-1 设计示例

### 示例：L1 回光觉知（眉心法）

**Q**：每天 10 分钟眉心聚焦训练，能否降低我的"念头频率"？

**H**：与基线期相比，干预期中每日"念头频率"（自我估计）下降至少 30%。

**B**：1 周，每天早晨记录：
- 念头频率（每分钟估计）
- HRV（RMSSD，如有设备）
- 主观焦虑感（0-10）

**I**：每天早晨起床后，坐姿，闭眼，注意力放在眉心深处，持续 10 分钟。操作："感受眉心区域的压力/温度/酥麻，不思考、不分析、不追求效果。"

**M**：
- 念头频率：训练后立刻记录（主观估计，每分钟多少个念头飘过）
- HRV：训练前后各测 1 分钟（如有智能手表）
- 焦虑感：训练后记录（0-10）
- 日期、时间、睡眠时长、咖啡因摄入（控制变量）

**A**：比较基线期（7 天）与干预期（7 天）的平均念头频率。下降 ≥ 30% 视为有效。

**D**：
- 若有效：继续，尝试减少训练时长（找到最小有效剂量）
- 若无效：检查操作是否准确（注意力是否真在眉心？），或尝试听息法/耳根法
- 若恶化：停止，记录可能原因（如操作过度用力导致紧张）

### 示例：L2 天心定位（α 切换训练）

**Q**："知止"训练（每 90 分钟主动切换焦点→全局）能否提升我的注意力灵活性？

**H**：干预期中，注意力网络测试（ANT）的反应时变化率改善 ≥ 20%。

**B**：1 周，每天 ANT 测试 1 次（在线免费工具）。

**I**：深度工作 90 分钟后，强制停止，进行 15 分钟"知止"——散步、静坐或做手工，不思考工作。

**M**：
- ANT 反应时（每天固定时间测试）
- 主观切换速度（从"聚焦"到"放松"需要几分钟？）
- 每日工作产出（作为副作用指标）

**A**：ANT 反应时变化率 = （干预后 - 干预前）/ 干预前。改善 ≥ 20% 视为有效。

**D**：
- 若有效：固定为工作节奏（90+15）
- 若无效：调整知止时长（30 分钟？）或知止内容（静坐 vs 散步）
- 若工作产出下降：缩短知止时长，或只在下午使用

---

## 五、数据记录模板（SQLite 格式）

```sql
-- N-of-1 实验记录表
CREATE TABLE n_of_1_experiments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experiment_name TEXT NOT NULL,
    question TEXT NOT NULL,
    hypothesis TEXT NOT NULL,
    design_type TEXT NOT NULL,  -- A-B / A-B-A-B / 多基线
    start_date TEXT NOT NULL,
    end_date TEXT,
    status TEXT NOT NULL DEFAULT 'running'  -- running / completed / stopped
);

-- 每日记录表
CREATE TABLE n_of_1_daily_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experiment_id INTEGER NOT NULL REFERENCES n_of_1_experiments(id),
    date TEXT NOT NULL,
    phase TEXT NOT NULL,  -- A（基线）或 B（干预）
    
    -- 操作记录
    intervention_done INTEGER NOT NULL DEFAULT 0,  -- 0/1
    intervention_duration_min INTEGER,
    intervention_notes TEXT,
    
    -- 指标（用户自定义，JSON 格式）
    metrics TEXT NOT NULL,  -- JSON: {"念头频率": 15, "HRV_RMSSD": 45, "焦虑": 3}
    
    -- 控制变量
    sleep_hours REAL,
    caffeine_mg INTEGER,
    stress_events TEXT,  -- 当天重大事件
    
    -- 主观备注
    notes TEXT
);

-- 结果分析表（实验结束后填写）
CREATE TABLE n_of_1_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experiment_id INTEGER NOT NULL REFERENCES n_of_1_experiments(id),
    baseline_mean TEXT NOT NULL,  -- JSON
    intervention_mean TEXT NOT NULL,  -- JSON
    effect_size TEXT,  -- JSON: {"念头频率": {"change_pct": -35, "significant": true}}
    decision TEXT NOT NULL,  -- continue / adjust / stop
    decision_reason TEXT NOT NULL
);
```

---

## 六、关键原则

### 6.1 诚实不自欺

N-of-1 的有效性完全取决于**数据质量**。常见自欺行为：
- **选择性记录**：只记好的天数，跳过坏的天数 → 数据失真
- **事后归因**："那天状态不好是因为没睡好"——可以备注，但不能删除数据
- **操作变异**：今天练 10 分钟，明天练 5 分钟，后天忘了 → 无法判断剂量效应

**对策**：
- 固定测量时间（如每天起床后、睡前）
- 使用设备自动记录（HRV、睡眠）减少主观偏差
- 即使干预没做，也要记录"0"，不要跳过

### 6.2 最小有效剂量

训练不是越多越好。N-of-1 的目标之一是找到**最小有效剂量**——花最少时间获得效果。

- 从最低剂量开始（如每天 5 分钟）
- 有效后尝试减少 50%，看是否仍有效
- 无效后尝试增加 50%，看是否开始有效
- 找到"刚好有效"的剂量，固定执行

### 6.3 与 CHARTER.md 的对齐

- **不剥夺**：用户随时可停止实验，无后果
- **不自欺**：数据必须如实记录，系统不猜测、不补全
- **不进化**：实验协议固定，不基于用户数据自动调整
- **公开可审查**：用户可随时导出全部数据（JSON/SQLite）

---

## 七、重要声明

> **N-of-1 是科学研究方法，不是医学诊断。**
>
> 如果涉及心理健康问题（如抑郁、焦虑障碍、创伤），N-of-1 可以作为**辅助工具**，但不能替代专业医疗。任何训练导致症状恶化时，应立即停止并寻求专业帮助。

---

*本文档基于 dao-science 项目的 N-of-1 协议，适配 trit-core 框架。Q-H-B-I-M-A-D 框架为科学方法论的标准结果。三法一门的 N-of-1 设计为启发性示例，需根据个体实情调整。不是指教，是提醒。*
