import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { refreshInbound } from '@/api'
import { nanoid } from 'nanoid'
import Logger from '@/utils/logger'

// 网络请求状态类型
export interface RequestStatus {
  requestId: string
  endpoint: string
  isRetrying: boolean
  retryCount: number
  maxRetries: number
  lastError?: string
}

// 网络状态事件
export enum NetworkEvents {
  REQUEST_RETRY = 'request-retry',
  REQUEST_FAILED = 'request-failed',
  INBOUND_REFRESHING = 'inbound-refreshing',
  INBOUND_REFRESHED = 'inbound-refreshed',
}

// 全局默认配置
const DEFAULT_CONFIG = {
  maxRetries: 1, // 最大重试次数 (从2改为1)
  retryDelay: 0, // 初始重试延迟（设为0，不延迟）
  useExponentialBackoff: false, // 是否使用指数退避策略 (关闭)
  refreshInboundOnMaxRetries: true, // 重试失败后是否刷新线路
  showRetryNotification: true, // 是否显示重试通知
}

// 事件订阅者列表
const subscribers: Record<string, Array<(data: any) => void>> = {}
// 跟踪已发送的事件，防止重复发送相同事件
const sentEvents = new Map<string, number>()
// 跟踪正在进行中的请求，防止并发相同请求
const inFlightRequests = new Map<string, Promise<any>>()

// ApiClient 单例
export class ApiClient {
  private static instance: ApiClient
  private activeRequests: Map<string, RequestStatus> = new Map()
  private refreshInboundPromise: Promise<boolean> | null = null
  private inboundRefreshedListener: (() => void) | null = null
  private config = { ...DEFAULT_CONFIG }
  private isRefreshingInbound = false // 标记是否正在刷新线路，防止重复刷新
  private currentRetryOrRefreshAction: string | null = null // 标记当前是否有重试或刷新操作

  private constructor() {
    // 监听inbound-refreshed事件
    this.setupInboundRefreshedListener()
  }

  public static getInstance(): ApiClient {
    if (!ApiClient.instance) {
      ApiClient.instance = new ApiClient()
    }
    return ApiClient.instance
  }

  // 设置配置项
  public configure(config: Partial<typeof DEFAULT_CONFIG>): void {
    this.config = { ...this.config, ...config }
  }

  // 发起API请求，支持重试
  public async request<T>(
    command: string,
    params: Record<string, any> = {},
    options?: Partial<typeof DEFAULT_CONFIG>,
  ): Promise<T> {
    // 合并配置
    const requestConfig = { ...this.config, ...options }
    const requestId = nanoid(8)
    // 为无参数或简单参数的命令生成一个简单的请求键
    const requestKey = `${command}_${JSON.stringify(params || {})}`

    // 如果相同的请求正在进行中，则返回该请求的Promise
    if (inFlightRequests.has(requestKey)) {
      await Logger.info(`请求 ${requestKey} 已在进行中，复用现有请求`)
      return inFlightRequests.get(requestKey)!
    }

    // 初始化请求状态
    const requestStatus: RequestStatus = {
      requestId,
      endpoint: command,
      isRetrying: false,
      retryCount: 0,
      maxRetries: requestConfig.maxRetries,
    }

    this.activeRequests.set(requestId, requestStatus)

    const requestPromise = this.executeRequest<T>(command, params, requestStatus, requestConfig)
    inFlightRequests.set(requestKey, requestPromise)

    try {
      return await requestPromise
    } finally {
      // 清理请求状态和进行中的请求标记
      this.activeRequests.delete(requestId)
      inFlightRequests.delete(requestKey)
    }
  }

  // 执行请求并处理重试逻辑
  private async executeRequest<T>(
    command: string,
    params: Record<string, any>,
    status: RequestStatus,
    config: typeof DEFAULT_CONFIG,
  ): Promise<T> {
    try {
      // 记录请求开始
      await Logger.info(`开始请求 ${status.endpoint} (ID: ${status.requestId})`)

      // 执行实际请求
      return await invoke<T>(status.endpoint, params || {})
    } catch (error) {
      // 更新错误信息
      const errorMessage = error instanceof Error ? error.message : String(error)
      status.lastError = errorMessage

      // 记录请求失败
      await Logger.error(`请求 ${status.endpoint} (ID: ${status.requestId}) 失败: ${errorMessage}`)

      // 如果已经有其他请求正在重试或刷新线路，则当前请求直接抛出错误，避免连锁反应
      if (
        this.currentRetryOrRefreshAction &&
        this.currentRetryOrRefreshAction !== `${status.endpoint}_retry` &&
        this.currentRetryOrRefreshAction !== `${status.endpoint}_refresh`
      ) {
        await Logger.warn(
          `已有操作 ${this.currentRetryOrRefreshAction} 进行中，请求 ${status.endpoint} (ID: ${status.requestId}) 失败后不再自动重试或刷新。`,
        )
        throw error // 直接抛出错误，让调用方处理，或等待当前操作完成后再由用户决定是否重试
      }

      // 检查是否还可以重试
      if (status.retryCount < status.maxRetries) {
        this.currentRetryOrRefreshAction = `${status.endpoint}_retry`
        try {
          status.retryCount++
          status.isRetrying = true

          await Logger.warn(
            `正在重试 ${status.endpoint} (ID: ${status.requestId}), 第 ${status.retryCount}/${status.maxRetries} 次`,
          )

          this.publishEvent(
            NetworkEvents.REQUEST_RETRY,
            {
              ...status,
              delay: 0,
            },
            `${status.endpoint}_retry_${status.retryCount}`,
          )

          return await this.executeRequest<T>(command, params, status, config) // 注意：这里递归调用
        } finally {
          this.currentRetryOrRefreshAction = null
        }
      } else {
        // 已达到最大重试次数
        status.isRetrying = false
        await Logger.error(
          `请求 ${status.endpoint} (ID: ${status.requestId}) 达到最大重试次数 ${status.maxRetries}`,
        )
        this.publishEvent(NetworkEvents.REQUEST_FAILED, status, `${status.endpoint}_failed`)

        // 如果配置为在最大重试后刷新线路，并且当前没有其他线路刷新操作
        if (config.refreshInboundOnMaxRetries && !this.isRefreshingInbound) {
          this.currentRetryOrRefreshAction = `${status.endpoint}_refresh`
          this.isRefreshingInbound = true // 标记开始刷新
          try {
            await Logger.info(`开始刷新线路 (由 ${status.endpoint} (ID: ${status.requestId}) 触发)`)

            await this.refreshInbound() // 调用刷新线路的函数

            await Logger.info(`线路刷新后重试 ${status.endpoint} (ID: ${status.requestId})`)
            // 线路刷新完成后，再次尝试原始请求 (只尝试一次)
            return await invoke<T>(status.endpoint, params || {})
          } catch (refreshError) {
            const refreshErrorMsg =
              refreshError instanceof Error ? refreshError.message : String(refreshError)
            await Logger.error(
              `线路刷新失败 (由 ${status.endpoint} (ID: ${status.requestId}) 触发): ${refreshErrorMsg}`,
            )
            throw error // 线路刷新也失败，则抛出原始错误
          } finally {
            this.isRefreshingInbound = false // 标记刷新结束
            this.currentRetryOrRefreshAction = null
          }
        } else {
          if (this.isRefreshingInbound) {
            await Logger.warn(
              `请求 ${status.endpoint} (ID: ${status.requestId}) 失败，但线路已在刷新中，不再重复触发刷新。`,
            )
          }
          throw error // 不刷新线路或已在刷新，则直接抛出原始错误
        }
      }
    }
  }

  // 刷新线路（保证同一时间只有一个刷新请求）
  private async refreshInbound(): Promise<boolean> {
    // 发布线路刷新开始事件
    this.publishEvent(
      NetworkEvents.INBOUND_REFRESHING,
      { timestamp: Date.now() },
      'inbound_refreshing',
    )

    // 如果已经有刷新过程在进行，直接返回该Promise
    if (this.refreshInboundPromise) {
      await Logger.info('已有线路刷新进行中，等待完成')
      return this.refreshInboundPromise
    }

    try {
      await Logger.info('开始刷新线路流程')

      // 创建新的刷新Promise
      this.refreshInboundPromise = new Promise<boolean>((resolve, reject) => {
        // 设置超时以防止永久等待
        const timeoutId = setTimeout(() => {
          Logger.error('线路刷新超时')
          reject(new Error('Inbound refresh timeout'))
        }, 30000) // 30秒超时

        // 创建一次性监听器，等待后端的刷新完成事件
        const unlistenPromise = listen('inbound-refreshed', () => {
          clearTimeout(timeoutId)
          Logger.info('收到线路刷新完成事件')
          this.publishEvent(NetworkEvents.INBOUND_REFRESHED, { success: true }, 'inbound_refreshed')
          resolve(true)
        })

        // 然后调用刷新API
        refreshInbound().catch((e) => {
          clearTimeout(timeoutId)
          unlistenPromise.then((unlisten) => unlisten())
          const errMsg = e instanceof Error ? e.message : String(e)
          Logger.error(`刷新线路API调用失败: ${errMsg}`)
          reject(e)
        })
      })

      // 等待刷新完成
      return await this.refreshInboundPromise
    } finally {
      // 重置Promise以允许将来的刷新
      this.refreshInboundPromise = null
    }
  }

  // 订阅网络事件
  public static subscribe(event: NetworkEvents, callback: (data: any) => void): () => void {
    if (!subscribers[event]) {
      subscribers[event] = []
    }
    subscribers[event].push(callback)

    // 返回取消订阅函数
    return () => {
      subscribers[event] = subscribers[event].filter((cb) => cb !== callback)
    }
  }

  // 发布事件
  private publishEvent(event: NetworkEvents, data: any, eventKey?: string): void {
    // 如果提供了事件键，并且该事件最近已经发送过，则不重复发送
    if (eventKey) {
      const now = Date.now()
      const lastSentTime = sentEvents.get(eventKey)

      if (lastSentTime && now - lastSentTime < 1000) {
        // 1秒内不重复发送相同事件
        return
      }

      // 记录事件发送时间
      sentEvents.set(eventKey, now)

      // 清理旧事件记录
      if (sentEvents.size > 100) {
        const oldKeys = Array.from(sentEvents.keys()).slice(0, 50)
        oldKeys.forEach((key) => sentEvents.delete(key))
      }
    }

    if (subscribers[event]) {
      subscribers[event].forEach((callback) => callback(data))
    }
  }

  // 设置inbound-refreshed事件监听器
  private async setupInboundRefreshedListener(): Promise<void> {
    // 确保不重复监听
    if (this.inboundRefreshedListener) return

    this.inboundRefreshedListener = await listen('inbound-refreshed', () => {
      this.publishEvent(NetworkEvents.INBOUND_REFRESHED, { timestamp: Date.now() })
    })
  }

  // 获取当前活动重试请求
  public getActiveRetries(): RequestStatus[] {
    return Array.from(this.activeRequests.values()).filter((r) => r.isRetrying)
  }

  // 清理资源（在应用关闭前调用）
  public cleanup(): void {
    if (this.inboundRefreshedListener) {
      this.inboundRefreshedListener()
      this.inboundRefreshedListener = null
    }
  }
}

// 提供一个全局访问点
export const apiClient = ApiClient.getInstance()
