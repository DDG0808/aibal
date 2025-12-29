import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { resolve } from 'path';

// Tauri 需要的环境变量
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],

  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@contracts': resolve(__dirname, 'contracts'),
    },
  },

  // 阻止 Vite 清屏，以便看到 Rust 日志
  clearScreen: false,

  // Tauri 需要固定端口
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Tauri 监听 src-tauri 目录
      ignored: ['**/src-tauri/**'],
    },
  },

  // 生产构建配置
  build: {
    // Tauri 需要 ES2021
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    // 不压缩以便调试
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    // 生成 sourcemap 以便调试
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
