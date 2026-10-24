import { apiRequest } from '~/utils/api'

export default defineNuxtPlugin(() => {
  const connection = useConnection()
  const fragment = new URLSearchParams(window.location.hash.slice(1))
  let token: string | null = fragment.get('token')
  let socket: WebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | undefined
  let handshakeTimer: ReturnType<typeof setTimeout> | undefined
  let attempts = 0
  let busy = false
  let stopped = false

  function scheduleRetry() {
    if (stopped || reconnectTimer) return
    attempts += 1
    connection.value = attempts >= 3 ? 'unavailable' : 'reconnecting'
    reconnectTimer = setTimeout(() => {
      reconnectTimer = undefined
      void connect()
    }, Math.min(500 * 2 ** Math.min(attempts - 1, 4), 5_000))
  }

  async function connect() {
    if (busy || stopped || socket?.readyState === WebSocket.OPEN || socket?.readyState === WebSocket.CONNECTING) return
    busy = true
    try {
      if (token) {
        await apiRequest('/api/session', { token })
        // Keep the fragment through failures or refresh until the cookie is issued.
        // Fragments are not included in HTTP requests.
        window.history.replaceState(window.history.state, '', window.location.pathname + window.location.search)
        token = null
      }
      if (stopped) return
      const url = new URL('/api/heartbeat', window.location.href)
      url.protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
      const current = new WebSocket(url)
      socket = current
      // Protocol Ping/Pong is handled by the browser, not a JS timer.
      handshakeTimer = setTimeout(() => current.close(), 10_000)
      current.onopen = () => {
        if (socket !== current) return
        clearTimeout(handshakeTimer)
        attempts = 0
        connection.value = 'connected'
      }
      current.onclose = () => {
        if (socket !== current) return
        clearTimeout(handshakeTimer)
        socket = null
        scheduleRetry()
      }
      current.onerror = () => current.close()
    } catch {
      scheduleRetry()
    } finally {
      busy = false
    }
  }

  function retry(force = false) {
    if (stopped || busy) return
    if (force && socket) {
      socket.onopen = null
      socket.onclose = null
      socket.onerror = null
      socket.close()
      socket = null
      clearTimeout(handshakeTimer)
      connection.value = 'reconnecting'
    }
    if (socket?.readyState === WebSocket.OPEN) return
    clearTimeout(reconnectTimer)
    reconnectTimer = undefined
    void connect()
  }
  function resume() {
    // An OPEN socket can be stale after sleep; establish a fresh one before writes.
    if (document.visibilityState === 'visible') retry(true)
  }
  function pause() {
    stopped = true
    clearTimeout(reconnectTimer)
    clearTimeout(handshakeTimer)
    socket?.close()
    socket = null
  }
  function restore() {
    if (!stopped) return
    stopped = false
    connection.value = 'reconnecting'
    void connect()
  }
  window.addEventListener('focus', () => retry(true))
  window.addEventListener('online', () => retry(true))
  window.addEventListener('app:reconnect', () => retry(true))
  document.addEventListener('visibilitychange', resume)
  window.addEventListener('pagehide', pause)
  window.addEventListener('pageshow', restore)
  void connect()
})
