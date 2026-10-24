export class ApiError extends Error {
  status: number
  code?: string
  constructor(status: number, message: string, code?: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }
}

export async function apiRequest<T>(path: string, body?: unknown): Promise<T> {
  const response = await fetch(path, {
    method: body === undefined ? 'GET' : 'POST',
    credentials: 'same-origin',
    cache: 'no-store',
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(15_000),
  })
  const data = await response.json().catch(() => null)
  if (!response.ok) {
    throw new ApiError(response.status, typeof data?.error === 'string' ? data.error : '요청을 처리하지 못했습니다.', typeof data?.code === 'string' ? data.code : undefined)
  }
  if (data === null && response.status !== 204) throw new Error('invalid-response')
  return data as T
}
