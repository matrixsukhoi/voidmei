import React from 'react'
import ReactDOM from 'react-dom/client'
import { ConfigProvider, theme } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import App from './App'

// 深色主题起步 (HUD 工具气质; 阶段②按 cfg 细化)
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ConfigProvider locale={zhCN} theme={{ algorithm: theme.darkAlgorithm }}>
      <App />
    </ConfigProvider>
  </React.StrictMode>,
)
