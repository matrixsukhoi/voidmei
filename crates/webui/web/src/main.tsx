import React from 'react'
import ReactDOM from 'react-dom/client'
import { ConfigProvider } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import App from './App'
import { ComparisonWindowRoot } from './windows/comparison'
import { PowerCurveWindowRoot } from './windows/powercurve'
import { pinkTheme } from './theme'
import './index.css'

// 窗口路由 (批3): 同一 frontendDist 服务多窗口 — index.html?win=<kind> 区分,
// web_windows.rs 动态建窗时注入 query (Java 多 JDialog → 多 WebviewWindow 对位)
const winKind = new URLSearchParams(window.location.search).get('win')

if (winKind === 'comparison') {
  // 对比窗口: 自带暗色主题 (Java 窗体暗色, 主窗粉白主题不串染)
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <ComparisonWindowRoot />
    </React.StrictMode>,
  )
} else if (winKind === 'powercurve') {
  // 功率曲线窗口 ("功率曲线")
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <PowerCurveWindowRoot />
    </React.StrictMode>,
  )
} else {
  // 主窗 (亮色粉白主题, theme.ts 单一来源)
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <ConfigProvider locale={zhCN} theme={pinkTheme}>
        <App />
      </ConfigProvider>
    </React.StrictMode>,
  )
}
