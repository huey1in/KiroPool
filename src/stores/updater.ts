import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import Logger from '@/utils/logger'

const LATEST_RELEASE_API = 'https://api.github.com/repos/huey1in/KiroPool/releases/latest'

async function hasPublishedRelease() {
  const response = await fetch(LATEST_RELEASE_API, {
    headers: { Accept: 'application/vnd.github+json' },
  })
  if (response.status === 404) return false
  if (!response.ok) {
    throw new Error(`GitHub Release API 返回 HTTP ${response.status}`)
  }
  return true
}

export const useUpdaterStore = defineStore('updater', () => {
  // 状态
  const isChecking = ref(false)
  const isDownloading = ref(false)
  const isInstalling = ref(false)
  const hasUpdate = ref(false)
  const downloadProgress = ref(0)
  const downloadedBytes = ref(0)
  const totalBytes = ref(0)
  const updateVersion = ref('')
  const updateNotes = ref('')
  const error = ref<string | null>(null)

  // 计算属性
  const isUpdating = computed(() => isChecking.value || isDownloading.value || isInstalling.value)
  const progressPercentage = computed(() => {
    if (totalBytes.value === 0) return 0
    return Math.round((downloadedBytes.value / totalBytes.value) * 100)
  })

  // 检查更新
  async function checkForUpdates() {
    if (isUpdating.value) return

    try {
      isChecking.value = true
      error.value = null
      hasUpdate.value = false

      if (!(await hasPublishedRelease())) {
        Logger.info('更新信息: GitHub 仓库暂未发布 Release')
        return
      }

      const update = await check()
      Logger.info(`更新信息: ${JSON.stringify(update)}`)

      if (update) {
        hasUpdate.value = true
        updateVersion.value = update.version
        updateNotes.value = update.body || ''

        // 立即开始下载和安装，无需用户确认
        await installUpdate(update)
      }
    } catch (err) {
      // 获取错误消息，处理不同类型的错误
      const errorMessage = err instanceof Error ? err.message : String(err)

      if (err instanceof Error) {
        // 保存完整错误信息
        error.value = `错误: ${err.message}\n${err.stack || ''}`

        // 检查是否是网络错误
        if (
          err.message.includes('network') ||
          err.message.includes('连接') ||
          err.message.includes('connect') ||
          err.message.includes('timeout')
        ) {
          error.value = `网络连接问题: ${err.message}`
        }

        // 检查是否是签名验证错误
        if (err.message.includes('signature') || err.message.includes('签名')) {
          error.value = `更新签名验证失败: ${err.message}`
        }

        // 检查是否是解析错误
        if (err.message.includes('parse') || err.message.includes('json')) {
          error.value = `更新信息解析错误: ${err.message}`
        }
      } else {
        // 未知类型的错误
        error.value = `检查更新失败: ${errorMessage}`
      }

      Logger.error(`更新错误: ${error.value}`)
    } finally {
      isChecking.value = false
    }
  }

  // 下载并安装更新
  async function installUpdate(update: any) {
    try {
      isDownloading.value = true
      error.value = null
      downloadProgress.value = 0
      downloadedBytes.value = 0
      totalBytes.value = 0

      // 下载并安装，监听进度事件
      await update.downloadAndInstall((event: any) => {
        switch (event.event) {
          case 'Started':
            totalBytes.value = event.data.contentLength || 0
            break

          case 'Progress':
            downloadedBytes.value += event.data.chunkLength || 0
            // 确保进度不超过100%
            if (totalBytes.value > 0) {
              downloadProgress.value = Math.min(
                Math.round((downloadedBytes.value / totalBytes.value) * 100),
                99, // 保留安装的1%
              )
            }
            break

          case 'Finished':
            downloadProgress.value = 99
            break
        }
      })

      // 如果代码执行到这里，说明下载完成并准备安装
      isDownloading.value = false
      isInstalling.value = true
      downloadProgress.value = 100

      // 添加延迟确保用户能看到100%完成状态
      await new Promise((resolve) => setTimeout(resolve, 1000))

      // Windows会自动重启，其他平台需要手动重启
      await relaunch()
    } catch (err) {
      isDownloading.value = false
      isInstalling.value = false

      // 统一错误消息格式
      const errorMessage = err instanceof Error ? err.message : String(err)

      if (err instanceof Error) {
        // 保存完整错误信息
        error.value = `安装错误: ${err.message}\n${err.stack || ''}`

        // 检查是否是下载错误
        if (err.message.includes('download') || err.message.includes('下载')) {
          error.value = `更新包下载失败: ${err.message}`
        }

        // 检查是否是权限错误
        if (err.message.includes('permission') || err.message.includes('权限')) {
          error.value = `安装权限不足: ${err.message}`
        }

        // 检查是否是文件损坏
        if (
          err.message.includes('corrupt') ||
          err.message.includes('损坏') ||
          err.message.includes('invalid') ||
          err.message.includes('无效')
        ) {
          error.value = `更新包损坏或无效: ${err.message}`
        }
      } else {
        // 未知类型的错误
        error.value = `更新安装失败: ${errorMessage}`
      }

      Logger.error(`更新安装错误: ${error.value}`)
    }
  }

  return {
    // 状态
    isChecking,
    isDownloading,
    isInstalling,
    hasUpdate,
    downloadProgress,
    downloadedBytes,
    totalBytes,
    updateVersion,
    updateNotes,
    error,
    isUpdating,
    progressPercentage,

    // 方法
    checkForUpdates,
  }
})
