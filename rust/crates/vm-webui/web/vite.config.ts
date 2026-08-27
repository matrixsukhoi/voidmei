import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// 嵌入式前端 (tauri generate_context! 编译期吃 dist): base 必须相对路径
export default defineConfig({
  base: './',
  plugins: [react()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    // WebView2 (Chromium) 目标, 无需 legacy
    target: 'chrome110',
  },
  clearScreen: false,
})
