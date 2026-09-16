// 亮色粉白主题 — 色板对位 PinkStyle.java (Hot Pink 主色/白卡片/浅灰底/细灰边)
// 主窗与编辑窄面板共用 (窗口路由各自的 ConfigProvider 载体)
import type { ThemeConfig } from 'antd'

export const pinkTheme: ThemeConfig = {
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
}
