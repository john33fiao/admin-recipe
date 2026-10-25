import type { CatalogState } from '../types/material.ts'
import { catalogState, currentAppState, currentStateGeneration, observeCurrentState } from './current-state.ts'

export class ApiError extends Error {
  status: number
  code?: string
  requestStatus?: 'already_processed' | 'not_processed'
  state?: CatalogState
  constructor(status: number, message: string, code?: string, detail?: { request_status?: 'already_processed' | 'not_processed'; state?: CatalogState }) {
    super(code === 'stale_generation' ? detail?.request_status === 'already_processed' ? '복원 전에 처리된 요청입니다. 현재 자료에 다시 적용하지 않습니다. 원래 입력을 보존했으니 현재 자료와 처리 이력을 확인해 주세요.' : '복원으로 자료가 바뀌어 이 요청은 처리하지 않았습니다. 입력을 보존했으니 현재 자료를 확인하고 다시 검토해 주세요.' : message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
    this.requestStatus = detail?.request_status
    this.state = detail?.state
  }
}

export async function apiRequest<T>(path: string, body?: unknown): Promise<T> {
  const started = currentStateGeneration()
  const initialEpoch = currentAppState.value?.epoch
  const multipart = typeof FormData !== 'undefined' && body instanceof FormData
  const raw = path === '/api/restore-targets/import' && typeof Blob !== 'undefined' && body instanceof Blob
  const read = body === undefined || path.endsWith('/preview')
  const response = await fetch(path, {
    method: body === undefined ? 'GET' : 'POST',
    credentials: 'same-origin',
    cache: 'no-store',
    headers: body === undefined || multipart ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : multipart || raw ? body as BodyInit : JSON.stringify(body),
    signal: AbortSignal.timeout(multipart ? 90_000 : raw ? 65_000 : 15_000),
  })
  const data = await response.json().catch(() => null)
  if (!response.ok) {
    const state = catalogState(data?.state)
    if (data?.code === 'stale_generation' && state) observeCurrentState(state, started)
    throw new ApiError(response.status, typeof data?.error === 'string' ? data.error : '요청을 처리하지 못했습니다.', typeof data?.code === 'string' ? data.code : undefined, { request_status: data?.request_status, state: state ?? undefined })
  }
  if (data === null && response.status !== 204) throw new Error('invalid-response')
  const state = catalogState(data?.state) ?? catalogState(data)
  const generationChanged = currentAppState.value && (initialEpoch ? initialEpoch !== currentAppState.value.epoch : currentStateGeneration() > started + 1)
  if (!read && !state && path !== '/api/session' && path !== '/api/restore-targets/import' && generationChanged) throw new ApiError(409, '', 'stale_generation', { request_status: 'already_processed', state: currentAppState.value! })
  if (read && (state ? !observeCurrentState(state, started) : generationChanged)) throw new ApiError(409, '조회하는 동안 자료가 복원되었습니다. 현재 자료를 다시 조회해 주세요.', 'stale_read')
  if (state && !read) {
    if (path === '/api/restore-targets/import') observeCurrentState(state, started)
    else if (path === '/api/restores' ? !observeCurrentState(state, started) : currentAppState.value && currentAppState.value.epoch !== state.epoch) {
      throw new ApiError(409, '', 'stale_generation', { request_status: 'already_processed', state: currentAppState.value! })
    } else if (!currentAppState.value || currentAppState.value.epoch === state.epoch) observeCurrentState(state, started)
  }
  return data as T
}
