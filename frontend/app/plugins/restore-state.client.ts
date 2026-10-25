import { apiRequest } from '~/utils/api'

export default defineNuxtPlugin(() => {
  const connection = useConnection()
  let busy = false
  let again = false
  async function verify() {
    if (connection.value !== 'connected') return
    if (busy) { again = true; return }
    busy = true
    try { await apiRequest('/api/state') } catch { /* Keep input and let existing connection/read errors offer retry. */ }
    finally { busy = false; if (again) { again = false; void verify() } }
  }
  watch(connection, value => { if (value === 'connected') void verify() }, { immediate: true })
  window.addEventListener('storage', event => { if (event.key === 'sunchang.restored.v1') void verify() })
})
