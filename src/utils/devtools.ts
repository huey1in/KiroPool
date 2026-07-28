import { openDevTools } from '../api'

export class DevToolsManager {
  private static instance: DevToolsManager
  private konamiCode: string[] = []
  private konamiSequence = [
    'ArrowUp',
    'ArrowUp',
    'ArrowDown',
    'ArrowDown',
    'ArrowLeft',
    'ArrowRight',
    'ArrowLeft',
    'ArrowRight',
    'b',
    'a',
  ]
  private isInitialized = false

  private constructor() {
    if (typeof window !== 'undefined') {
      this.initialize()
    }
  }

  public static getInstance(): DevToolsManager {
    if (!DevToolsManager.instance) {
      DevToolsManager.instance = new DevToolsManager()
    }
    return DevToolsManager.instance
  }

  private initialize() {
    if (this.isInitialized) return

    // 禁用右键菜单
    document.addEventListener('contextmenu', (e) => {
      e.preventDefault()
      return false
    })

    // 禁用常见开发者工具快捷键
    document.addEventListener('keydown', (e) => {
      this.konamiCode.push(e.key)
      if (this.konamiCode.length > this.konamiSequence.length) {
        this.konamiCode.shift()
      }

      // 检查是否匹配konami代码
      if (this.checkKonamiCode()) {
        openDevTools()
        this.konamiCode = []
        return
      }

      // 禁用各种开发者工具快捷键
      const forbiddenKeys = [
        { key: 'F12' },
        { key: 'i', ctrl: true, shift: true },
        { key: 'j', ctrl: true, shift: true },
        { key: 'c', ctrl: true, shift: true },
        { key: 'k', ctrl: true, shift: true },
        { key: 'u', ctrl: true },
        { key: 's', ctrl: true },
        { key: 'p', ctrl: true, shift: true },
      ]

      for (const combo of forbiddenKeys) {
        if (e.key === combo.key && (!combo.ctrl || e.ctrlKey) && (!combo.shift || e.shiftKey)) {
          e.preventDefault()
          return false
        }
      }
    })

    this.isInitialized = true
  }

  private checkKonamiCode(): boolean {
    return this.konamiCode.join(',') === this.konamiSequence.join(',')
  }
}

// 导出初始化函数
export const initializeDevToolsProtection = () => {
  DevToolsManager.getInstance()
}
