import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Vite 默认给 <script> 添加 crossorigin 属性，
// 但 tauri.localhost 自定义协议不支持 CORS，
// 导致 JS 加载失败。此插件在构建后移除 crossorigin。
function removeCrossorigin() {
  return {
    name: 'remove-crossorigin',
    transformIndexHtml(html: string) {
      return html.replace(/ crossorigin/g, '');
    },
  };
}

export default defineConfig({
  plugins: [react(), removeCrossorigin()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        manualChunks: {
          // ponytail: maplibre/pmtiles/protomaps 是 2D 面板重依赖，动态 import，
          // 单独拆 chunk 避免主 bundle 膨胀，且只在打开 2D 面板时加载。
          maplibre: ['maplibre-gl'],
          pmtiles: ['pmtiles', '@protomaps/basemaps'],
        },
      },
    },
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  define: {
    CESIUM_BASE_URL: JSON.stringify('/cesium'),
  },
});
