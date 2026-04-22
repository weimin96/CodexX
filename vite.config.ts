import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { NaiveUiResolver } from 'unplugin-vue-components/resolvers'
import { resolve } from 'path'

function matchesNodeModule(id: string, packageName: string): boolean {
  return id.includes(`/node_modules/${packageName}/`) || id.includes(`\\node_modules\\${packageName}\\`)
}

export default defineConfig({
  plugins: [
    vue(),
    AutoImport({
      imports: [
        'vue',
        'vue-router',
        'pinia',
        {
          'naive-ui': ['useDialog', 'useMessage', 'useNotification', 'useLoadingBar'],
        },
      ],
      dts: 'src/auto-imports.d.ts',
    }),
    Components({
      resolvers: [NaiveUiResolver()],
      dts: 'src/components.d.ts',
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) {
            return undefined
          }

          // 设计取舍：
          // - framework 单独拆分，保证 Vue 基础运行时稳定缓存。
          // - echarts 与 vue-echarts 体积大且只在额度图表中需要，单独拆分可明显降低首屏路由体积。
          // - tauri API 单独拆分，避免桌面能力与普通页面代码混成一个大公共包。
          // - naive-ui 保持默认拆分策略，避免其内部模块形成循环 chunk 依赖。
          if (
            matchesNodeModule(id, 'vue') ||
            matchesNodeModule(id, '@vue') ||
            matchesNodeModule(id, 'vue-router') ||
            matchesNodeModule(id, 'pinia')
          ) {
            return 'framework'
          }

          if (matchesNodeModule(id, 'date-fns') || matchesNodeModule(id, 'date-fns-tz')) {
            return 'date-utils'
          }

          if (
            matchesNodeModule(id, 'echarts') ||
            matchesNodeModule(id, 'zrender') ||
            matchesNodeModule(id, 'vue-echarts')
          ) {
            return 'charts'
          }

          if (id.includes('/node_modules/@tauri-apps/') || id.includes('\\node_modules\\@tauri-apps\\')) {
            return 'tauri'
          }

          return undefined
        },
      },
    },
  },
})
