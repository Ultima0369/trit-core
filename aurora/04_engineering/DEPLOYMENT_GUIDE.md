# Aurora 部署指南

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 04_engineering — 工程规格

---

## 一、本地部署（个人版）

### 1.1 系统要求

| 平台 | 最低配置 | 推荐配置 |
|------|----------|----------|
| macOS | 10.15+，4GB RAM | 12+，8GB RAM |
| Windows | 10，4GB RAM | 11，8GB RAM |
| Linux | Ubuntu 20.04+，4GB RAM | 22.04+，8GB RAM |

### 1.2 安装步骤

```bash
# 下载安装包
wget https://aurora.example.com/download/aurora-desktop-0.1.0-mac.dmg

# 安装
open aurora-desktop-0.1.0-mac.dmg
# 拖拽到 Applications

# 首次启动
# 1. 选择数据目录
# 2. 设置加密密码
# 3. 配置数据源（邮件、日历等）
# 4. 完成
```

### 1.3 目录结构

```
~/.aurora/
├── config.toml           # 配置文件
├── aurora.db             # SQLite 数据库（加密）
├── aurora.db.backup      # 自动备份
├── logs/                 # 日志目录
├── exports/              # 导出数据目录
└── cache/                # 缓存目录
```

### 1.4 升级

```bash
# 自动升级（默认）
# Aurora 会检查更新，提示用户升级

# 手动升级
# 1. 下载新版本
# 2. 关闭 Aurora
# 3. 替换应用程序
# 4. 启动（自动迁移数据库）
```

---

## 二、团队部署（团队版）

### 2.1 系统要求

| 组件 | 最低配置 | 推荐配置 |
|------|----------|----------|
| 服务器 | 4核 CPU，8GB RAM | 8核，16GB RAM |
| 存储 | 100GB SSD | 500GB SSD |
| 网络 | 内网 | 内网 + VPN |

### 2.2 安装步骤

```bash
# 下载服务器版
docker pull aurora/aurora-server:0.1.0

# 运行
docker run -d \
  -v /data/aurora:/data \
  -p 8080:8080 \
  -e AURORA_ENV=production \
  aurora/aurora-server:0.1.0

# 配置
# 访问 http://localhost:8080/admin
# 设置管理员密码
# 配置 LDAP/SSO
# 添加用户
```

### 2.3 客户端配置

```bash
# 桌面客户端连接到团队服务器
# 设置 → 团队 → 输入服务器地址和凭据
```

---

## 三、企业部署（企业版）

### 3.1 系统要求

| 组件 | 最低配置 | 推荐配置 |
|------|----------|----------|
| 服务器集群 | 3节点，8核/16GB | 5节点，16核/32GB |
| 数据库 | PostgreSQL 13+ | PostgreSQL 15+ |
| 负载均衡 | Nginx | Nginx + HAProxy |
| 监控 | Prometheus + Grafana | 完整监控栈 |

### 3.2 高可用部署

```yaml
# docker-compose.yml
version: '3.8'
services:
  aurora-server:
    image: aurora/aurora-server:0.1.0
    replicas: 3
    environment:
      - AURORA_ENV=production
      - DATABASE_URL=postgresql://...
    volumes:
      - /data/aurora:/data
  
  postgres:
    image: postgres:15
    environment:
      - POSTGRES_PASSWORD=...
    volumes:
      - /data/postgres:/var/lib/postgresql/data
  
  nginx:
    image: nginx:latest
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
```

---

## 四、备份与恢复

### 4.1 备份策略

| 类型 | 频率 | 保留期 | 方式 |
|------|------|--------|------|
| 自动备份 | 每次启动 | 10 个 | 本地 `.backup` |
| 手动导出 | 用户触发 | 永久 | 加密 JSON/SQLite |
| 团队备份 | 每日 | 30 天 | 服务器快照 |

### 4.2 恢复步骤

```bash
# 本地恢复
# 1. 关闭 Aurora
# 2. 替换 ~/.aurora/aurora.db 为备份文件
# 3. 启动 Aurora

# 团队恢复
# 1. 停止 Aurora Server
# 2. 从备份恢复 PostgreSQL
# 3. 启动 Aurora Server
```

---

## 五、故障排除

| 问题 | 原因 | 解决 |
|------|------|------|
| 启动失败 | 数据库损坏 | 恢复备份 |
| 分析缓慢 | 数据量过大 | 减少保留期或升级硬件 |
| 内存不足 | 内存泄漏 | 重启应用，报告 bug |
| 数据源不工作 | 权限变更 | 重新授权 |
| 同步失败 | 网络问题 | 检查网络，重试 |

---

*本文档为 Aurora 的部署指南。完整部署脚本见 GitHub 仓库。不是指教，是提醒。*
