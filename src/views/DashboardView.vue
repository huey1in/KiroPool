<script setup lang="ts">
  import { computed, onMounted } from 'vue'
  import {
    NButton,
    NCard,
    NDivider,
    NGrid,
    NGridItem,
    NNumberAnimation,
    NProgress,
    NSpace,
    NTag,
    useDialog,
    useMessage,
  } from 'naive-ui'
  import { RefreshOutline, SwapHorizontalOutline } from '@vicons/ionicons5'
  import { useKiroStore, useUserStore } from '@/stores'

  const kiro = useKiroStore()
  const user = useUserStore()
  const dialog = useDialog()
  const message = useMessage()

  const accountEmail = computed(
    () =>
      kiro.usage?.userEmail ||
      kiro.currentAccount?.email ||
      (kiro.status?.authenticated ? '本地 Kiro 已登录' : '本地 Kiro 未登录'),
  )
  const provider = computed(
    () => kiro.currentAccount?.provider || kiro.status?.provider || '未绑定',
  )
  const subscription = computed(
    () => kiro.usage?.subscriptionTitle || kiro.usage?.subscriptionType || '未读取',
  )
  const creditUsedPercent = computed(() =>
    user.userInfo?.totalCredits
      ? Math.min(100, (user.userInfo.usedCredits / user.userInfo.totalCredits) * 100)
      : 0,
  )
  const kiroRemainingPercent = computed(() =>
    kiro.usage?.limit ? Math.min(100, (kiro.remainingCredits / kiro.usage.limit) * 100) : 0,
  )
  const cardExpiry = computed(() => {
    const info = user.userInfo
    if (!info || info.code_status !== 1) {
      return { status: '未激活', date: '激活卡密后生效', type: 'warning' as const }
    }
    if (!info.expireTime) {
      return { status: '永久有效', date: '无到期时间', type: 'success' as const }
    }

    const expiry = parseServerDate(info.expireTime)
    if (Number.isNaN(expiry.getTime())) {
      return { status: info.expireTime, date: '', type: 'default' as const }
    }

    const remainingDays = Math.ceil((expiry.getTime() - Date.now()) / 86_400_000)
    if (remainingDays <= 0 || info.isExpired) {
      return { status: '已到期', date: expiry.toLocaleString(), type: 'error' as const }
    }
    return {
      status: `剩余 ${remainingDays} 天`,
      date: expiry.toLocaleString(),
      type: remainingDays <= 7 ? ('warning' as const) : ('success' as const),
    }
  })

  async function refresh() {
    try {
      await Promise.all([kiro.refresh(), user.checkLoginStatus()])
    } catch (error) {
      message.error(error instanceof Error ? error.message : String(error))
    }
  }

  async function performSwitch(forceClose: boolean) {
    try {
      const result = await kiro.switchAccount({ forceClose, launchAfterSwitch: true })
      await user.checkLoginStatus()
      if (result.launchError) {
        message.warning(`已切换到 ${result.email}，但启动 Kiro 失败：${result.launchError}`)
      } else {
        message.success(`已切换到 ${result.email}`)
      }
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error)
      if (detail.includes('KIRO_RUNNING')) {
        confirmRunningSwitch()
        return
      }
      message.error(detail)
    }
  }

  function confirmRunningSwitch() {
    dialog.warning({
      title: 'Kiro 正在运行',
      content: '切换账号需要关闭 Kiro，并同时写入该账号绑定的机器码。',
      positiveText: '关闭并切换',
      negativeText: '取消',
      onPositiveClick: () => performSwitch(true),
    })
  }

  function switchAccount() {
    if (kiro.status?.running) confirmRunningSwitch()
    else void performSwitch(false)
  }

  function formatDate(value?: string) {
    if (!value) return '无'
    const date = parseServerDate(value)
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString()
  }

  function parseServerDate(value: string) {
    return new Date(value.includes('T') ? value : value.replace(' ', 'T'))
  }

  onMounted(refresh)
</script>

<template>
  <n-space
    vertical
    size="large"
  >
    <n-grid
      :cols="2"
      :x-gap="24"
      style="display: grid; grid-template-columns: repeat(2, 1fr)"
    >
      <n-grid-item style="display: grid">
        <n-card
          title="账号信息"
          class="user-info-card"
          style="height: 100%; user-select: none"
        >
          <n-space
            vertical
            :size="12"
          >
            <n-space
              :size="8"
              style="line-height: 1.2"
            >
              <span class="field-label">账号</span>
              <span class="field-value">{{ accountEmail }}</span>
            </n-space>
            <n-divider style="margin: 0" />
            <n-space
              :size="8"
              style="line-height: 1.2"
            >
              <span class="field-label">认证方式</span>
              <n-tag size="tiny">{{ provider }}</n-tag>
            </n-space>
            <n-space
              :size="8"
              style="line-height: 1.2"
            >
              <span class="field-label">订阅</span>
              <span class="field-value">{{ subscription }}</span>
            </n-space>
            <n-space
              :size="8"
              style="line-height: 1.2"
            >
              <span class="field-label">登录状态</span>
              <n-tag
                :type="kiro.status?.authenticated ? 'success' : 'warning'"
                size="tiny"
              >
                {{ kiro.status?.authenticated ? '本地已登录' : '未登录' }}
              </n-tag>
            </n-space>
            <n-space
              :size="8"
              style="line-height: 1.2"
            >
              <span class="field-label">重置时间</span>
              <span class="field-value">{{ formatDate(kiro.usage?.nextResetAt) }}</span>
            </n-space>
            <span class="machine-code">{{ kiro.machineId || '无法读取 MachineGuid' }}</span>
          </n-space>
        </n-card>
      </n-grid-item>

      <n-grid-item style="display: grid">
        <n-card
          title="使用统计"
          style="height: 100%; user-select: none"
        >
          <n-space
            vertical
            :size="24"
            style="height: 100%; justify-content: space-around"
          >
            <n-space
              vertical
              :size="8"
            >
              <n-space justify="space-between">
                <span>当前账号剩余额度</span>
                <n-space :size="0">
                  <n-number-animation
                    :from="0"
                    :to="kiro.remainingCredits"
                    :duration="1000"
                  />
                  <span>/{{ kiro.usage?.limit || 0 }}</span>
                </n-space>
              </n-space>
              <n-progress
                type="line"
                status="success"
                :percentage="kiroRemainingPercent"
                :show-indicator="false"
                :height="12"
                :border-radius="6"
                :processing="kiro.loading || user.isCheckingLogin"
              />
            </n-space>

            <n-space
              vertical
              :size="8"
            >
              <n-space justify="space-between">
                <span>积分使用</span>
                <span>
                  {{ user.userInfo?.usedCredits || 0 }}/{{ user.userInfo?.totalCredits || 0 }}
                </span>
              </n-space>
              <n-progress
                type="line"
                status="success"
                :percentage="creditUsedPercent"
                :show-indicator="false"
                :height="12"
                :border-radius="6"
                :processing="kiro.loading || user.isCheckingLogin"
              />
            </n-space>

            <n-space
              vertical
              :size="8"
            >
              <n-space justify="space-between">
                <span>卡密有效期</span>
                <n-tag
                  :type="cardExpiry.type"
                  size="small"
                >
                  {{ cardExpiry.status }}
                </n-tag>
              </n-space>
              <span class="expiry-date">{{ cardExpiry.date }}</span>
            </n-space>
          </n-space>
        </n-card>
      </n-grid-item>
    </n-grid>

    <n-card
      title="快捷操作"
      class="quick-actions-card"
      style="user-select: none"
    >
      <n-space justify="center">
        <n-button
          :loading="kiro.loading"
          style="width: 160px"
          @click="refresh"
        >
          <template #icon><RefreshOutline /></template>
          刷新状态
        </n-button>
        <n-button
          type="primary"
          :loading="kiro.switching"
          style="width: 200px"
          @click="switchAccount"
        >
          <template #icon><SwapHorizontalOutline /></template>
          切换账号与机器码
        </n-button>
      </n-space>
    </n-card>
  </n-space>
</template>

<style scoped>
  .field-label {
    width: 70px;
    flex: 0 0 70px;
  }

  .field-value {
    min-width: 0;
    font-size: 14px;
    overflow-wrap: anywhere;
  }

  .machine-code {
    font-size: 12px;
    color: #999;
    word-break: break-all;
    text-align: center;
    font-family: 'Fira Code', monospace;
  }

  .expiry-date {
    min-height: 20px;
    color: #888;
    font-size: 13px;
    text-align: right;
  }

  @media (max-width: 760px) {
    :deep(.n-grid) {
      grid-template-columns: 1fr !important;
      row-gap: 16px;
    }
  }
</style>
