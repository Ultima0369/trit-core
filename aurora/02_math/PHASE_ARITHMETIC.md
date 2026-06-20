# Phase 算术：形式化定义

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 02_math — 数学支持

---

## 一、Phase 的类型定义

### 1.1 Trit-Core 的 Phase

```rust
pub struct Phase(f64);  // 范围 [0.0, 1.0]
```

- `0.0`：完全倾向于 False
- `0.5`：完全中性
- `1.0`：完全倾向于 True
- `0.73`：倾向于 True，但保留 27% 的不确定性空间

### 1.2 语义解释

`Phase` 不是概率，不是置信度，不是模糊隶属度。

`Phase` 是**连续倾向度**——在 True 和 False 之间的光谱位置，但保留"这个倾向本身可能是被环境塑造的"这一元信息。

---

## 二、Phase 运算

### 2.1 构造

```rust
impl Phase {
    // 安全构造：验证范围，拒绝非法值
    pub fn new(v: f64) -> Result<Phase, PhaseError> {
        if v.is_nan() || v.is_infinite() {
            Err(PhaseError::InvalidValue(v))
        } else if !(0.0..=1.0).contains(&v) {
            Err(PhaseError::OutOfRange(v))
        } else {
            Ok(Phase(v))
        }
    }
    
    // 静默构造：非法值归一化到 [0,1]，但记录警告
    pub fn new_clamped(v: f64) -> Phase {
        Phase(v.clamp(0.0, 1.0))
    }
    
    // 中性 Phase
    pub fn neutral() -> Phase {
        Phase(0.5)
    }
}
```

### 2.2 均值运算（同 Frame 信号融合）

同 Frame 的多个信号融合时，Phase 取均值：

$$\text{mean}(p_1, p_2, ..., p_n) = \frac{1}{n} \sum_{i=1}^{n} p_i$$

**量化（Quantization）**：

$$\text{quantize}(p, \epsilon) = \begin{cases}
0.5 & \text{if } |p - 0.5| < \epsilon \quad \text{(中性锚点优先检查)} \\
0.0 & \text{if } |p - 0.0| < \epsilon \\
1.0 & \text{if } |p - 1.0| < \epsilon \\
p & \text{otherwise}
\end{cases}$$

**为什么中性锚点优先**：在大量级联中，0.50000001 和 0.49999999 在语义上应该被视为相同。

### 2.3 互补运算

$$\text{complement}(p) = 1.0 - p$$

**语义**：
- `complement(0.7)` = `0.3`（倾向 True 的互补是倾向 False）
- `complement(0.5)` = `0.5`（中性的互补还是中性）

### 2.4 承诺方向（Commitment Direction）

```rust
pub enum Commitment {
    TowardTrue,   // p > 0.5 + ε
    TowardFalse,  // p < 0.5 - ε
    Neutral,      // |p - 0.5| ≤ ε
}
```

**语义**：不是"True/False"，而是"倾向于哪个方向"——这个倾向本身可能是被环境塑造的。

---

## 三、Phase 与小波特征的映射

### 3.1 基频稳定性 → Phase

$$\Phi_{\text{stability}} = 1 - \frac{\sigma_f}{f_0}$$

其中 $\sigma_f$ 是基频的标准差，$f_0$ 是基频均值。

- 基频稳定 → $\sigma_f$ 小 → $\Phi$ 接近 1.0（True/稳定）
- 基频漂移 → $\sigma_f$ 大 → $\Phi$ 接近 0.0（False/不稳定）

### 3.2 谐波完整性 → Phase

$$\Phi_{\text{harmonic}} = \frac{\text{检测到的谐波数量}}{\text{预期的谐波数量}}$$

- 谐波完整 → $\Phi$ 接近 1.0
- 谐波缺失 → $\Phi$ 接近 0.0

### 3.3 相位漂移速度 → Phase

$$\Phi_{\text{drift}} = 1 - \min\left(\frac{|\Delta\phi / \Delta t|}{v_{\text{max}}}, 1.0\right)$$

其中 $v_{\text{max}}$ 是最大可接受的漂移速度。

- 漂移慢 → $\Phi$ 接近 1.0（稳定）
- 漂移快 → $\Phi$ 接近 0.0（不稳定）

### 3.4 频谱重构强度 → Phase

$$\Phi_{\text{reconfig}} = 1 - \min\left(\frac{D_{KL}}{D_{\text{max}}}, 1.0\right)$$

其中 $D_{KL}$ 是 KL 散度，$D_{\text{max}}$ 是最大可接受的散度。

- 频谱稳定 → $\Phi$ 接近 1.0
- 频谱重构 → $\Phi$ 接近 0.0

### 3.5 跨信号同步 → Phase

$$\Phi_{\text{sync}} = S_{xy} = e^{-\sigma_{\Delta\phi}^2}$$

- 完全同步 → $\Phi$ = 1.0
- 完全异步 → $\Phi$ = 0.0

---

## 四、Phase 与 TritValue 的转换

### 4.1 从 Phase 到 TritValue

```rust
impl From<Phase> for TritValue {
    fn from(phase: Phase) -> Self {
        match phase.commitment() {
            Commitment::TowardTrue => TritValue::True,
            Commitment::TowardFalse => TritValue::False,
            Commitment::Neutral => TritValue::Hold,
        }
    }
}
```

**注意**：这不是自动转换。Phase 到 TritValue 的转换只在特定 Domain 和特定仲裁规则下发生。

### 4.2 从 TritValue 到 Phase

```rust
impl From<TritValue> for Phase {
    fn from(value: TritValue) -> Self {
        match value {
            TritValue::True => Phase(1.0),
            TritValue::Hold => Phase(0.5),
            TritValue::False => Phase(0.0),
            TritValue::Unknown => Phase(0.5), // Unknown 映射到中性，但标记为不可计算
        }
    }
}
```

---

## 五、Phase 在组织涡旋中的映射

### 5.1 信息密度 → Phase

$$\Phi_{\text{density}} = \frac{\rho(r, t)}{\rho_{\text{max}}} = \left(\frac{r_0}{r}\right)^\alpha \cdot e^{-\beta t}$$

- 中心（$r$ 小）：信息密度高，$\Phi$ 接近 1.0
- 外围（$r$ 大）：信息密度低，$\Phi$ 接近 0.0

### 5.2 角速度 → Phase

$$\Phi_{\text{velocity}} = \frac{\omega(r)}{\omega_{\text{max}}} = \begin{cases}
1.0 & r \leq r_c \\
\left(\frac{r_c}{r}\right)^2 & r > r_c
\end{cases}$$

- 核心：角速度高，思维同步，$\Phi$ 接近 1.0
- 外围：角速度低，各自为战，$\Phi$ 接近 0.0

### 5.3 涡旋强度 → Phase

$$\Phi_{\text{vorticity}} = \frac{\zeta}{2\omega_0} = \begin{cases}
1.0 & r \leq r_c \\
0.0 & r > r_c
\end{cases}$$

- 核心：涡旋强度恒定，信息保真度高，$\Phi$ = 1.0
- 外围：涡旋强度为零，信息扩散但无旋，$\Phi$ = 0.0

---

## 六、Phase 的边界与异常

### 6.1 非法 Phase 值

| 值 | 类型 | 处理 |
|----|------|------|
| NaN | 非法 | `Phase::new` 返回 `Err` |
| +Inf | 非法 | `Phase::new` 返回 `Err` |
| -Inf | 非法 | `Phase::new` 返回 `Err` |
| -0.1 | 越界 | `Phase::new` 返回 `Err` |
| 1.1 | 越界 | `Phase::new` 返回 `Err` |
| 0.5 ± ε | 中性锚点 | `quantize` 归一化到 0.5 |

### 6.2 浮点漂移

长链级联中的浮点误差累积：
- 100 次 `mean` 运算后，最大误差约 100 × ε_machine ≈ 2.2e-14
- 对 `Phase` 的语义影响：可忽略（ε = 1e-6 足够）
- 但量化是必要的：防止 0.50000001 被错误地视为 TowardTrue

---

*本文档为 Phase 算术的形式化定义。所有运算在 Trit-Core 中已实现。Aurora 在此基础上扩展了与小波特征和组织涡旋的映射。不是指教，是提醒。*
