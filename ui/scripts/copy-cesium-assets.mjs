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
