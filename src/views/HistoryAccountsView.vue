<script setup lang="ts">
  import { h, onMounted, ref } from 'vue'
  import { NButton, NCard, NDataTable, NIcon, NSpace, NTag, useDialog, useMessage } from 'naive-ui'
  import { SwapHorizontalOutline, TrashOutline } from '@vicons/ionicons5'
  import type { DataTableColumns } from 'naive-ui'
  import type { KiroHistoryEntry } from '@/api/types'
  import { useKiroStore } from '@/stores'

  const kiro = useKiroStore()
  const dialog = useDialog()
  const message = useMessage()
  const operatingId = ref<number | null>(null)

  async function switchAccount(row: KiroHistoryEntry) {
    operatingId.value = row.id
    try {
      const result = await kiro.switchHistoryAccount(row, {
        forceClose: true,
        launchAfterSwitch: true,
      })
      if (result.syncError) {
        message.warning(`已切换到 ${result.email}，但云端凭证同步失败：${result.syncError}`)
      } else {
        message.success(`已切换到 ${result.email}`)
      }
    } catch (error) {
      message.error(error instanceof Error ? error.message : String(error))
    } finally {
      operatingId.value = null
    }
  }

  function confirmDelete(row: KiroHistoryEntry) {
    dialog.warning({
      title: '删除账号',
      content: `确认删除 ${row.email}？删除后不可恢复，也不会再次从云端同步。`,
      positiveText: '删除',
      negativeText: '取消',
      onPositiveClick: async () => {
        operatingId.value = row.id
        try {
          await kiro.deleteHistoryAccount(row.id)
          message.success('账号已删除')
        } catch (error) {
          message.error(error instanceof Error ? error.message : String(error))
        } finally {
          operatingId.value = null
        }
      },
    })
  }

  const columns: DataTableColumns<KiroHistoryEntry> = [
    { title: '账号', key: 'email', minWidth: 210 },
    {
      title: '认证方式',
      key: 'provider',
      width: 120,
      render: (row) => h(NTag, { size: 'small' }, { default: () => row.provider }),
    },
    {
      title: '绑定 MachineGuid',
      key: 'machineId',
      minWidth: 260,
      render: (row) => h('code', { class: 'machine-id' }, row.machineId),
    },
    {
      title: '账号额度',
      key: 'creditQuota',
      width: 110,
    },
    {
      title: '切换时间',
      key: 'switchedAt',
      width: 190,
      render: (row) => new Date(row.switchedAt).toLocaleString(),
    },
    {
      title: '状态',
      key: 'status',
      width: 100,
      render: (row) =>
        row.id === kiro.currentAccount?.account?.id
          ? h(NTag, { type: 'success', size: 'small' }, { default: () => '当前' })
          : '',
    },
    {
      title: '操作',
      key: 'actions',
      width: 110,
      fixed: 'right',
      render: (row) =>
        h(
          NSpace,
          { size: 6, wrap: false },
          {
            default: () => [
              h(
                NButton,
                {
                  quaternary: true,
                  circle: true,
                  title: '切换到此账号',
                  loading: operatingId.value === row.id,
                  disabled: row.id === kiro.currentAccount?.account?.id,
                  onClick: () => switchAccount(row),
                },
                { icon: () => h(NIcon, null, { default: () => h(SwapHorizontalOutline) }) },
              ),
              h(
                NButton,
                {
                  quaternary: true,
                  circle: true,
                  type: 'error',
                  title: '删除账号',
                  disabled: operatingId.value !== null,
                  onClick: () => confirmDelete(row),
                },
                { icon: () => h(NIcon, null, { default: () => h(TrashOutline) }) },
              ),
            ],
          },
        ),
    },
  ]

  onMounted(() => kiro.refresh().catch(() => undefined))
</script>

<template>
  <n-space
    vertical
    :size="24"
  >
    <n-card title="Kiro 账号历史">
      <n-data-table
        :columns="columns"
        :data="kiro.history"
        :loading="kiro.loading"
        :bordered="false"
        :pagination="{ pageSize: 10 }"
        :scroll-x="1010"
      />
    </n-card>
  </n-space>
</template>

<style scoped>
  :deep(.machine-id) {
    font-family: 'Fira Code', monospace;
    font-size: 12px;
    letter-spacing: 0;
  }
</style>
