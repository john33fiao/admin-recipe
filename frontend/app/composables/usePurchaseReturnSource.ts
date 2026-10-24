import type { Ref } from 'vue'
import type { DraftPurchaseReturnV1, PurchaseReturnSource } from '~/types/purchase-return'
import type { HoldingDetail, ReceiptList } from '~/types/receipt'
import { apiRequest } from '~/utils/api'

export function usePurchaseReturnSource(data: Ref<DraftPurchaseReturnV1>, disabled: Ref<boolean>) {
  const connection = useConnection()
  const choices = ref<ReceiptList['items']>([]), source = ref<PurchaseReturnSource | null>(null), holding = ref<HoldingDetail | null>(null)
  const failure = ref(''), checking = ref(false)
  let active = true, generation = 0, choicesGeneration = 0
  onScopeDispose(() => { active = false; generation += 1; choicesGeneration += 1 })
  async function loadChoices() {
    const current = ++choicesGeneration
    if (connection.value !== 'connected') return
    try { const value = await apiRequest<ReceiptList>('/api/receipts'); if (active && current === choicesGeneration) choices.value = value.items }
    catch { if (active && current === choicesGeneration) failure.value = '원입고 목록을 불러오지 못했습니다. 입력을 유지했습니다.' }
  }
  async function load(apply = false) {
    const id = data.value.receipt_id, fingerprint = JSON.stringify(data.value), current = ++generation
    if (!id || connection.value !== 'connected' || apply && disabled.value) { checking.value = false; return }
    checking.value = true; failure.value = ''
    try {
      const value = await apiRequest<PurchaseReturnSource>(`/api/receipts/${encodeURIComponent(id)}/return-source`)
      if (!active || current !== generation || data.value.receipt_id !== id) return
      source.value = value
      if (apply && !disabled.value && JSON.stringify(data.value) === fingerprint) {
        data.value.expected_receipt_version_id = value.current?.reference.kind === 'existing' ? value.current.reference.version_id : null
        if (value.current) data.value.unit = value.current.unit
      }
      holding.value = null
      if (value.current) {
        const result = await apiRequest<HoldingDetail>(`/api/holdings/${encodeURIComponent(value.current.material_id)}`)
        if (active && current === generation && data.value.receipt_id === id) holding.value = result
      }
    } catch { if (active && current === generation) failure.value = '원입고나 현재 보유 근거를 확인하지 못했습니다. 입력과 선택한 버전을 유지했습니다.' }
    finally { if (active && current === generation) checking.value = false }
  }
  async function select(id: string) {
    if (disabled.value || connection.value !== 'connected') return
    data.value.receipt_id = id || null; data.value.expected_receipt_version_id = null
    await nextTick()
    if (active && !disabled.value && data.value.receipt_id === (id || null)) await load(true)
  }
  watch([connection, () => data.value.receipt_id], () => { source.value = null; holding.value = null; void load() }, { immediate: true })
  watch(connection, () => { void loadChoices() }, { immediate: true })
  return { choices, source, holding, checking, failure, select, load, loadChoices }
}
