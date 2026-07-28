export interface OperationRecord {
  id: number
  type: string
  detail: string
  timestamp: string
  operator: string
}

export type HistoryRecords = OperationRecord[]
