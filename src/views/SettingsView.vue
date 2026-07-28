<script setup lang="ts">
  import { computed, onMounted, reactive, ref } from 'vue'
  import {
    NAlert,
    NAvatar,
    NButton,
    NCard,
    NForm,
    NFormItem,
    NInput,
    NInputGroup,
    NSelect,
    NSpace,
    NTag,
    NTooltip,
    useDialog,
    useMessage,
  } from 'naive-ui'
  import { open } from '@tauri-apps/plugin-dialog'
  import { open as openExternal } from '@tauri-apps/plugin-shell'
  import { LogoGithub } from '@vicons/ionicons5'
  import authorAvatar from '@/assets/huey1in.png'
  import LanguageSwitch from '../components/LanguageSwitch.vue'
  import InboundSelector from '../components/InboundSelector.vue'
  import CloseTypeSelector from '../components/CloseTypeSelector.vue'
  import {
    activate,
    changePassword,
    closeKiro,
    launchKiro,
    restoreOriginalMachineId,
    setKiroPath,
  } from '@/api'
  import { useAppStore, useKiroStore, useUserStore } from '@/stores'
  import packageInfo from '../../package.json'

  const kiro = useKiroStore()
  const appStore = useAppStore()
  const userStore = useUserStore()
  const version = packageInfo.version
  const message = useMessage()
  const dialog = useDialog()
  const activationCode = ref('')
  const kiroPath = ref('')
  const activating = ref(false)
  const savingPath = ref(false)
  const processBusy = ref(false)
  const passwordBusy = ref(false)
  const password = reactive({ current: '', next: '', confirm: '' })

  const processLabel = computed(() => (kiro.status?.running ? '运行中' : '未运行'))
  const buttonModeOptions = [
    { label: '简洁模式', value: 'simple' },
    { label: '高级模式', value: 'advanced' },
  ]
  const buttonMode = computed({
    get: () => (appStore.showAllButtons ? 'advanced' : 'simple'),
    set: (value: string) => appStore.setButtonVisibility(value === 'advanced'),
  })

  async function refreshKiro() {
    try {
      await kiro.refresh()
      kiroPath.value = kiro.status?.executablePath || kiroPath.value
    } catch {
      // 未配置路径时仍允许用户手动选择。
    }
  }

  async function redeem() {
    if (!activationCode.value.trim()) {
      message.warning('请输入激活码')
      return
    }
    activating.value = true
    try {
      await activate(activationCode.value.trim())
      activationCode.value = ''
      message.success('激活成功')
    } catch (error) {
      message.error(error instanceof Error ? error.message : String(error))
    } finally {
      activating.value = false
    }
  }

  async function chooseKiro() {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Kiro', extensions: ['exe'] }],
    })
    if (typeof selected === 'string') kiroPath.value = selected
  }

  async function savePath() {
    savingPath.value = true
    try {
      await setKiroPath(kiroPath.value)
      await refreshKiro()
      message.success('Kiro 路径已保存')
    } catch (error) {
      message.error(error instanceof Error ? error.message : String(error))
    } finally {
      savingPath.value = false
    }
  }

  async function toggleKiro() {
    processBusy.value = true
    try {
      if (kiro.status?.running) await closeKiro()
      else await launchKiro()
      await refreshKiro()
    } catch (error) {
      message.error(error instanceof Error ? error.message : String(error))
    } finally {
      processBusy.value = false
    }
  }

  async function updatePassword() {
    if (!password.current || !password.next || !password.confirm) {
      message.warning('请填写完整密码')
      return
    }
    if (password.next !== password.confirm) {
      message.warning('两次输入的新密码不一致')
      return
    }
    passwordBusy.value = true
    try {
      await changePassword(password.current, password.next)
      password.current = password.next = password.confirm = ''
      message.success('密码已修改')
    } catch (error) {
      message.error(error instanceof Error ? error.message : String(error))
    } finally {
      passwordBusy.value = false
    }
  }

  async function logout() {
    try {
      await userStore.logout()
    } catch (error) {
      message.error(error instanceof Error ? error.message : '退出登录失败')
    }
  }

  async function openGitHub(url: string) {
    try {
      await openExternal(url)
    } catch (error) {
      message.error(error instanceof Error ? error.message : '无法打开 GitHub 主页')
    }
  }

  function confirmRestoreMachineId() {
    dialog.warning({
      title: '恢复原始 MachineGuid',
      content: '这会修改 Windows 系统级机器标识，需要管理员权限。请先关闭 Kiro。',
      positiveText: '确认恢复',
      negativeText: '取消',
      onPositiveClick: async () => {
        try {
          if (kiro.status?.running) await closeKiro()
          const restored = await restoreOriginalMachineId()
          await refreshKiro()
          message.success(`已恢复 ${restored}`)
        } catch (error) {
          message.error(error instanceof Error ? error.message : String(error))
        }
      },
    })
  }

  onMounted(refreshKiro)
</script>

<template>
  <n-space
    vertical
    :size="24"
  >
    <n-card>
      <template #header>
        <div class="text-xl font-medium">系统控制</div>
      </template>
      <n-space
        vertical
        :size="16"
      >
        <div class="flex items-center justify-between">
          <div>
            Kiro 客户端状态：
            <n-tag
              :type="kiro.status?.running ? 'success' : 'warning'"
              size="small"
              round
            >
              {{ processLabel }}
            </n-tag>
          </div>
          <div class="flex gap-2">
            <n-button
              :loading="processBusy"
              :disabled="kiro.status?.running"
              type="primary"
              @click="toggleKiro"
            >
              启动 Kiro
            </n-button>
            <n-button
              :loading="processBusy"
              :disabled="!kiro.status?.running"
              type="warning"
              @click="toggleKiro"
            >
              关闭 Kiro
            </n-button>
          </div>
        </div>

        <div class="flex items-center justify-between gap-2 mt-1">
          <n-tooltip
            placement="bottom"
            trigger="hover"
            style="max-width: 400px; white-space: normal; word-break: break-word"
          >
            <template #trigger>
              <span class="text-sm overflow-hidden text-ellipsis whitespace-nowrap path-text">
                Kiro 路径：{{ kiroPath || '未设置' }}
              </span>
            </template>
            <div style="white-space: normal; word-break: break-all">
              {{ kiroPath || '未设置' }}
            </div>
          </n-tooltip>
          <n-button @click="chooseKiro">更改路径</n-button>
          <n-button
            type="primary"
            :loading="savingPath"
            @click="savePath"
          >
            保存
          </n-button>
        </div>
      </n-space>
    </n-card>

    <n-card>
      <template #header>
        <div class="text-xl font-medium">全局偏好设置</div>
      </template>
      <div class="p-5">
        <div class="grid grid-cols-2 gap-x-8 gap-y-6 preferences-grid">
          <div class="flex items-center">
            <div class="text-base w-20">线路</div>
            <div class="flex-1">
              <inbound-selector
                :show-label="false"
                class="settings-selector"
              />
            </div>
          </div>
          <div class="flex items-center">
            <div class="text-base w-20">关闭方式</div>
            <div class="flex-1">
              <close-type-selector
                :show-label="false"
                class="settings-selector"
              />
            </div>
          </div>
          <div class="flex items-center">
            <div class="text-base w-20">语言</div>
            <div class="flex-1">
              <language-switch
                :show-label="false"
                class="settings-selector"
              />
            </div>
          </div>
          <div class="flex items-center">
            <div class="text-base w-20">操作模式</div>
            <div class="flex-1">
              <n-select
                v-model:value="buttonMode"
                :options="buttonModeOptions"
                size="small"
                class="w-full"
              />
            </div>
          </div>
        </div>
      </div>
    </n-card>

    <n-card>
      <template #header>
        <div class="text-xl font-medium">账号与机器绑定</div>
      </template>
      <n-alert
        type="warning"
        :show-icon="true"
      >
        每个账号永久绑定一个 MachineGuid。切换时账号凭据和机器码会一起写入，失败会自动回滚。
      </n-alert>
      <div class="machine-row">
        <span>当前 MachineGuid</span>
        <code>{{ kiro.machineId || '无法读取' }}</code>
      </div>
      <n-button
        type="warning"
        secondary
        @click="confirmRestoreMachineId"
      >
        恢复原始机器码
      </n-button>
    </n-card>

    <n-card>
      <template #header>
        <div class="text-xl font-medium">激活码兑换</div>
      </template>
      <n-space
        vertical
        :size="16"
      >
        <div class="flex items-center justify-between">
          <div style="width: 80px">激活码</div>
          <div class="flex-1">
            <n-input-group>
              <n-input
                v-model:value="activationCode"
                placeholder="请输入激活码"
                class="flex-1"
                @keyup.enter="redeem"
              />
              <n-button
                type="primary"
                :loading="activating"
                @click="redeem"
              >
                激活
              </n-button>
            </n-input-group>
          </div>
        </div>
      </n-space>
    </n-card>

    <n-card>
      <template #header>
        <div class="text-xl font-medium">修改密码</div>
      </template>
      <n-space
        vertical
        :size="16"
      >
        <n-form
          :model="password"
          label-placement="left"
          label-width="100"
          require-mark-placement="right-hanging"
        >
          <n-form-item label="当前密码">
            <n-input
              v-model:value="password.current"
              type="password"
              show-password-on="click"
              maxlength="20"
              minlength="6"
            />
          </n-form-item>
          <n-form-item label="新密码">
            <n-input
              v-model:value="password.next"
              type="password"
              show-password-on="click"
              maxlength="20"
              minlength="6"
            />
          </n-form-item>
          <n-form-item label="确认密码">
            <n-input
              v-model:value="password.confirm"
              type="password"
              show-password-on="click"
              maxlength="20"
              minlength="6"
            />
          </n-form-item>
          <div style="margin-top: 12px">
            <n-space>
              <n-button
                type="primary"
                :loading="passwordBusy"
                @click="updatePassword"
              >
                修改密码
              </n-button>
              <n-button
                type="error"
                @click="logout"
              >
                退出登录
              </n-button>
            </n-space>
          </div>
        </n-form>
      </n-space>
    </n-card>

    <n-card>
      <template #header>
        <div class="text-xl font-medium">关于</div>
      </template>
      <n-space
        vertical
        :size="12"
      >
        <p>Kiro Pool v{{ version }}</p>
        <div class="author-profile">
          <n-avatar
            :size="48"
            :src="authorAvatar"
            alt="huey1in 的 GitHub 头像"
            class="author-avatar"
            @click="openGitHub('https://github.com/huey1in')"
          />
          <div class="author-details">
            <span>本项目作者：huey1in</span>
            <n-button
              text
              type="primary"
              @click="openGitHub('https://github.com/huey1in')"
            >
              <template #icon><LogoGithub /></template>
              GitHub 主页
            </n-button>
          </div>
        </div>
        <span class="upstream-note">本项目基于 CursorPool_Client 开发</span>
        <div class="original-authors">
          <strong>原作者</strong>
          <span>Cloxl、Sanyela</span>
          <n-button
            text
            type="primary"
            @click="openGitHub('https://github.com/Cloxl/CursorPool_Client')"
          >
            <template #icon><LogoGithub /></template>
            原始代码仓库
          </n-button>
        </div>
      </n-space>
    </n-card>
  </n-space>
</template>

<style scoped>
  .settings-selector :deep(.n-select),
  .settings-selector :deep(.n-base-selection) {
    width: 100% !important;
  }

  .path-text {
    flex: 1;
  }

  .machine-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin: 18px 0;
  }

  .author-profile {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .original-authors {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .upstream-note {
    color: var(--n-text-color-3);
    font-size: 13px;
  }

  .author-avatar {
    flex: 0 0 auto;
    cursor: pointer;
    border-radius: 6px;
  }

  .author-details {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
  }

  code {
    font-family: 'Fira Code', monospace;
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  @media (max-width: 760px) {
    .preferences-grid {
      grid-template-columns: 1fr;
    }

    .machine-row {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
