/**
 * 批3小件弹窗组 (独立组件, 不碰 rows.tsx 的表单分发):
 * - checkUpdate (Application.java:451-484): web 就绪后异步一次; dev 守卫 /
 *   tag_name 截取 / 正则提取 / 数值比较全保真 (纯函数在 api.ts, vitest 钉住);
 *   有新版弹 Modal.info (showUpdateDialog 文案对位 Lang), 链接经 opener 打开浏览器
 *   (Java Desktop.browse); 检查失败静默继续 (Java logAndContinue)。
 * - about-requested: 托盘"关于" (Application.java:236-245 三段 showAbout 文案,
 *   Rust 主循环 emit, Lang 单一来源) → About Modal (批3裁决: 附版本号)。
 * - config-dialog: ConfigManager 弹窗 (ConfigManager.java:425-477) →
 *   parse-error 走 Modal.error (ERROR_MESSAGE) / merge-report 走 Modal.info
 *   (INFORMATION_MESSAGE); web 未就绪期的弹窗由 Rust 侧日志兜底。
 */
import React, { useEffect, useRef } from 'react'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { fetch } from '@tauri-apps/plugin-http'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Modal, Typography } from 'antd'
import { assetRootOnce, extractLatestVersion, getAppVersion, hasNewerVersion } from './api'

const { Link, Paragraph } = Typography

/** Java Application.java:404 RELEASE_URL (发布页, 弹窗链接的跳转目标) */
const RELEASE_URL = 'https://github.com/matrixsukhoi/voidmei/releases'

/** GitHub API 端点 (Java owner/repository 拼接, Application.java:70-71 + :462) */
const LATEST_RELEASE_API = 'https://api.github.com/repos/matrixsukhoi/voidmei/releases/latest'

/**
 * showUpdateDialog (Application.java:410-436): Lang.mUpdateAvailableContent 的
 * %s 两段 (latestVersion, version) + 链接行 (mUpdateAvailableLinkText),
 * INFORMATION_MESSAGE → Modal.info。
 */
function showUpdateDialog(latestVersion: string, version: string) {
  Modal.info({
    title: '发现新版本', // Lang.mUpdateAvailableTitle
    content: (
      <div>
        <p>GitHub上已发布新版本: {latestVersion}</p>
        <p>当前版本: {version}</p>
        <p>请点击下方链接下载更新。</p>
        {/* Lang.mUpdateAvailableLinkText; Java Desktop.browse → opener (scope 限 releases 页) */}
        <Link
          onClick={() =>
            openUrl(RELEASE_URL).catch((e) => console.warn('[Update] Failed to open browser:', e))
          }
        >
          前往下载页面
        </Link>
      </div>
    ),
  })
}

/** checkUpdate 的前端形态 (Application.java:451-484) */
async function checkUpdate(version: string) {
  // dev 版 (本地未打 jar) 无版本号可比, 跳过更新检查, 避免 Double.parseDouble 崩溃
  if (version === 'dev') return
  try {
    // 超时兜底 (审查 W5): 挂起网络不让 promise 永久 pending; Java HttpHelper 无
    // 显式超时, 失败面同为静默 catch (logAndContinue)
    const res = await fetch(LATEST_RELEASE_API, { signal: AbortSignal.timeout(15_000) })
    const text = await res.text()
    const latest = extractLatestVersion(text)
    if (latest === null) return // 正则不中: Java m.find()==false 分支, 静默
    console.info(`[Update] Latest remote version: ${latest}`) // Java Logger.info("Update", ...)
    if (hasNewerVersion(version, latest)) showUpdateDialog(latest, version)
  } catch (e) {
    // 检查更新失败，使用统一异常处理 (Java logAndContinue — 静默继续)
    console.warn('[Update] 检查更新失败:', e)
  }
}

/** about-requested 载荷 (vm-webui bridge.rs AboutPayload) */
interface AboutPayload {
  version: string
  /** [aboutcontent, aboutcontentsub1, aboutcontentsub2] (Lang 单一来源) */
  contents: [string, string, string]
}

/** 托盘关于 Modal: Java 三段 showAbout 通知 (sub2 24s/sub1 16s/main 8s 叠放,
 *  图标 image/fubuki.jpg) → 单窗三段呈现 (次序 1→2→3), 版本号批3裁决附带。
 *  关闭回执 (审查 B1): Rust 侧的 Modal 展示期标记由此清除 → 恢复 InGame 收窗
 *  (Java 通知弹窗独立于 MainForm 可见性, 游戏中"关于"恒可读) */
function showAboutModal(p: AboutPayload, fubuki: string | null) {
  Modal.info({
    title: `关于 VoidMei v${p.version}`,
    width: 560,
    content: (
      <div style={{ display: 'flex', gap: 12, alignItems: 'flex-start' }}>
        {fubuki && (
          <img src={fubuki} alt="" width={72} style={{ flexShrink: 0, borderRadius: 4 }} />
        )}
        <div>
          {p.contents.map((c, i) => (
            <Paragraph key={i} style={{ whiteSpace: 'pre-wrap', marginBottom: 4 }}>
              {c}
            </Paragraph>
          ))}
        </div>
      </div>
    ),
    afterClose: () => {
      invoke('about_modal_closed').catch((e) => console.warn('[About] 关闭回执失败:', e))
    },
  })
}

/** config-dialog 载荷 (vm-webui bridge.rs ConfigDialogPayload) */
interface ConfigDialogPayload {
  kind: 'parse-error' | 'merge-report'
  title: string
  message: string
}

/** ConfigManager 弹窗 (ConfigManager.java:425-477): ERROR_MESSAGE / INFORMATION_MESSAGE */
function showConfigDialog({ kind, title, message }: ConfigDialogPayload) {
  if (kind === 'parse-error') {
    Modal.error({ title, width: 520, content: <p style={{ whiteSpace: 'pre-wrap' }}>{message}</p> })
  } else {
    Modal.info({
      title,
      width: 520,
      content: <p style={{ whiteSpace: 'pre-wrap', margin: 0 }}>{message}</p>,
    })
  }
}

/** 弹窗宿主 (挂载即注册监听, 渲染 null); ready 后跑一次 checkUpdate */
export const AppDialogs: React.FC<{ ready: boolean }> = ({ ready }) => {
  // 单次守卫: Java checkUpdate 每次启动一次 (StrictMode 双挂载/重渲不重查)
  const checkedRef = useRef(false)
  // 关于弹窗的 fubuki 图标 (image/fubuki.jpg, asset protocol; 失败不显示图)。
  // ref 承载: 监听闭包在挂载时注册一次, 读 ref.current 恒见最新值 (无陈旧闭包)
  const fubukiRef = useRef<string | null>(null)

  useEffect(() => {
    assetRootOnce()
      .then((root) => {
        fubukiRef.current = convertFileSrc(`${root}/image/fubuki.jpg`)
      })
      .catch(() => undefined)
    // 托盘关于 → About Modal (Rust 主循环 emit)
    const un1 = listen<AboutPayload>('about-requested', (e) =>
      showAboutModal(e.payload, fubukiRef.current),
    )
    // config_manager 弹窗 → Modal.error / Modal.info
    const un2 = listen<ConfigDialogPayload>('config-dialog', (e) => showConfigDialog(e.payload))
    return () => {
      un1.then((f) => f())
      un2.then((f) => f())
    }
  }, [])

  useEffect(() => {
    // 启动后异步一次 (Java: EDT 上 Controller 创建后 checkUpdate; web 就绪后此处)
    if (!ready || checkedRef.current) return
    checkedRef.current = true
    getAppVersion()
      .then(checkUpdate)
      .catch(() => undefined)
  }, [ready])

  return null
}
