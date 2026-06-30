# M3a: 璇玑离线地球 — 设计规格

**Date**: 2026-06-25
**Status**: design-approved
**Target**: Aurora M3a — Tauri v2 桌面应用中嵌入 CesiumJS 离线 3D 地球，内置微型瓦片服务器，零文字纯视听璇玑视觉层。

---

## 1. 目标

双击一个 exe，看到离线 3D 地球在旋转。零文字。零网络。零 UI。

这是"流沙"项目的**璇玑视觉层**——忠实模拟天道运转，不附会任何意义。用户透过它看到地球，至于看到后产生什么感受——敬畏、虚无、紧迫、释然——那是用户自己的事。

---

## 2. 架构总览

```
┌──────────────────────────────────────────────────────┐
│  Tauri v2 窗口 (单个 exe, 全屏无边框)                  │
│  ┌────────────────────────────────────────────────┐  │
│  │  WebView (WebView2, Windows 系统自带)            │  │
│  │  ┌──────────────────────────────────────────┐  │  │
│  │  │  CesiumJS 1.x (离线, 本地文件)             │  │  │
│  │  │  • 离线影像瓦片 ← tile_server:21337       │  │  │
│  │  │  • 离线地形数据 ← tile_server:21337       │  │  │
│  │  │  • 零文字 UI (璇玑: 纯粹旋转)              │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────┘  │
│                       │                              │
│  ┌────────────────────┴───────────────────────────┐  │
│  │  Tauri Rust 后端 (src-tauri/)                   │  │
│  │  • tile_server: tiny_http @ port 21337         │  │
│  │  • 窗口管理 (全屏, 无边框, 无菜单)               │  │
│  │  • 全局快捷键 (Esc = 退出, 无确认弹窗)           │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
         │
         ▼
D:/quicksand-data/tile/
├── img/{z}/{x}/{y}.jpg     ← 离线影像瓦片
└── terrain/                 ← 离线地形数据 (quantized-mesh)
```

**关键架构决策**：

| 决策 | 方案 | 理由 |
|------|------|------|
| 桌面框架 | Tauri v2 | 已有设计文档 `2026-06-24-tauri-desktop-shell.md`，复用 AuroraApp facade |
| WebView | WebView2 (Windows 自带) | 零额外依赖，系统内置 Edge 内核 |
| 瓦片服务器 | `tiny_http` (Rust) | 嵌入 Tauri 进程，无需外部 Nginx |
| 3D 引擎 | CesiumJS 1.x 离线包 | 最成熟的 3D GIS，支持离线部署 |
| 前端框架 | React 18 + TypeScript + Vite | 与现有 Tauri 桌面壳计划一致 |
| 瓦片数据 | 预下载到 `D:/quicksand-data/tile/` | 一次性手动准备，永久离线 |

---

## 3. 文件结构

```
trit-core/                        # workspace root (不变)
├── Cargo.toml                    # MODIFIED: 新增 src-tauri 成员
│
├── src-tauri/                    # NEW: Tauri v2 项目
│   ├── Cargo.toml                # tauri, tiny_http, serde, serde_json
│   ├── tauri.conf.json           # 窗口配置 (全屏, 无边框)
│   ├── build.rs                  # tauri-build
│   ├── src/
│   │   ├── main.rs               # Tauri 入口 + 启动 tile_server
│   │   ├── tile_server.rs        # 微型瓦片 HTTP 服务器
│   │   └── lib.rs                # Tauri 命令注册
│   └── icons/                    # 应用图标
│
├── ui/                           # NEW: React + TypeScript 前端
│   ├── package.json              # react, react-dom, cesium, vite
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html                # 最小 HTML 壳
│   └── src/
│       ├── main.tsx              # React 入口
│       ├── App.tsx               # 根组件 (全屏暗色容器)
│       └── Earth.tsx             # CesiumJS 封装组件
│
├── python/                       # NEW: M3b 数据采集脚本 (预留)
│   └── README.md
│
├── aurora/                       # 不变 (M3a 不碰 aurora 核心)
└── trit-core/                    # 不变
```

---

## 4. 核心组件设计

### 4.1 瓦片服务器 (tile_server.rs)

```rust
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::io::{Read, Write};
use std::fs;

/// 启动微型 HTTP 瓦片服务器。
///
/// 监听 127.0.0.1:21337，将 URL 路径映射到 data_dir 下的文件。
/// 路径安全检查：拒绝包含 ".." 的请求，防止目录遍历攻击。
pub fn start_tile_server(data_dir: PathBuf) {
    thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:21337").unwrap();
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            // 读取 HTTP 请求的第一行
            let mut buffer = [0; 1024];
            let n = stream.read(&mut buffer).unwrap_or(0);
            if n == 0 { continue; }
            let request = String::from_utf8_lossy(&buffer[..n]);

            // 解析 GET /path HTTP/1.1
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");

            // 安全: 拒绝路径遍历
            if path.contains("..") {
                let _ = stream.write(b"HTTP/1.1 403 Forbidden\r\n\r\n");
                continue;
            }

            let file_path = data_dir.join(&path[1..]); // strip leading /

            if file_path.exists() && file_path.is_file() {
                let content = fs::read(&file_path).unwrap_or_default();
                let mime = mime_for_path(&file_path);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n",
                    mime,
                    content.len()
                );
                let _ = stream.write(response.as_bytes());
                let _ = stream.write(&content);
            } else {
                let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n");
            }
        }
    });
}

fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("terrain") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}
```

### 4.2 Tauri 入口 (main.rs)

```rust
// 隐藏 Windows 控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tile_server;

fn main() {
    let data_dir = std::path::PathBuf::from("D:/quicksand-data/tile");

    // 如果瓦片目录不存在，创建它（首次运行）
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).ok();
    }

    // 启动内置瓦片服务器
    tile_server::start_tile_server(data_dir);

    // 启动 Tauri 应用
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 4.3 Tauri 窗口配置 (tauri.conf.json)

```json
{
  "productName": "流沙",
  "version": "0.1.0",
  "identifier": "com.quicksand.aurora",
  "build": {
    "frontendDist": "../ui/dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "cd ui && npm run dev",
    "beforeBuildCommand": "cd ui && npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "",
        "fullscreen": false,
        "width": 1920,
        "height": 1080,
        "decorations": false,
        "resizable": true,
        "center": true,
        "visible": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; connect-src http://127.0.0.1:21337; img-src 'self' http://127.0.0.1:21337; worker-src 'self' blob:;"
    }
  }
}
```

### 4.4 CesiumJS 封装 (Earth.tsx)

```typescript
import { useEffect, useRef } from 'react';
import * as Cesium from 'cesium';

export default function Earth() {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const viewer = new Cesium.Viewer(containerRef.current, {
      imageryProvider: false,       // 禁用默认在线影像 — 离线关键
      terrainProvider: new Cesium.CesiumTerrainProvider({
        url: 'http://127.0.0.1:21337/terrain/'
      }),
      baseLayerPicker: false,
      geocoder: false,
      homeButton: false,
      sceneModePicker: false,
      navigationHelpButton: false,
      animation: false,
      timeline: false,
      fullscreenButton: false,
      vrButton: false,
      infoBox: false,              // 流沙: 零文字
      selectionIndicator: false,
      creditContainer: undefined,  // 隐藏 Cesium 版权
    });

    // 璇玑: 加载离线瓦片
    viewer.imageryLayers.addImageryProvider(
      new Cesium.UrlTemplateImageryProvider({
        url: 'http://127.0.0.1:21337/img/{z}/{x}/{y}.jpg',
        maximumLevel: 8,
      })
    );

    // 璇玑: 缓慢自转
    viewer.clock.multiplier = 0.05;
    viewer.scene.globe.enableLighting = false;

    // 璇玑: 初始视角 (从远处看整个地球)
    viewer.camera.setView({
      destination: Cesium.Cartesian3.fromDegrees(0, 20, 20000000)
    });

    // 渲染完成后显示窗口
    viewer.scene.globe.tileLoadProgressEvent.addEventListener((remaining: number) => {
      if (remaining === 0) {
        // 通知 Tauri 显示窗口
        (window as any).__TAURI_INTERNALS__?.invoke('show_window');
      }
    });

    return () => {
      viewer.destroy();
    };
  }, []);

  return (
    <div
      ref={containerRef}
      style={{
        width: '100vw',
        height: '100vh',
        margin: 0,
        padding: 0,
        background: '#000',
      }}
    />
  );
}
```

### 4.5 根组件 (App.tsx)

```typescript
import Earth from './Earth';

export default function App() {
  return (
    <div style={{
      width: '100vw',
      height: '100vh',
      overflow: 'hidden',
      background: '#000',
      cursor: 'default',
      userSelect: 'none',
    }}>
      <Earth />
    </div>
  );
}
```

---

## 5. 启动时序

```
用户双击 aurora.exe
        │
        ▼
┌─────────────────────────────────────────────────┐
│ 1. Tauri 主进程启动                              │
│    ├── 创建 D:/quicksand-data/tile/ (如不存在)    │
│    ├── 启动 tile_server (127.0.0.1:21337)        │
│    └── 创建 Tauri 窗口 (visible: false)           │
│              │                                   │
│              ▼                                   │
│ 2. WebView 加载 ui/dist/index.html               │
│    ├── 加载 CesiumJS (本地文件, 零网络请求)        │
│    ├── 初始化 Viewer (imageryProvider: false)     │
│    ├── 加载离线瓦片 + 地形                        │
│    ├── 设置自转 + 初始视角                        │
│    └── 瓦片加载完成 → IPC show_window             │
│              │                                   │
│              ▼                                   │
│ 3. 窗口可见                                       │
│    黑暗背景 → 地球浮现 → 缓慢自转                  │
│              │                                   │
│              ▼                                   │
│ 4. 用户交互                                       │
│    ├── 滚轮: 缩放                                 │
│    ├── 拖拽: 旋转                                 │
│    ├── 右键: 平移                                 │
│    └── Esc: 退出 (无确认弹窗)                     │
└─────────────────────────────────────────────────┘
```

---

## 6. 测试策略

| 层 | 测试内容 | 工具 | 自动化 |
|----|----------|------|--------|
| 单元 | `tile_server` 正确解析路径 | `cargo test` | ✅ |
| 单元 | `tile_server` 对不存在的文件返回 404 | `cargo test` | ✅ |
| 单元 | `tile_server` 拒绝 `../` 路径遍历 | `cargo test` | ✅ |
| 单元 | `tile_server` 返回正确的 Content-Type | `cargo test` | ✅ |
| 伦理门 | 窗口无任何 DOM 文字元素 | Playwright | ✅ (M3b) |
| 伦理门 | CesiumJS 无 infoBox/credit/control | `document.querySelector` 断言 | ✅ |
| 集成 | 应用启动不崩溃 | `cargo build --release` | ✅ |
| 集成 | CesiumJS 离线加载瓦片成功 | 手动验证 | ❌ |
| 集成 | 全屏、Esc 退出 | 手动验证 | ❌ |

---

## 7. 依赖清单

| 依赖 | 版本 | 用途 |
|------|------|------|
| `tauri` | 2.x | 桌面壳框架 |
| `tauri-build` | 2.x | 构建脚本 |
| `serde` / `serde_json` | 1.x | 配置序列化 |
| `react` | 18.x | UI 框架 |
| `react-dom` | 18.x | DOM 渲染 |
| `cesium` | 1.x (离线静态文件) | 3D 地球引擎 |
| `vite` | 5.x | 前端构建 |
| `typescript` | 5.x | 类型安全 |
| `@types/cesium` | 1.x | Cesium TypeScript 类型 |

**不需要的**：
- ❌ Nginx（被内置 `tiny_http` 替代）
- ❌ `tiny_http` crate（使用 `std::net::TcpListener` stdlib 方案，零额外依赖）
- ❌ Python 运行时（M3a 不需要科学数据采集）
- ❌ Node.js 服务端（Vite 仅构建时需要，运行时不需要）

---

## 8. 零改动区域

- `trit-core/` — 零改动
- `aurora/src/bc/` — 零改动
- `aurora/src/db/` — 零改动
- `aurora/src/percept/` — 零改动
- `aurora/src/pipeline/` — 零改动
- `aurora/src/config/` — 零改动
- `aurora/src/app.rs` — 零改动

---

## 9. 流沙哲学合规清单

| 原则 | 实现 | 验证 |
|------|------|------|
| **璇玑** — 忠实旋转 | CesiumJS clock.multiplier = 0.05 缓慢自转 | 视觉验证 |
| **零文字** | infoBox: false, creditContainer: undefined, 无 DOM 文字 | `document.body.innerText === ""` |
| **不引导** | 无 UI 控件, 无高亮, 无热点 | 视觉验证 |
| **微风** | Esc 退出无确认弹窗 | 视觉验证 |
| **不解释** | 无图例, 无标签, 无提示 | 视觉验证 |

---

## 10. M3b 预留扩展点

M3a 完成后，M3b 在此基础上叠加科学数据图层：

```
M3b 新增:
├── python/               # 数据采集脚本
│   ├── fetch_era5.py
│   ├── fetch_nasa_power.py
│   └── fetch_usgs_eq.py
├── aurora/src/ingest/sci/  # Rust 科学数据管道
│   ├── mod.rs
│   ├── pipeline.rs       # SciDataPipeline
│   ├── store.rs          # SciDataStore (SQLite)
│   ├── sources/
│   │   ├── era5.rs
│   │   ├── nasa_power.rs
│   │   ├── usgs_eq.rs
│   │   └── ...
│   └── trait.rs          # SciDataSource trait
├── ui/src/layers/        # 前端数据图层
│   ├── ClimateLayer.tsx
│   ├── QuakeLayer.tsx
│   └── EcoLayer.tsx
└── src-tauri/src/
    └── commands.rs       # Tauri IPC: 查询科学数据
```

M3b 的 SciDataSource 实现现有 `ExternalPercept` trait — 科学数据源的 `perceive()` 方法从 NetCDF/GeoJSON 提取 TritWord 信号，`raw_data_layer` 字段承载物理测量值。
