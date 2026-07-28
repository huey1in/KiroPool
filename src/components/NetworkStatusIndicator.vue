<script setup lang="ts">
  import { ref, onMounted, onUnmounted, computed } from 'vue'
  import { useMessage } from 'naive-ui'
  import { NSpin } from 'naive-ui'
  import { ApiClient, NetworkEvents, RequestStatus } from '@/utils/apiClient'
  import { useI18n } from '@/locales'

  const message = useMessage()
  const { t } = useI18n()

  // 状态
  const activeRetries = ref<RequestStatus[]>([])
  const isRefreshingInbound = ref(false)

  // 计算属性 - 是否有活动的重试
  const hasActiveRetries = computed(() => activeRetries.value.length > 0)

  // 计算属性 - 获取最新的重试请求
  const latestRetry = computed(() => {
    if (activeRetries.value.length === 0) return null
    return activeRetries.value[activeRetries.value.length - 1]
  })

  // 显示重试消息
  function showRetryMessage(status: RequestStatus & { delay: number }) {
    message.info(
      t('network.retrying', {
        endpoint: status.endpoint,
        attempt: status.retryCount,
        max: status.maxRetries,
      }),
    )
  }

  // 显示线路刷新消息
  function showInboundRefreshingMessage() {
    message.info(t('network.refreshingInbound'))
  }

  // 显示线路刷新完成消息
  function showInboundRefreshedMessage() {
    message.success(t('network.inboundRefreshed'))
  }

  // 初始化事件监听
  onMounted(() => {
    // 监听请求重试事件
    const retryUnsub = ApiClient.subscribe(
      NetworkEvents.REQUEST_RETRY,
      (status: RequestStatus & { delay: number }) => {
        // 添加到活动重试列表
        activeRetries.value = [...activeRetries.value, status]

        // 显示重试通知
        showRetryMessage(status)

        // 设置一个定时器，在重试完成后从列表中移除
        setTimeout(() => {
          activeRetries.value = activeRetries.value.filter((r) => r.requestId !== status.requestId)
        }, 3000) // 使用固定的3秒时间
      },
    )

    // 监听线路刷新开始事件
    const refreshingUnsub = ApiClient.subscribe(NetworkEvents.INBOUND_REFRESHING, () => {
      isRefreshingInbound.value = true
      showInboundRefreshingMessage()
    })

    // 监听线路刷新完成事件
    const refreshedUnsub = ApiClient.subscribe(NetworkEvents.INBOUND_REFRESHED, () => {
      isRefreshingInbound.value = false
      showInboundRefreshedMessage()
    })

    // 清理订阅
    onUnmounted(() => {
      retryUnsub()
      refreshingUnsub()
      refreshedUnsub()
    })
  })
</script>

<template>
  <div
    v-if="hasActiveRetries || isRefreshingInbound"
    class="network-status-indicator"
  >
    <transition name="fade">
      <div
        v-if="isRefreshingInbound"
        class="status refreshing"
      >
        <NSpin size="small" />
        <span>{{ t('network.refreshingInbound') }}</span>
      </div>
      <div
        v-else-if="latestRetry"
        class="status retrying"
      >
        <NSpin size="small" />
        <span>
          {{
            t('network.retrying', {
              endpoint: latestRetry.endpoint,
              attempt: latestRetry.retryCount,
              max: latestRetry.maxRetries,
            })
          }}
        </span>
      </div>
    </transition>
  </div>
</template>

<style scoped>
  .network-status-indicator {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 9999;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 4px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    animation: pulse 1.5s infinite;
    transition: all 0.3s ease;
  }

  .refreshing {
    background-color: var(--primary-color);
    color: white;
  }

  .retrying {
    background-color: var(--warning-color);
    color: white;
  }

  .fade-enter-active,
  .fade-leave-active {
    transition: opacity 0.3s;
  }

  .fade-enter-from,
  .fade-leave-to {
    opacity: 0;
  }

  @keyframes pulse {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.8;
    }
    100% {
      opacity: 1;
    }
  }
</style>
