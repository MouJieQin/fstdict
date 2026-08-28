import { defineConfig } from "vite";
import vue from '@vitejs/plugin-vue';
import { fileURLToPath, URL } from 'node:url';
import { visualizer } from 'rollup-plugin-visualizer';
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers'

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  base: './',

  plugins: [vue(),
  AutoImport({
    resolvers: [ElementPlusResolver()],
  }),
  Components({
    resolvers: [ElementPlusResolver()],
  }),
  visualizer({ open: false })
  ],

  build: {
    // Vite 8 uses Rolldown configurations here
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: 'icons-vendor',
              // Target vue-icons-plus package files
              test: /node_modules[\\/]vue-icons-plus/,
              priority: 20,
            },
            {
              name: 'element-vendor',
              // Target element-plus UI components to lighten index.js further
              test: /node_modules[\\/]element-plus/,
              priority: 10,
            }
          ]
        }
      }
    }
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available

  server: {
    port: 9595,
    strictPort: true,
    host: '127.0.0.1',

    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**", 'node_modules', 'src-python', 'src-helper'],
    },
  },
}));
