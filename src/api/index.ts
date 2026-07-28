import { invoke } from '@tauri-apps/api/core'
import type {
  ApiResponse,
  LoginResponse,
  UserInfo,
  PublicInfo,
  HistoryRecord,
  Article,
  CheckUserResponse,
  RegisterResponse,
  KiroStatus,
  KiroUsage,
  KiroSwitchOptions,
  KiroSwitchResult,
  KiroOwnedAccount,
} from './types'
import Logger from '../utils/logger'
import { apiClient } from '@/utils/apiClient'

// 错误处理
function handleApiResponse<T>(response: ApiResponse<T>): T {
  if (response.status === 200) {
    // 成功时返回 data
    if (response.data) {
      return response.data
    }
    // 如果没有data，返回空对象
    return {} as T
  }

  // 状态码不为200时抛出错误，优先使用服务器返回的消息
  throw new ApiError(response.msg || '链接服务器失败，请稍后再试')
}

// API 错误类
export class ApiError extends Error {
  public statusCode?: number

  constructor(message: string, statusCode?: number) {
    super(message)
    this.name = 'ApiError'
    this.statusCode = statusCode
  }
}

export async function getKiroStatus(): Promise<KiroStatus> {
  return invoke<KiroStatus>('get_kiro_status')
}

export async function getKiroUsage(): Promise<KiroUsage> {
  return invoke<KiroUsage>('get_kiro_usage')
}

export async function switchKiroAccount(options: KiroSwitchOptions): Promise<KiroSwitchResult> {
  return invoke<KiroSwitchResult>('switch_kiro_account', { options })
}

export async function listOwnedKiroAccounts(): Promise<KiroOwnedAccount[]> {
  return invoke<KiroOwnedAccount[]>('list_owned_kiro_accounts')
}

export async function deleteOwnedKiroAccount(accountId: number): Promise<void> {
  return invoke<void>('delete_owned_kiro_account', { accountId })
}

export async function switchOwnedKiroAccount(
  account: KiroOwnedAccount,
  options: KiroSwitchOptions,
): Promise<KiroSwitchResult> {
  return invoke<KiroSwitchResult>('switch_owned_kiro_account', { account, options })
}

export async function closeKiro(): Promise<void> {
  return invoke<void>('close_kiro')
}

export async function launchKiro(): Promise<void> {
  return invoke<void>('launch_kiro')
}

export async function setKiroPath(path: string): Promise<void> {
  return invoke<void>('set_kiro_path', { path })
}

export async function getMachineId(): Promise<string> {
  return invoke<string>('get_machine_id')
}

export async function restoreOriginalMachineId(): Promise<string> {
  return invoke<string>('restore_original_machine_id')
}

// 用户认证相关 API

/**
 * 检查用户登录状态
 * @param email 用户邮箱
 * @returns 用户登录状态信息
 */
export async function checkUser(email?: string): Promise<CheckUserResponse> {
  try {
    const response = await apiClient.request<ApiResponse<CheckUserResponse>>('check_user', {
      email: email || '',
    })
    // 直接返回原始响应中的字段，而不是通过handleApiResponse处理
    if (response.status === 200) {
      return {
        status: response.data?.isLoggedIn ? response.status : 404,
        msg: response.msg,
        isLoggedIn: response.data?.isLoggedIn || false,
        userInfo: response.data?.userInfo,
      }
    }
    return handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '无法验证用户状态')
  }
}

export async function sendCode(email: string, purpose: string): Promise<void> {
  try {
    const response = await apiClient.request<ApiResponse<void>>('send_code', {
      email,
      type: purpose,
    })
    return handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '发送验证码失败')
  }
}

export async function register(
  email: string,
  code: string,
  password: string,
  spread: string,
): Promise<RegisterResponse> {
  try {
    const response = await apiClient.request<ApiResponse<RegisterResponse>>('register', {
      email,
      code,
      password,
      spread,
    })
    return handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '注册失败')
  }
}

export async function login(
  account: string,
  password: string,
  spread: string,
): Promise<LoginResponse> {
  try {
    const response = await apiClient.request<ApiResponse<LoginResponse>>('login', {
      account,
      password,
      spread,
    })
    return handleApiResponse(response)
  } catch (error) {
    await Logger.error('登录失败', { file: 'api/index.ts' })
    throw new ApiError(error instanceof Error ? error.message : '登录失败')
  }
}

// 用户信息相关 API
export async function getUserInfo(): Promise<UserInfo> {
  try {
    const response = await apiClient.request<ApiResponse<UserInfo>>('get_user_info')
    return handleApiResponse(response)
  } catch (error) {
    if (error instanceof ApiError) {
      throw error
    }
    throw new ApiError(error instanceof Error ? error.message : '链接服务器失败')
  }
}

// 系统信息相关 API
export async function getPublicInfo(): Promise<PublicInfo> {
  try {
    const response = await apiClient.request<ApiResponse<PublicInfo>>('get_public_info')
    return handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '获取公共信息失败')
  }
}

export async function refreshInbound(): Promise<boolean> {
  try {
    return await invoke<boolean>('refresh_inbound')
  } catch (error) {
    await Logger.error('刷新线路失败', { file: 'api/index.ts' })
    throw new ApiError(error instanceof Error ? error.message : '刷新线路失败')
  }
}

// 账户管理相关 API
export async function activate(code: string): Promise<void> {
  try {
    const response = await apiClient.request<ApiResponse<void>>('activate', { code })
    handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '激活失败')
  }
}

export async function changePassword(oldPassword: string, newPassword: string): Promise<void> {
  try {
    const response = await apiClient.request<ApiResponse<void>>('change_password', {
      oldPassword,
      newPassword,
    })
    handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '修改密码失败')
  }
}

// 管理员权限相关 API
export async function checkAdminPrivileges(): Promise<boolean> {
  try {
    return await invoke<boolean>('check_admin_privileges')
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '检查管理员权限失败')
  }
}

export async function resetPassword(email: string, code: string, password: string): Promise<void> {
  try {
    const response = await apiClient.request<ApiResponse<void>>('reset_password', {
      email,
      code,
      password,
    })
    return handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '重置密码失败')
  }
}

// 登出
export async function logout(): Promise<void> {
  try {
    const response = await apiClient.request<ApiResponse<void>>('logout')
    return handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '登出失败')
  }
}

// 使用键值存储实现历史记录功能

/**
 * 保存历史记录
 * @param record 历史记录
 */
export async function saveHistoryRecord(record: HistoryRecord): Promise<void> {
  try {
    const records = await getHistoryRecords()

    records.push(record)

    await setUserData('user.history', JSON.stringify(records))
  } catch (error) {
    Logger.error(`保存历史记录失败: ${error}`)
    throw new ApiError(error instanceof Error ? error.message : '保存历史记录失败')
  }
}

/**
 * 批量保存历史记录
 * @param records 历史记录数组
 */
export async function saveHistoryRecords(records: HistoryRecord[]): Promise<void> {
  try {
    let existingRecords = await getHistoryRecords()

    existingRecords = [...existingRecords, ...records]

    await setUserData('user.history', JSON.stringify(existingRecords))
  } catch (error) {
    Logger.error(`批量保存历史记录失败: ${error}`)
    throw new ApiError(error instanceof Error ? error.message : '保存历史记录失败')
  }
}

/**
 * 获取所有历史记录
 * @returns 历史记录数组
 */
export async function getHistoryRecords(): Promise<HistoryRecord[]> {
  try {
    const data = await getUserData('user.history')
    if (!data) {
      return []
    }

    try {
      return JSON.parse(data) as HistoryRecord[]
    } catch (e) {
      Logger.error(`历史记录解析失败: ${e}`)
      return []
    }
  } catch (error) {
    Logger.error(`获取历史记录失败: ${error}`)
    throw new ApiError(error instanceof Error ? error.message : '获取历史记录失败')
  }
}

/**
 * 清除所有历史记录
 */
export async function clearHistoryRecords(): Promise<void> {
  try {
    await delUserData('user.history')
  } catch (error) {
    Logger.error(`清除历史记录失败: ${error}`)
    throw new ApiError(error instanceof Error ? error.message : '清除历史记录失败')
  }
}

/**
 * 保存用户API Token
 * @param token API Token
 */
export async function saveUserApiToken(token: string): Promise<void> {
  try {
    await setUserData('user.info.token', token)
  } catch (error) {
    Logger.error(`保存API Token失败: ${error}`)
    throw new ApiError(error instanceof Error ? error.message : '保存API Token失败')
  }
}

/**
 * 获取用户API Token
 * @returns API Token，如果不存在则返回null
 */
export async function getUserApiToken(): Promise<string | null> {
  try {
    return await getUserData('user.info.token')
  } catch (error) {
    Logger.error(`获取API Token失败: ${error}`)
    throw new ApiError(error instanceof Error ? error.message : '获取API Token失败')
  }
}

/**
 * 清除用户API Token
 */
export async function clearUserApiToken(): Promise<void> {
  try {
    await delUserData('user.info.token')
  } catch (error) {
    Logger.error(`清除API Token失败: ${error}`)
    throw new ApiError(error instanceof Error ? error.message : '清除API Token失败')
  }
}

// 添加通用的键值存储 API 方法

/**
 * 设置用户数据
 * @param key 键名
 * @param value 值
 */
export async function setUserData(key: string, value: string): Promise<void> {
  try {
    await invoke<ApiResponse<any>>('set_user_data', { key, value })
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '设置用户数据失败')
  }
}

/**
 * 获取用户数据
 * @param key 键名
 * @returns 获取的值，如果不存在则返回 null
 */
export async function getUserData(key: string): Promise<string | null> {
  try {
    const response = await invoke<
      ApiResponse<{
        value: string | null
      }>
    >('get_user_data', { key })
    const result = handleApiResponse(response)
    return result.value
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '获取用户数据失败')
  }
}

/**
 * 删除用户数据
 * @param key 键名
 */
export async function delUserData(key: string): Promise<void> {
  try {
    await invoke<ApiResponse<any>>('del_user_data', { key })
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '删除用户数据失败')
  }
}

// 使用通用 API 实现的特定功能

/**
 * 检查用户是否已接受免责声明
 * @returns 是否已接受
 */
export async function checkDisclaimerAccepted(): Promise<boolean> {
  try {
    const value = await getUserData('user.disclaimer.accepted')
    return value === 'true'
  } catch (error) {
    Logger.error(`检查免责声明失败: ${error}`)
    return false
  }
}

/**
 * 设置用户已接受免责声明
 */
export async function setDisclaimerAccepted(): Promise<void> {
  try {
    await setUserData('user.disclaimer.accepted', 'true')
  } catch (error) {
    Logger.error(`设置免责声明状态失败: ${error}`)
    throw error
  }
}

/**
 * 清除用户的免责声明接受状态
 */
export async function clearDisclaimerAccepted(): Promise<void> {
  try {
    await delUserData('user.disclaimer.accepted')
  } catch (error) {
    Logger.error(`清除免责声明状态失败: ${error}`)
    throw error
  }
}

// 获取公告列表
export async function getArticleList(): Promise<Article[]> {
  try {
    const response = await apiClient.request<ApiResponse<Article[]>>('get_article_list')
    return handleApiResponse(response)
  } catch (error) {
    Logger.error(`获取公告列表失败: ${error}`)
    return []
  }
}

// 检查文章是否已读
export async function isArticleRead(articleId: number): Promise<boolean> {
  try {
    const valueStr = await getUserData('system.articles')
    if (!valueStr) return false

    try {
      const readIds = JSON.parse(valueStr) as number[]

      if (Array.isArray(readIds)) {
        const result = readIds.includes(articleId)
        return result
      } else {
        Logger.error('已读文章ID不是一个数组:', readIds)
        return false
      }
    } catch (parseError) {
      Logger.error(`解析已读文章ID失败: ${parseError}, 原始数据: ${valueStr}`)
      return false
    }
  } catch (error) {
    Logger.error(`获取已读文章状态失败: ${error}`)
    return false
  }
}

// 标记文章为已读
export async function markArticleRead(articleId: number): Promise<void> {
  try {
    const response = await apiClient.request<ApiResponse<void>>('mark_article_read', { articleId })
    handleApiResponse(response)
  } catch (error) {
    throw new ApiError(error instanceof Error ? error.message : '标记文章已读失败')
  }
}

/**
 * 打开开发者工具
 */
export const openDevTools = () => {
  return invoke('open_devtools')
}
