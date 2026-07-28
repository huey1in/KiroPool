import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { HistoryRecords } from '@/types/history'
import { getHistoryList, syncLocalHistoryToBackend } from '@/utils/history'
import Logger from '@/utils/logger'

export const useHistoryStore = defineStore('history', () => {
  const records = ref<HistoryRecords>([])
  const isLoading = ref(false)
  const error = ref('')
  const initialized = ref(false)

  const sortedRecords = computed(() => [...records.value].sort((a, b) => b.id - a.id))

  function filterByType(type: string) {
    return sortedRecords.value.filter((record) => record.type === type)
  }

  async function loadHistoryRecords(shouldSync = true) {
    isLoading.value = true
    error.value = ''
    try {
      if (shouldSync) await syncLocalHistoryToBackend()
      records.value = await getHistoryList()
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : '加载历史记录失败'
      Logger.error(`加载历史记录失败: ${caught}`)
    } finally {
      isLoading.value = false
    }
  }

  function setupHistoryListener() {
    const handler = () => loadHistoryRecords()
    window.addEventListener('history_updated', handler)
    return () => window.removeEventListener('history_updated', handler)
  }

  async function init() {
    if (initialized.value) return
    await loadHistoryRecords()
    setupHistoryListener()
    initialized.value = true
  }

  return {
    records,
    isLoading,
    error,
    sortedRecords,
    filterByType,
    loadHistoryRecords,
    setupHistoryListener,
    init,
  }
})
