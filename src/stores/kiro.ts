import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  getKiroStatus,
  getKiroUsage,
  getMachineId,
  getUserData,
  delUserData,
  setUserData,
  switchKiroAccount,
  switchOwnedKiroAccount,
  listOwnedKiroAccounts,
  deleteOwnedKiroAccount,
} from '@/api'
import type {
  KiroHistoryEntry,
  KiroOwnedAccount,
  KiroStatus,
  KiroSwitchOptions,
  KiroSwitchResult,
  KiroUsage,
} from '@/api/types'

const CURRENT_ACCOUNT_KEY = 'system.kiro.current_account'
const HISTORY_KEY = 'system.kiro.account_history'

export const useKiroStore = defineStore('kiro', () => {
  const status = ref<KiroStatus | null>(null)
  const usage = ref<KiroUsage | null>(null)
  const machineId = ref('')
  const currentAccount = ref<KiroSwitchResult | null>(null)
  const history = ref<KiroHistoryEntry[]>([])
  const loading = ref(false)
  const switching = ref(false)
  const error = ref('')

  const remainingCredits = computed(() =>
    usage.value ? Math.max(0, usage.value.limit - usage.value.current) : 0,
  )
  const usagePercent = computed(() =>
    usage.value?.limit ? Math.min(100, (usage.value.current / usage.value.limit) * 100) : 0,
  )

  async function loadLocalState() {
    const [accountJSON, historyJSON] = await Promise.all([
      getUserData(CURRENT_ACCOUNT_KEY),
      getUserData(HISTORY_KEY),
    ])
    currentAccount.value = parseJSON<KiroSwitchResult | null>(accountJSON, null)
    history.value = parseJSON<KiroHistoryEntry[]>(historyJSON, [])
  }

  async function refresh() {
    loading.value = true
    error.value = ''
    try {
      await loadLocalState()
      const [nextStatus, nextMachineId] = await Promise.all([getKiroStatus(), getMachineId()])
      status.value = nextStatus
      machineId.value = nextMachineId
      try {
        usage.value = await getKiroUsage()
      } catch {
        usage.value = null
      }
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught)
      throw caught
    } finally {
      loading.value = false
    }
  }

  async function switchAccount(options: KiroSwitchOptions) {
    switching.value = true
    error.value = ''
    try {
      const result = await switchKiroAccount(options)
      const entry: KiroHistoryEntry = { ...result.account, switchedAt: new Date().toISOString() }
      currentAccount.value = result
      machineId.value = result.machineId
      history.value = [entry, ...history.value.filter((item) => item.id !== result.account.id)]
      await Promise.all([
        setUserData(CURRENT_ACCOUNT_KEY, JSON.stringify(result)),
        setUserData(HISTORY_KEY, JSON.stringify(history.value)),
      ])
      await refresh()
      return result
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught)
      throw caught
    } finally {
      switching.value = false
    }
  }

  async function syncOwnedAccounts() {
    await loadLocalState()
    const accounts = await listOwnedKiroAccounts()
    const switchedAtByID = new Map(history.value.map((item) => [item.id, item.switchedAt]))
    history.value = accounts.map((account) => ({
      ...account,
      switchedAt: switchedAtByID.get(account.id) || new Date().toISOString(),
    }))
    const currentID = currentAccount.value?.account?.id
    if (currentID && !accounts.some((account) => account.id === currentID)) {
      currentAccount.value = null
      await delUserData(CURRENT_ACCOUNT_KEY)
    }
    await setUserData(HISTORY_KEY, JSON.stringify(history.value))
  }

  async function switchHistoryAccount(account: KiroOwnedAccount, options: KiroSwitchOptions) {
    switching.value = true
    try {
      const result = await switchOwnedKiroAccount(account, options)
      const entry: KiroHistoryEntry = { ...result.account, switchedAt: new Date().toISOString() }
      currentAccount.value = result
      machineId.value = result.machineId
      history.value = [entry, ...history.value.filter((item) => item.id !== account.id)]
      await Promise.all([
        setUserData(CURRENT_ACCOUNT_KEY, JSON.stringify(result)),
        setUserData(HISTORY_KEY, JSON.stringify(history.value)),
      ])
      await refresh()
      return result
    } finally {
      switching.value = false
    }
  }

  async function deleteHistoryAccount(accountId: number) {
    await deleteOwnedKiroAccount(accountId)
    history.value = history.value.filter((item) => item.id !== accountId)
    const writes: Promise<unknown>[] = [setUserData(HISTORY_KEY, JSON.stringify(history.value))]
    if (currentAccount.value?.account?.id === accountId) {
      currentAccount.value = null
      writes.push(delUserData(CURRENT_ACCOUNT_KEY))
    }
    await Promise.all(writes)
  }

  return {
    status,
    usage,
    machineId,
    currentAccount,
    history,
    loading,
    switching,
    error,
    remainingCredits,
    usagePercent,
    refresh,
    switchAccount,
    syncOwnedAccounts,
    switchHistoryAccount,
    deleteHistoryAccount,
  }
})

function parseJSON<T>(value: string | null, fallback: T): T {
  if (!value) return fallback
  try {
    return JSON.parse(value) as T
  } catch {
    return fallback
  }
}
