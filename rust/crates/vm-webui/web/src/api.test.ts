/**
 * 前端纯函数测试: COLOR 行值解析/格式化 (与 vm-ui color.rs 的十进制/hex 语义对齐)
 * + 阶段③纯函数 (URL 切分/desc-img 归一/语音包名/VC 键码转换)。
 * (tauri invoke 依赖 webview 环境, 不在此测 — Rust 侧 ipc/dto 已有对应单测。)
 */
import { describe, expect, it } from 'vitest'
import {
  browserCodeToVc,
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

describe('parseVoicePackValue (VoicePackConfig.parse 语义)', () => {
  it('空值/null 回退 default', () => {
    expect(parseVoicePackValue('')).toBe('default')
    expect(parseVoicePackValue(null)).toBe('default')
    expect(parseVoicePackValue(undefined)).toBe('default')
  })
  it('"pack:enabled" 取 pack 段', () => {
    expect(parseVoicePackValue('jarvis:true')).toBe('jarvis')
    expect(parseVoicePackValue('default')).toBe('default')
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
