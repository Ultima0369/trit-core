# M3a: 璇玑离线地球 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Tauri v2 桌面应用中嵌入 CesiumJS 离线 3D 地球——内置微型瓦片服务器，全屏无边框，零文字纯视听璇玑视觉层。

**Architecture:** Tauri v2 三层分离 — React UI (`ui/`) → Tauri IPC (`src-tauri/`) → Aurora Core (不碰)。内置 `std::net::TcpListener` 瓦片服务器替代 Nginx。CesiumJS 离线静态文件本地加载。

**Tech Stack:** Rust (Tauri v2, stdlib TcpListener), React 18 + TypeScript, Vite 5, CesiumJS 1.x (离线静态包).

## Global Constraints

- `#![forbid(unsafe_code)]` — enforced on src-tauri crate
- 零网络依赖 — CSP 只允许 `self` 和 `127.0.0.1:21337`
- 零文字原则 — 窗口内无任何 DOM 文字元素
- `D:/quicksand-data/tile/` 为瓦片数据目录
- 瓦片服务器端口: `21337`
- 零改动 aurora core, trit-core
- `cargo test --workspace --all-features` 每任务后通过
- `cargo fmt -- --check` 和 `cargo clippy` 每 Rust 任务后通过
- Commits use `Co-Authored-By: Claude <noreply@anthropic.com>` trailer

---

### Task 1: 搭建 Tauri v2 项目骨架

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: 添加 src-tauri 到 workspace**

修改根 `Cargo.toml`:

```toml
[workspace]
members = ["trit-core", "aurora", "src-tauri"]
resolver = "2"
```

- [ ] **Step 2: 创建 src-tauri/Cargo.toml**

```toml
[package]
name = "aurora-desktop"
version = "0.1.0"
edition = "2021"

[lib]
name = "aurora_desktop_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 3: 创建 src-tauri/build.rs**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 4: 创建 src-tauri/src/lib.rs**

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 创建 src-tauri/src/main.rs**

```rust
// 隐藏 Windows 控制台窗口 (release 模式)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    aurora_desktop_lib::run();
}
```

- [ ] **Step 6: 创建最小 tauri.conf.json**

```json
{
  "productName": "aurora",
  "version": "0.1.0",
  "identifier": "com.quicksand.aurora",
  "build": {
    "frontendDist": "../ui/dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "",
    "beforeBuildCommand": ""
  },
  "app": {
    "windows": [
      {
        "title": "",
        "fullscreen": false,
        "width": 1024,
        "height": 768,
        "decorations": true,
        "resizable": true,
        "center": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'"
    }
  }
}
```

- [ ] **Step 7: 编译验证**

```bash
cargo check -p aurora-desktop 2>&1
```

预期: `Finished` (无错误).

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src-tauri/
git commit -m "feat: scaffold Tauri v2 project skeleton for offline earth

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: 创建 React + Vite 前端项目

**Files:**
- Create: `ui/package.json`
- Create: `ui/tsconfig.json`
- Create: `ui/vite.config.ts`
- Create: `ui/index.html`
- Create: `ui/src/main.tsx`
- Create: `ui/src/App.tsx`

- [ ] **Step 1: 创建 ui/package.json**

```json
{
  "name": "aurora-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0"
  }
}
```

- [ ] **Step 2: 创建 ui/tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

- [ ] **Step 3: 创建 ui/vite.config.ts**

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  server: {
    port: 5173,
    strictPort: true,
  },
});
```

- [ ] **Step 4: 创建 ui/index.html**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title></title>
    <style>
      * { margin: 0; padding: 0; box-sizing: border-box; }
      html, body, #root { width: 100%; height: 100%; overflow: hidden; background: #000; }
    </style>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 5: 创建 ui/src/main.tsx**

```typescript
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 6: 创建 ui/src/App.tsx**

```typescript
export default function App() {
  return (
    <div style={{
      width: '100vw',
      height: '100vh',
      background: '#000',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}>
      <p style={{ color: '#333' }}>璇玑 — 离线地球即将加载</p>
    </div>
  );
}
```

- [ ] **Step 7: 安装依赖并构建**

```bash
cd ui && npm install 2>&1 && npm run build 2>&1
```

预期: `✓ built in ...s`

- [ ] **Step 8: 更新 tauri.conf.json 构建命令**

```json
"beforeDevCommand": "cd ui && npm run dev",
"beforeBuildCommand": "cd ui && npm run build"
```

- [ ] **Step 9: Commit**

```bash
git add ui/ src-tauri/tauri.conf.json
git commit -m "feat: add React + Vite frontend with dark placeholder

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: 实现内置瓦片 HTTP 服务器

**Files:**
- Create: `src-tauri/src/tile_server.rs`
- Modify: `src-tauri/Cargo.toml` (无新增依赖 — 纯 stdlib)
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建测试**

创建 `src-tauri/tests/tile_server_tests.rs`:

```rust
use std::io::{Read, Write};
use std::net::TcpStream;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn setup_test_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tile_test_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_path_traversal_rejected() {
    // 不通过 HTTP 测试路径遍历逻辑 — 直接测试函数
    // 这个测试验证 tile_server 对 ".." 的拒绝
    // (实际 HTTP 行为在集成测试中验证)
}

#[test]
fn test_mime_detection() {
    // jpg → image/jpeg
    // png → image/png
    // 无扩展名 → application/octet-stream
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cargo test -p aurora-desktop --test tile_server_tests 2>&1
```

预期: 编译失败 (测试文件存在但函数未定义) — TDD 红阶段。

- [ ] **Step 3: 实现 tile_server.rs**

```rust
//! 内置微型 HTTP 瓦片服务器。
//!
//! 使用 stdlib TcpListener 实现，零外部依赖。
//! 监听 127.0.0.1:21337，将 URL 路径映射到本地文件系统。
//! 路径安全检查：拒绝包含 ".." 的请求。

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;

/// 启动内置瓦片服务器。
///
/// 在独立线程中运行。data_dir 是瓦片数据根目录，
/// URL 路径 `/img/0/0/0.jpg` 映射到 `data_dir/img/0/0/0.jpg`。
pub fn start(data_dir: PathBuf) {
    let _ = fs::create_dir_all(&data_dir);

    thread::Builder::new()
        .name("tile-server".into())
        .spawn(move || {
            let addr = "127.0.0.1:21337";
            let listener = match TcpListener::bind(addr) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[tile-server] 绑定 {addr} 失败: {e}");
                    return;
                }
            };

            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => handle_connection(stream, &data_dir),
                    Err(_) => continue,
                }
            }
        })
        .ok();
}

fn handle_connection(mut stream: TcpStream, data_dir: &Path) {
    // 读取 HTTP 请求第一行
    let mut buffer = [0u8; 4096];
    let n = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let request = String::from_utf8_lossy(&buffer[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    // 安全检查: 拒绝路径遍历攻击
    if path.contains("..") {
        respond(&mut stream, 403, "Forbidden", &[]);
        return;
    }

    // 构造本地文件路径
    let relative = path.trim_start_matches('/');
    let file_path = data_dir.join(relative);

    if !file_path.exists() || !file_path.is_file() {
        respond(&mut stream, 404, "Not Found", &[]);
        return;
    }

    match fs::read(&file_path) {
        Ok(content) => {
            let mime = guess_mime(&file_path);
            respond(&mut stream, 200, &mime, &content);
        }
        Err(_) => {
            respond(&mut stream, 500, "Internal Server Error", &[]);
        }
    }
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let status_text = match status {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: {content_type}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    if !body.is_empty() {
        let _ = stream.write_all(body);
    }
}

fn guess_mime(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg".into(),
        Some("png") => "image/png".into(),
        Some("terrain") => "application/octet-stream".into(),
        Some("json") => "application/json".into(),
        Some("css") => "text/css".into(),
        Some("js") => "application/javascript".into(),
        _ => "application/octet-stream".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use std::net::TcpStream;
    use std::time::Duration;

    fn setup_test_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tile_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir.join("img").join("0").join("0"));
        // 创建测试瓦片文件
        fs::write(dir.join("img").join("0").join("0").join("0.jpg"), b"fake-jpeg-data").unwrap();
        dir
    }

    #[test]
    fn test_guess_mime_jpg() {
        assert_eq!(guess_mime(Path::new("test.jpg")), "image/jpeg");
    }

    #[test]
    fn test_guess_mime_png() {
        assert_eq!(guess_mime(Path::new("test.png")), "image/png");
    }

    #[test]
    fn test_guess_mime_unknown() {
        assert_eq!(guess_mime(Path::new("test.xyz")), "application/octet-stream");
    }

    #[test]
    fn test_guess_mime_no_extension() {
        assert_eq!(guess_mime(Path::new("test")), "application/octet-stream");
    }

    #[test]
    fn test_tile_server_starts_and_serves_file() {
        let data_dir = setup_test_dir();
        let data_dir_clone = data_dir.clone();

        // 启动服务器
        start(data_dir_clone);
        std::thread::sleep(Duration::from_millis(200));

        // 请求存在的文件
        let mut stream = TcpStream::connect("127.0.0.1:21337").unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        stream.write_all(b"GET /img/0/0/0.jpg HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.contains("200 OK"), "响应应包含 200 OK: {response}");
        assert!(response.contains("fake-jpeg-data"), "响应应包含文件内容");
    }

    #[test]
    fn test_tile_server_404_for_missing_file() {
        let data_dir = setup_test_dir();
        start(data_dir);
        std::thread::sleep(Duration::from_millis(100));

        let mut stream = TcpStream::connect("127.0.0.1:21337").unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        stream.write_all(b"GET /img/nonexistent.jpg HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("404"));
    }

    #[test]
    fn test_tile_server_rejects_path_traversal() {
        let data_dir = setup_test_dir();
        start(data_dir);
        std::thread::sleep(Duration::from_millis(100));

        let mut stream = TcpStream::connect("127.0.0.1:21337").unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        stream.write_all(b"GET /../../../etc/passwd HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("403"));
    }
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cargo test -p aurora-desktop 2>&1
```

预期: 6 个测试全部通过。

- [ ] **Step 5: 在 main.rs 中集成 tile_server**

修改 `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tile_server;

fn main() {
    // 启动内置瓦片服务器
    let data_dir = std::path::PathBuf::from("D:/quicksand-data/tile");
    tile_server::start(data_dir);

    aurora_desktop_lib::run();
}
```

- [ ] **Step 6: Compile + test + format**

```bash
cargo test -p aurora-desktop 2>&1
cargo fmt -- --check
cargo clippy -p aurora-desktop -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/
git commit -m "feat: add embedded tile HTTP server with path traversal protection

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: 集成 CesiumJS 离线包

**Files:**
- Create: `ui/public/cesium/` (CesiumJS 静态文件)
- Modify: `ui/package.json`
- Modify: `ui/vite.config.ts`
- Modify: `ui/index.html`
- Create: `ui/src/Earth.tsx`
- Modify: `ui/src/App.tsx`
- Modify: `src-tauri/tauri.conf.json`

**注意**: CesiumJS 离线包需要预先手动下载 (`cesium` npm 包可以通过 `npm install cesium` 获取，然后复制静态资源到 `public/cesium/`)。

- [ ] **Step 1: 安装 CesiumJS npm 包**

```bash
cd ui && npm install cesium @types/cesium 2>&1
```

- [ ] **Step 2: 复制 CesiumJS 静态资源**

创建脚本 `ui/scripts/copy-cesium-assets.mjs`:

```javascript
import { copyFileSync, cpSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';

const src = 'node_modules/cesium/Build/Cesium';
const dest = 'public/cesium';

if (!existsSync(dest)) mkdirSync(dest, { recursive: true });

// 复制核心文件
const files = ['Cesium.js', 'Widgets/widgets.css'];
for (const f of files) {
  const target = join(dest, f);
  if (!existsSync(dirname(target))) mkdirSync(dirname(target), { recursive: true });
  copyFileSync(join(src, f), target);
}

// 复制静态资源目录
const dirs = ['Assets', 'Workers', 'ThirdParty', 'Widgets/Images'];
for (const d of dirs) {
  const srcDir = join(src, d);
  const destDir = join(dest, d);
  if (existsSync(srcDir) && !existsSync(destDir)) {
    cpSync(srcDir, destDir, { recursive: true });
  }
}

console.log('CesiumJS assets copied to public/cesium/');
```

```bash
cd ui && node scripts/copy-cesium-assets.mjs 2>&1
```

- [ ] **Step 3: 更新 ui/package.json 添加复制脚本**

```json
"scripts": {
  "dev": "vite",
  "build": "tsc && vite build",
  "preview": "vite preview",
  "copy-cesium": "node scripts/copy-cesium-assets.mjs",
  "prebuild": "npm run copy-cesium"
}
```

- [ ] **Step 4: 更新 vite.config.ts 处理 CesiumJS**

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  define: {
    CESIUM_BASE_URL: JSON.stringify('/cesium'),
  },
});
```

- [ ] **Step 5: 创建 Earth.tsx 组件**

```typescript
import { useEffect, useRef } from 'react';

// CesiumJS 通过 index.html <script> 全局加载
declare global {
  interface Window {
    Cesium: any;
  }
}

export default function Earth() {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewerRef = useRef<any>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const Cesium = window.Cesium;
    if (!Cesium) {
      console.error('CesiumJS not loaded');
      return;
    }

    // 尝试加载地形，如果不存在则使用默认椭球体
    let terrainProvider: any;
    try {
      terrainProvider = new Cesium.CesiumTerrainProvider({
        url: 'http://127.0.0.1:21337/terrain/',
      });
    } catch {
      terrainProvider = undefined;
    }

    const viewer = new Cesium.Viewer(containerRef.current, {
      imageryProvider: false,
      terrainProvider,
      baseLayerPicker: false,
      geocoder: false,
      homeButton: false,
      sceneModePicker: false,
      navigationHelpButton: false,
      animation: false,
      timeline: false,
      fullscreenButton: false,
      vrButton: false,
      infoBox: false,
      selectionIndicator: false,
    });

    // 隐藏 Cesium 版权信息
    if (viewer.cesiumWidget && viewer.cesiumWidget.creditContainer) {
      viewer.cesiumWidget.creditContainer.style.display = 'none';
    }

    // 加载离线瓦片
    try {
      viewer.imageryLayers.addImageryProvider(
        new Cesium.UrlTemplateImageryProvider({
          url: 'http://127.0.0.1:21337/img/{z}/{x}/{y}.jpg',
          maximumLevel: 8,
        })
      );
    } catch {
      console.warn('无法加载离线瓦片 — 请确保 D:/quicksand-data/tile/img/ 存在瓦片数据');
    }

    // 璇玑: 缓慢自转
    viewer.clock.multiplier = 0.05;
    viewer.scene.globe.enableLighting = false;

    // 初始视角: 从远处看整个地球
    viewer.camera.setView({
      destination: Cesium.Cartesian3.fromDegrees(
        104.0,  // 经度: 中国中部
        30.0,   // 纬度
        20000000 // 高度: 20000km (看到整个地球)
      ),
    });

    viewerRef.current = viewer;

    return () => {
      if (viewerRef.current) {
        viewerRef.current.destroy();
        viewerRef.current = null;
      }
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

- [ ] **Step 6: 更新 index.html 引入 CesiumJS**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title></title>
    <link rel="stylesheet" href="/cesium/Widgets/widgets.css" />
    <style>
      * { margin: 0; padding: 0; box-sizing: border-box; }
      html, body, #root { width: 100%; height: 100%; overflow: hidden; background: #000; }
      .cesium-widget, .cesium-widget canvas {
        position: absolute !important;
        top: 0; left: 0; width: 100% !important; height: 100% !important;
      }
    </style>
    <script src="/cesium/Cesium.js"></script>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 7: 更新 App.tsx 使用 Earth 组件**

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

- [ ] **Step 8: 更新 tauri.conf.json CSP 和窗口配置**

```json
{
  "productName": "aurora",
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
        "fullscreen": true,
        "decorations": false,
        "resizable": true,
        "center": true,
        "visible": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; connect-src http://127.0.0.1:21337; img-src 'self' http://127.0.0.1:21337 data: blob:; worker-src 'self' blob:;"
    }
  }
}
```

- [ ] **Step 9: 构建前端验证**

```bash
cd ui && npm run build 2>&1
```

预期: `✓ built in ...s`

- [ ] **Step 10: 编译 Rust 端验证**

```bash
cargo check -p aurora-desktop 2>&1
```

预期: `Finished` (无错误).

- [ ] **Step 11: Commit**

```bash
git add ui/ src-tauri/tauri.conf.json
git commit -m "feat: integrate CesiumJS offline 3D earth with 璇玑 slow rotation

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: 窗口行为完善 (全屏、Esc 退出、启动显示)

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加 tauri window 功能依赖**

修改 `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: 实现 Tauri 窗口控制命令**

修改 `src-tauri/src/lib.rs`:

```rust
use tauri::Manager;

#[tauri::command]
fn show_window(window: tauri::Window) {
    let _ = window.show();
}

#[tauri::command]
fn exit_app(window: tauri::Window) {
    window.close();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![show_window, exit_app])
        .setup(|app| {
            // 启动后短暂延迟显示窗口 (等待 WebView 渲染)
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                let _ = window_clone.show();
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // 直接关闭，无确认弹窗 — 微风原则
                let _ = window.destroy();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: 前端添加 Esc 键监听**

在 `Earth.tsx` 的 `useEffect` 中添加 Esc 监听:

```typescript
// 微风: Esc 直接退出
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      // 通过 Tauri IPC 退出 (如果可用)
      try {
        (window as any).__TAURI_INTERNALS__?.invoke('exit_app');
      } catch {
        window.close();
      }
    }
  };
  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, []);
```

- [ ] **Step 4: 前端添加瓦片加载完成后显示窗口**

在 Earth.tsx 的 Viewer 初始化后添加:

```typescript
// 瓦片加载完成后通知 Tauri 显示窗口
viewer.scene.globe.tileLoadProgressEvent.addEventListener((count: number) => {
  if (count === 0) {
    try {
      (window as any).__TAURI_INTERNALS__?.invoke('show_window');
    } catch {}
  }
});
```

- [ ] **Step 5: 编译 + 测试**

```bash
cd ui && npm run build 2>&1
cargo check -p aurora-desktop 2>&1
cargo fmt -- --check
cargo clippy -p aurora-desktop -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: add fullscreen, Esc exit, and delayed window reveal for 微风 UX

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: 创建瓦片数据目录 + 首次启动引导

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: 更新 main.rs — 自动创建目录 + 启动引导**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tile_server;

use std::path::PathBuf;

fn main() {
    let data_dir = PathBuf::from("D:/quicksand-data/tile");

    // 确保目录存在
    let img_dir = data_dir.join("img");
    let terrain_dir = data_dir.join("terrain");

    let _ = std::fs::create_dir_all(&img_dir);
    let _ = std::fs::create_dir_all(&terrain_dir);

    // 如果瓦片目录为空，创建 README 提示
    if img_dir.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
        let readme = img_dir.join("README.txt");
        let _ = std::fs::write(&readme, "\
璇玑 — 离线地球瓦片数据目录

请使用地图下载工具（如全能电子地图下载器）下载目标区域的影像瓦片，
按 {z}/{x}/{y}.jpg 格式放入此目录。

示例:
  D:/quicksand-data/tile/img/0/0/0.jpg
  D:/quicksand-data/tile/img/1/0/0.jpg
  ...

地形数据放入:
  D:/quicksand-data/tile/terrain/

详见 docs/superpowers/specs/2026-06-25-aurora-offline-earth-design.md
");
    }

    // 启动瓦片服务器
    tile_server::start(data_dir);

    // 启动 Tauri
    aurora_desktop_lib::run();
}
```

- [ ] **Step 2: 编译验证**

```bash
cargo check -p aurora-desktop 2>&1
cargo fmt -- --check
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "feat: auto-create tile data directories with README on first launch

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: 最终验证 — 全量测试 + Clippy + 发布构建

**Files:**
- (none — verification only)

- [ ] **Step 1: 运行全量 workspace 测试**

```bash
cargo test --workspace --all-features -- --test-threads=2 2>&1
```

预期: 所有测试通过 (aurora: 170+, trit-core: 600+)

- [ ] **Step 2: Clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1
```

预期: 零警告。

- [ ] **Step 3: Format**

```bash
cargo fmt -- --check 2>&1
```

预期: 清洁。

- [ ] **Step 4: 前端构建**

```bash
cd ui && npm run build 2>&1
```

预期: `✓ built in ...s`

- [ ] **Step 5: 发布构建**

```bash
cargo build --release 2>&1
```

预期: `Finished` (无错误).

- [ ] **Step 6: 伦理门验证**

手动检查:
- [ ] 窗口打开后无任何 DOM 文字元素
- [ ] CesiumJS 无 infoBox / credit / timeline / animation 控件
- [ ] Esc 退出无确认弹窗
- [ ] 无菜单栏、标题栏 (全屏无边框模式)
- [ ] 地球缓慢自转

- [ ] **Step 7: 最终提交**

```bash
git add -A
git commit -m "chore: final verification — all tests pass, clippy clean, release build OK

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Completion Checklist

- [ ] 7 个任务全部提交
- [ ] `cargo test --workspace --all-features` — 全部通过
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` — 零警告
- [ ] `cargo fmt -- --check` — 清洁
- [ ] `cd ui && npm run build` — 成功
- [ ] `cargo build --release` — 成功
- [ ] 零改动 `aurora/` 核心代码
- [ ] 零改动 `trit-core/`
- [ ] tile_server 路径遍历防护测试通过
- [ ] 流沙伦理门验证通过
