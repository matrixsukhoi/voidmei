/**
 * 前端纯函数测试: COLOR 行值解析/格式化 (与 vm-ui color.rs 的十进制/hex 语义对齐)
 * + 阶段③纯函数 (URL 切分/desc-img 归一/语音包名/VC 键码转换)。
 * (tauri invoke 依赖 webview 环境, 不在此测 — Rust 侧 ipc/dto 已有对应单测。)
 */
import { describe, expect, it } from 'vitest'
import {
  browserCodeToVc,
  extractLatestVersion,
  hasNewerVersion,
  normalizeDescImg,
  parseColorValue,
  parseVoicePackValue,
  rgbaToHex,
  splitUrlText,
  vcToKeyName,
} from './api'

describe('parseColorValue (color.rs 同语义)', () => {
  it('十进制串 "232, 147, 50, 200" → rgba', () => {
    expect(parseColorValue('232, 147, 50, 200')).toEqual([232, 147, 50, 200])
  })
  it('十进制缺空格也可', () => {
    expect(parseColorValue('1,2,3,255')).toEqual([1, 2, 3, 255])
  })
  it('hex 8 位 → rgba', () => {
    expect(parseColorValue('#FFC864FF')).toEqual([255, 200, 100, 255])
  })
  it('非法值 → 白色兜底', () => {
    expect(parseColorValue('')).toEqual([255, 255, 255, 255])
    expect(parseColorValue(null)).toEqual([255, 255, 255, 255])
  })
  it('rgbaToHex 往返', () => {
    expect(rgbaToHex([232, 147, 50, 200])).toBe('#E89332C8')
    expect(parseColorValue(rgbaToHex([10, 20, 30, 40]))).toEqual([10, 20, 30, 40])
  })
})

describe('splitUrlText (InfoRowRenderer.URL_PATTERN 同源)', () => {
  it('前文本 + URL + 尾文本 三段切分', () => {
    expect(splitUrlText('仓库 https://github.com/x/voidmei 邮箱：a@b.c')).toEqual([
      { type: 'text', value: '仓库 ' },
      { type: 'url', value: 'https://github.com/x/voidmei' },
      { type: 'text', value: ' 邮箱：a@b.c' },
    ])
  })
  it('URL 在末尾: 前文本 + URL', () => {
    expect(splitUrlText('链接 http://a.b/c')).toEqual([
      { type: 'text', value: '链接 ' },
      { type: 'url', value: 'http://a.b/c' },
    ])
  })
  it('多 URL 各自成段', () => {
    const segs = splitUrlText('https://a.b https://c.d')
    expect(segs.filter((s) => s.type === 'url').map((s) => s.value)).toEqual(['https://a.b', 'https://c.d'])
    expect(segs[1]).toEqual({ type: 'text', value: ' ' })
  })
  it(') 截断 URL (防吞括号)', () => {
    expect(splitUrlText('(https://a.b/x)后文')).toEqual([
      { type: 'text', value: '(' },
      { type: 'url', value: 'https://a.b/x' },
      { type: 'text', value: ')后文' },
    ])
  })
  it('无 URL → 单段 text; 空串 → 空数组', () => {
    expect(splitUrlText('纯文本')).toEqual([{ type: 'text', value: '纯文本' }])
    expect(splitUrlText('')).toEqual([])
  })
})

describe('normalizeDescImg (ReplicaBuilder.java:625 同语义)', () => {
  it('裸文件名加 image/ 前缀', () => {
    expect(normalizeDescImg('aoa.png')).toBe('image/aoa.png')
  })
  it('已带 image/ 或 image\\ 前缀原样', () => {
    expect(normalizeDescImg('image/aoa.png')).toBe('image/aoa.png')
    expect(normalizeDescImg('image\\aoa.png')).toBe('image\\aoa.png')
  })
})

describe('parseVoicePackValue (VoicePackConfig.parse 语义, 分隔符 |)', () => {
  it('空值/null 回退 default', () => {
    expect(parseVoicePackValue('')).toBe('default')
    expect(parseVoicePackValue(null)).toBe('default')
    expect(parseVoicePackValue(undefined)).toBe('default')
  })
  it('"pack|enabled" 取 pack 段 (Java toConfigString 格式)', () => {
    expect(parseVoicePackValue('jarvis|true')).toBe('jarvis')
    expect(parseVoicePackValue('jarvis|false')).toBe('jarvis')
  })
  it('裸包名原样; 空包段回退 default (Java 构造器 isEmpty→default)', () => {
    expect(parseVoicePackValue('default')).toBe('default')
    expect(parseVoicePackValue('|true')).toBe('default')
  })
})

describe('VC 键码转换 (evdev 域; VC_P=25)', () => {
  it('字母/数字/功能键映射 (KeyboardEvent.code → VC)', () => {
    expect(browserCodeToVc('KeyP')).toBe(25)
    expect(browserCodeToVc('KeyA')).toBe(30)
    expect(browserCodeToVc('Digit1')).toBe(2)
    expect(browserCodeToVc('Escape')).toBe(1)
    expect(browserCodeToVc('F12')).toBe(88)
    expect(browserCodeToVc('Space')).toBe(57)
  })
  it('Lock 三键与未知键 → null (录制态忽略)', () => {
    expect(browserCodeToVc('CapsLock')).toBeNull()
    expect(browserCodeToVc('NumLock')).toBeNull()
    expect(browserCodeToVc('ScrollLock')).toBeNull()
    expect(browserCodeToVc('UnknownKey')).toBeNull()
  })
  it('VC → 键名 (getKeyText 近似)', () => {
    expect(vcToKeyName(25)).toBe('P')
    expect(vcToKeyName(1)).toBe('Esc')
    expect(vcToKeyName(0)).toBeNull()
    expect(vcToKeyName(9999)).toBeNull()
  })
})

describe('extractLatestVersion (Application.java:464-472 逐句对位)', () => {
  it('典型 GitHub 响应: tag_name 段截取 + 正则提取', () => {
    // 真实 releases/latest 响应形态 (字段顺序: url ... html_url ... tag_name ...)
    const res = `{"url":"https://api.github.com/repos/matrixsukhoi/voidmei/releases/1",
      "html_url":"https://github.com/matrixsukhoi/voidmei/releases/tag/1.590",
      "tag_name": "1.590","target_commitish":"master","name":"v1.590"}`
    expect(extractLatestVersion(res)).toBe('1.590')
  })
  it('v 前缀 tag: 正则跳过非数字前缀取数字段 (Java [0-9].([0-9])* 同源)', () => {
    expect(extractLatestVersion(`{"tag_name":"v2.01","draft":false}`)).toBe('2.01')
  })
  it('无 tag_name → null (Java indexOf=-1, substring 抛异常被线程池吞 = 静默)', () => {
    expect(extractLatestVersion('{"message":"Not Found"}')).toBeNull()
  })
  it('tag_name 后无逗号 → null (Java substring(sidx,-1) 抛异常面)', () => {
    expect(extractLatestVersion('{"tag_name":"1.590"')).toBeNull()
  })
  it('tag_name 段无数字 → null (m.find()==false 分支)', () => {
    expect(extractLatestVersion('{"tag_name":"beta","x":1}')).toBeNull()
  })
})

describe('hasNewerVersion (Double.parseDouble 数值比较, Application.java:474)', () => {
  it('数值比较非字符串比较: 2.0 < 10.0', () => {
    expect(hasNewerVersion('2.0', '10.0')).toBe(true)
  })
  it('尾零等值: 1.590 与 1.59 数值相等 → 不弹窗', () => {
    expect(hasNewerVersion('1.590', '1.59')).toBe(false)
  })
  it('旧 < 新 / 旧 > 新', () => {
    expect(hasNewerVersion('1.589', '1.590')).toBe(true)
    expect(hasNewerVersion('1.591', '1.590')).toBe(false)
  })
})
