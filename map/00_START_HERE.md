# Trit-Core 双螺旋知识库

> **这是一个 Obsidian 风格的知识库。** 用 Obsidian 打开本目录，所有 `[[文件名]]` 链接将激活双向导航和图谱视图。
>
> 如果你在用普通 Markdown 阅读器，这些链接显示为文本，但路径信息仍然可追踪。

---

## 什么是双螺旋

本项目由两条链交织而成：

- **链 A（软件实现）**：`src/` → 可编译、可测试、可运行的 Rust 代码
- **链 B（知识文档）**：`docs/` + `aurora/` → 可理解、可讨论、可传承的概念与决策

**关键洞见**：两者不是主从关系。代码是文档的物化检验，文档是代码的意图锚定。任何修改都应该能追踪到另一条链的对应位置。

---

## 快速入口

| 你想了解... | 进入这条 MOC |
|---|---|
| **今天加入项目，第一步该做什么** | [aurora/MASTER_PLAN.md](../aurora/MASTER_PLAN.md) — 唯一执行入口 |
| 项目为什么存在、底线是什么 | [[01_manifest]] |
| TritValue、Frame、Phase、Hold 到底是什么 | [[02_concepts]] |
| 关键架构决策及其理由 | [[03_adr]] |
| 数学模型、信息论、场方程 | [[04_math]] |
| 工程实现、测试策略、部署 | [[05_engineering]] |
| 代码与文档的对应关系 | [[06_code]] |
| 哲学洞察、认知科学、人文索引 | [[07_insights]] |
| 按标签浏览全部内容 | [[99_tag_index]] |

> **新协作者必读顺序**：MASTER_PLAN → 本文件（理解双螺旋） → 对应阶段的 MOC → 具体文档。不要一上来就读 60 份文件。

---

## 两条链的物理位置

### 链 A：代码

```
src/
├── core/          # 三值代数、帧系统、相位运算
├── meta/          # 元监控、仲裁、安全回退
├── sandbox/       # 沙盒、场景管道、验证器
├── bin/           # CLI 入口（trit-sandbox）
└── lib.rs         # 公共 API
```

### 链 B：文档

```
docs/              # Trit-Core crate 技术文档（英/中混合）
├── adr/           # 4 个英文架构决策记录
├── explanation/   # 架构、概念、哲学、洞察
├── how-to/        # CLI 参考、配置、贡献指南
├── reference/     # API 契约、模块、基准测试
├── reports/       # 验证与审计报告
└── tutorials/     # 快速上手、入门故事

aurora/            # Aurora 应用文档（全中文）
├── 00_manifest/   # 宪章、原则、认知架构
├── 01_insights/   # 认知科学洞察
├── 02_math/       # 数学模型
├── 03_whitepaper/ # 技术白皮书、API 契约、安全模型
├── 04_engineering/# 工程实现、数据模型、部署
├── 05_adr/        # 9 个架构决策记录
├── 06_roadmap/    # 里程碑与出口标准
├── 07_specs/      # 详细规格（小波引擎、告警等）
└── 08_reports/    # 审计、评估、报告模板
```

---

## 使用这个知识库

### 在 Obsidian 中

1. 打开 `C:\Users\Ultima\Documents\kimi\workspace\trit-core` 作为 Vault
2. 安装 **Graph View** 插件（默认已启用）
3. 从 `map/00_START_HERE.md` 开始浏览
4. 点击 `[[链接]]` 在文件间跳转
5. 在 Graph View 中查看概念之间的关联网络

### 在 GitHub / 普通 Markdown 中

1. `[[链接]]` 显示为纯文本，但文件名明确
2. 使用 `map/` 目录的 MOC 文件作为手动导航
3. 搜索关键词定位相关文件

---

## 维护约定

- **新增概念文件** → 在对应 MOC 中添加 `[[链接]]`
- **新增 ADR** → 同时在 `map/03_adr.md` 的 **docs** 和 **aurora** 两节中登记
- **修改代码** → 检查 `map/06_code.md` 中对应条目，确认文档是否同步
- **跨链引用** → 优先用 `[[文件名]]` 建立显式链接，而非依赖目录邻近

---

**版本**: 0.3.0  
**最后更新**: 2026-06-20

#trit-core #aurora #knowledge-base #obsidian #map-of-content
