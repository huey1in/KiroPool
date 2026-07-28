export interface ApiResponse<T> {
  status: number
  msg: string
  data?: T
  code?: string
}

export type KiroProvider = 'BuilderId' | 'Enterprise' | 'Github' | 'Google'

export interface KiroUsageBonus {
  code: string
  name: string
  current: number
  limit: number
  expiresAt?: string
}

export interface KiroUsage {
  current: number
  limit: number
  baseCurrent: number
  baseLimit: number
  trialCurrent: number
  trialLimit: number
  trialExpiresAt?: string
  bonuses: KiroUsageBonus[]
  subscriptionTitle: string
  subscriptionType: string
  nextResetAt?: string
  userEmail?: string
  userId?: string
}

export interface KiroStatus {
  running: boolean
  executablePath: string
  authenticated: boolean
  provider?: KiroProvider
  authMethod?: string
  expiresAt?: string
}

export interface KiroSwitchOptions {
  forceClose: boolean
  launchAfterSwitch: boolean
}

export interface KiroSwitchResult {
  email: string
  provider: KiroProvider
  machineId: string
  deductedCredits: number
  account: KiroOwnedAccount
  launchError?: string
  syncError?: string
}

export interface KiroOwnedAccount {
  id: number
  email: string
  provider: KiroProvider
  authMethod: 'IdC' | 'social'
  accessToken: string
  refreshToken: string
  expiresAt: number
  clientId?: string
  clientSecret?: string
  region: string
  startUrl?: string
  profileArn?: string
  machineId: string
  creditQuota: number
}

export interface KiroHistoryEntry extends KiroOwnedAccount {
  switchedAt: string
}

export interface UserInfo {
  totalCredits: number
  usedCredits: number
  creditBalance: number
  expireTime: string
  level: number
  isExpired: boolean
  username: string
  code_level?: string
  code_status?: number
}

export interface RegisterResponse {
  token: string
  expires_time: number
}

export interface LoginResponse {
  token?: string
  user_info?: UserInfo
}

export interface CheckUserResponse {
  status: number
  msg: string
  isLoggedIn: boolean
  userInfo?: UserInfo
}

export interface PublicInfo {
  type: string
  closeable: boolean
  props: { title: string; description: string }
  actions: Array<{ type: string; text: string; url: string }>
}

export interface HistoryRecord {
  id: number
  type_name: string
  detail: string
  timestamp: string
  operator: string
}

export interface Article {
  id: number
  title: string
  content: string
}
