# 小波引擎模块规格

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 07_specs — 详细模块规格

---

## 一、模块职责

对离散时间序列进行小波变换，提取认知特征（基频、谐波、相位漂移、频谱重构、跨信号同步）。

## 二、接口定义

```rust
pub struct WaveletEngine {
    pub mother_wavelet: MotherWavelet,
    pub scales: Vec<f64>,
    pub sampling_rate: f64,
}

impl WaveletEngine {
    pub fn analyze(&self, signal: &[f64]) -> Result<WaveletResult, WaveletError>;
    pub fn extract_features(&self, result: &WaveletResult) -> Result<Vec<WaveletFeature>, WaveletError>;
    pub fn get_scalogram(&self, result: &WaveletResult) -> Scalogram;
}

pub struct WaveletResult {
    pub coefficients: Vec<Vec<Complex<f64>>>,
    pub scales: Vec<f64>,
    pub times: Vec<f64>,
}

pub struct WaveletFeature {
    pub feature_type: FeatureType,
    pub value: f64,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
}

pub enum FeatureType {
    FundamentalFreq,
    Harmonic { order: u8 },
    PhaseDrift,
    SpectralReconfig,
    CrossSync { signal_pair: (String, String) },
}
```

## 三、算法参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| 母小波 | Morlet | 平衡时间-频率分辨率 |
| 中心频率 ω₀ | 6.0 | 标准 Morlet 参数 |
| 尺度范围 | 2-128 | 覆盖分钟到日级周期 |
| 尺度步进 | 对数均匀 | 每八度 16 个尺度 |
| 采样率 | 1/小时 | 通信频率等信号的默认采样率 |

## 四、性能要求

| 指标 | 目标 | 验证 |
|------|------|------|
| 日数据分析 | < 1秒 | Criterion |
| 周数据分析 | < 5秒 | Criterion |
| 内存 | < 200MB | dhat |
| 精度 | 基频误差 < 1% | 合成数据验证 |

## 五、降级策略

| 场景 | 降级 |
|------|------|
| 数据量过大 | 减少尺度数量 |
| 内存不足 | 使用 DWT 替代 CWT |
| 精度要求低 | 减少采样率 |
| 实时模式 | 滑动窗口，只分析最新数据 |

---

*不是指教，是提醒。*
