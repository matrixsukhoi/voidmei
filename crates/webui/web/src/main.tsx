import React from 'react'
import ReactDOM from 'react-dom/client'
import { ConfigProvider } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import App from './App'
import { ComparisonWindowRoot } from './windows/comparison'
import { PowerCurveWindowRoot } from './windows/powercurve'
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
  // 亮色粉白主题 — 色板对位 PinkStyle.java (Hot Pink 主色/白卡片/浅灰底/细灰边)
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <ConfigProvider
        locale={zhCN}
        theme={{
          // antd 亮色即默认算法 (无 lightAlgorithm; 原深色为 darkAlgorithm 已弃用)
          token: {
            colorPrimary: '#FF69B4', // PinkStyle.COLOR_PRIMARY (255,105,180)
            colorBgLayout: '#F5F5F5', // PinkStyle.COLOR_BG_MAIN
            colorBgContainer: '#FFFFFF', // PinkStyle.COLOR_BG_PANEL
            colorBorder: '#D9D9D9',
            colorBorderSecondary: '#E6E6E6', // PinkStyle.COLOR_BORDER
            colorText: '#333333', // PinkStyle.COLOR_TEXT
            colorTextSecondary: '#777777',
            borderRadius: 6,
            fontSize: 13,
          },
          components: {
            Tabs: { itemSelectedColor: '#FF69B4', inkBarColor: '#FF69B4', horizontalItemPadding: '6px 12px' },
            Switch: { trackHeight: 22, trackMinWidth: 44, handleSize: 16 },
            Tooltip: { fontSize: 12 },
          },
        }}
      >
        <App />
      </ConfigProvider>
    </React.StrictMode>,
  )
}
