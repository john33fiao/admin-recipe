<script setup lang="ts">
import type { CorrectionSeed } from '~/types/correction'
import { correctionDraftIdentity } from '~/utils/correction-draft'
import { cloneCorrection } from '~/utils/correction'
import { blankStocktakeDraft, blankWasteDraft } from '~/utils/stocktake'
import type { BusinessDetail, BusinessList, BusinessPayload } from '~/types/business'
import type { SaleDetail, SaleOriginal } from '~/types/sale'
import type { RecipeDetail } from '~/types/recipe'
import { blankSaleDraft } from '~/utils/sale'
import type { MaterialDetail } from '~/types/material'
import type { OrderDetail } from '~/types/purchase-order'
import { apiRequest } from '~/utils/api'
import { blankPriceDraft, businessStatuses, copyPayload, restoreBusinessPending } from '~/utils/business'
import { formatCreatedAt } from '~/utils/material'
import { blankOrderDraft, orderActionDraft, orderDraftIdentity } from '~/utils/purchase-order'
import { blankProductionDraft } from '~/utils/production'
import { blankReceiptDraft } from '~/utils/receipt'

const route = useRoute()
const connection = useConnection()
const filter = ref('draft')
const kind = ref('')
const list = ref<BusinessList | null>(null)
const loading = ref(false)
const failure = ref('')
const seed = ref<BusinessPayload | null>(null)
const seedFailure = ref('')
const correctionSeed = ref<CorrectionSeed | null>(null)
const frozenSale = ref<SaleOriginal | null>(null)
const corrects = computed(() => typeof route.query.corrects === 'string' ? route.query.corrects : null)
const identity = ref('new.price')
let listGeneration = 0
let seedGeneration = 0
async function load(): Promise<null> {
  const generation = ++listGeneration
  if (connection.value !== 'connected') return null
  loading.value = true
  failure.value = ''
  try { const result = await apiRequest<BusinessList>(`/api/business?status=${filter.value}${kind.value ? `&kind=${kind.value}` : ''}`); if (generation === listGeneration) list.value = result }
  catch { if (generation === listGeneration) failure.value = '업무 목록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (generation === listGeneration) loading.value = false }
  return null
}
async function loadSeed() {
  const generation = ++seedGeneration
  if (connection.value !== 'connected') return
  seed.value = null
  frozenSale.value = null
  correctionSeed.value = null
  seedFailure.value = ''
  try {
    let payload: BusinessPayload = blankPriceDraft()
    let nextIdentity = `new.${corrects.value ?? (typeof route.query.material === 'string' ? route.query.material : 'price')}`
    let nextFrozen: SaleOriginal | null = null
    let nextCorrection: CorrectionSeed | null = null
    if (route.query.kind === 'correction') {
      if (!corrects.value) throw new Error('missing-target')
      nextCorrection = await apiRequest<CorrectionSeed>(`/api/business/confirmations/${encodeURIComponent(corrects.value)}/correction-seed`)
      payload = { kind: 'correction', version: 1, data: cloneCorrection(nextCorrection.draft) }
      let entries: Array<[string, string]> = []
      try { entries = Object.entries(sessionStorage) } catch { /* The editor handles unavailable persistence. */ }
      nextIdentity = correctionDraftIdentity(nextCorrection, entries)
    } else if (route.query.kind === 'sale') {
      const sale = blankSaleDraft()
      if (corrects.value) {
        if (typeof route.query.record !== 'string') throw new Error('missing-record')
        const detail = await apiRequest<BusinessDetail>(`/api/business/${encodeURIComponent(route.query.record)}`)
        let replay: ReturnType<typeof restoreBusinessPending> = null
        try { replay = restoreBusinessPending(sessionStorage.getItem(`sunchang.business.new.sale.${corrects.value}.pending.v1`)) } catch { /* The editor reports unavailable local storage. */ }
        const recoveringCreate = replay?.action === 'create' && replay.body.payload.kind === 'sale' && replay.body.corrects_confirmation_id === corrects.value
        // A previously sent create can already exist even when another correction won.
        // Restore its immutable request; only a new, unsent correction needs a live target.
        if (detail.head.payload.kind !== 'sale' || detail.item.confirmation_id !== corrects.value || (detail.item.effective_confirmation_id !== corrects.value && !recoveringCreate)) throw new Error('not-effective')
        const result = detail.confirmations.find(value => value.id === corrects.value)?.result
        if (result?.kind !== 'sale') throw new Error('missing-sale')
        nextFrozen = (await apiRequest<SaleDetail>(`/api/sales/${result.sale_id}`)).item
        Object.assign(sale.data, detail.head.payload.data, { note: '' })
      } else if (typeof route.query.material === 'string') {
        const recipe = await apiRequest<RecipeDetail>(`/api/recipes/${encodeURIComponent(route.query.material)}`)
        Object.assign(sale.data, { material_id: recipe.item.id, material_version_id: recipe.item.version_id, recipe_version_id: recipe.head.id })
      }
      payload = sale
      nextIdentity = `new.sale.${corrects.value ?? (typeof route.query.material === 'string' ? route.query.material : 'blank')}`
    } else if (route.query.kind === 'stocktake' || route.query.kind === 'waste') {
      const stock = route.query.kind === 'stocktake' ? blankStocktakeDraft() : blankWasteDraft()
      if (typeof route.query.material === 'string') {
        const material = await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(route.query.material)}`)
        const row = stock.kind === 'stocktake' ? stock.data.rows[0]! : stock.data
        Object.assign(row, { material_id: material.id, material_version_id: material.version_id, unit: material.unit })
      }
      payload = stock
      nextIdentity = `new.${stock.kind}.${typeof route.query.material === 'string' ? route.query.material : 'blank'}`
    } else if (route.query.kind === 'production') {
      const production = blankProductionDraft()
      if (typeof route.query.material === 'string') {
        const material = await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(route.query.material)}`)
        production.data.inputs[0] = { material_id: material.id, material_version_id: material.version_id, quantity: '', unit: material.unit }
      }
      payload = production
      nextIdentity = `new.production.${typeof route.query.material === 'string' ? route.query.material : 'blank'}`
    } else if (route.query.kind === 'receipt') {
      const receipt = blankReceiptDraft()
      if (typeof route.query.order === 'string') {
        const order = await apiRequest<OrderDetail>(`/api/orders/${encodeURIComponent(route.query.order)}`)
        const material = await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(order.item.material_id)}`)
        receipt.data.source = { kind: 'order', order_id: order.item.id, expected_order_version_id: order.head.id, material_version_id: material.version_id }
        receipt.data.unit = order.head.input_unit
      } else if (typeof route.query.material === 'string') {
        const material = await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(route.query.material)}`)
        receipt.data.source = { kind: 'direct', material_id: material.id, material_version_id: material.version_id }
        receipt.data.unit = material.unit
      }
      payload = receipt
      nextIdentity = `new.receipt.${typeof route.query.order === 'string' ? `order.${route.query.order}` : typeof route.query.material === 'string' ? `material.${route.query.material}` : 'direct'}`
    } else if (route.query.kind === 'purchase_order') {
      if (route.query.action === 'revise' || route.query.action === 'cancel') {
        if (typeof route.query.order !== 'string') throw new Error('missing-order')
        const order = await apiRequest<OrderDetail>(`/api/orders/${encodeURIComponent(route.query.order)}`)
        payload = orderActionDraft(order, route.query.action)
      } else payload = blankOrderDraft()
      let entries: Array<[string, string]> = []
      try { entries = Object.entries(sessionStorage) } catch { /* The editor reports storage failures before writes. */ }
      nextIdentity = orderDraftIdentity(payload, entries)
    } else if (corrects.value) {
      if (typeof route.query.record !== 'string') throw new Error('missing-record')
      const detail = await apiRequest<BusinessDetail>(`/api/business/${encodeURIComponent(route.query.record)}`)
      if (detail.head.payload.kind !== 'catalog_price' || detail.item.confirmation_id !== corrects.value || detail.item.effective_confirmation_id !== corrects.value) throw new Error('not-effective')
      payload = copyPayload(detail.head.payload)
      payload.data.note = ''
    } else if (typeof route.query.material === 'string') {
      const material = await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(route.query.material)}`)
      payload.data = { ...payload.data, material_id: material.id, material_version_id: material.version_id, unit: material.unit }
    }
    if (generation === seedGeneration) { identity.value = nextIdentity; seed.value = payload; frozenSale.value = nextFrozen; correctionSeed.value = nextCorrection }
  } catch { if (generation === seedGeneration) seedFailure.value = '초안의 대상 기록을 확인하지 못했습니다. 최신 재료나 유효한 확정 기록에서 다시 열어 주세요.' }
}
watch([connection, filter, kind], () => { void load() }, { immediate: true })
watch([connection, () => route.query.material, () => route.query.corrects, () => route.query.record, () => route.query.kind, () => route.query.action, () => route.query.order], () => { void loadSeed() }, { immediate: true })
useHead({ title: '업무 기록 | 순창한상' })
</script>

<template>
  <div class="page-heading"><div><h1>업무 기록</h1><p>작성 중인 가격·발주·입고·생산·판매·실사·폐기를 임시 저장하고, 확인한 내용을 확정하세요.</p></div><div class="button-row"><NuxtLink class="secondary-link" to="/business">가격 초안 작성</NuxtLink><NuxtLink class="secondary-link" to="/business?kind=purchase_order&amp;action=place">주문 초안 작성</NuxtLink><NuxtLink class="secondary-link" to="/business?kind=receipt">직접 입고 초안</NuxtLink><NuxtLink class="secondary-link" to="/business?kind=production">생산 초안 작성</NuxtLink><NuxtLink class="secondary-link" to="/business?kind=sale">판매 초안 작성</NuxtLink><NuxtLink class="secondary-link" to="/business?kind=stocktake">실사 초안 작성</NuxtLink><NuxtLink class="secondary-link" to="/business?kind=waste">폐기 · 직원식 초안</NuxtLink></div></div>
  <p v-if="seedFailure" class="notice warning" role="alert">{{ seedFailure }} <NuxtLink to="/business">새 가격 초안 작성</NuxtLink></p>
  <div class="business-workspace">
    <BusinessCorrectionEditor v-if="seed?.kind === 'correction' && correctionSeed" :key="identity" :identity="identity" :seed="correctionSeed" :initial="{ payload: seed, state: correctionSeed.state, recordId: null, headId: null, status: 'draft', corrects: correctionSeed.original.reference.original_confirmation_id, issues: [] }" :refresh="load" />
    <BusinessPriceEditor v-else-if="seed?.kind === 'catalog_price' && list" :key="identity" :identity="identity" :initial="{ payload: seed, state: list.state, recordId: null, headId: null, status: 'draft', corrects, issues: [] }" :refresh="load" />
    <BusinessOrderEditor v-else-if="seed?.kind === 'purchase_order' && list" :key="identity" :identity="identity" :initial="{ payload: seed, state: list.state, recordId: null, headId: null, status: 'draft', corrects: null, issues: [] }" :refresh="load" />
    <BusinessReceiptEditor v-else-if="seed?.kind === 'receipt' && list" :key="identity" :identity="identity" :initial="{ payload: seed, state: list.state, recordId: null, headId: null, status: 'draft', corrects: null, issues: [] }" :refresh="load" />
    <BusinessProductionEditor v-else-if="seed?.kind === 'production' && list" :key="identity" :identity="identity" :initial="{ payload: seed, state: list.state, recordId: null, headId: null, status: 'draft', corrects: null, issues: [] }" :refresh="load" />
    <BusinessSaleEditor v-else-if="seed?.kind === 'sale' && list" :key="identity" :identity="identity" :initial="{ payload: seed, state: list.state, recordId: null, headId: null, status: 'draft', corrects, issues: [] }" :frozen="frozenSale" :refresh="load" />
    <BusinessStockEditor v-else-if="(seed?.kind === 'stocktake' || seed?.kind === 'waste') && list" :key="identity" :identity="identity" :initial="{ payload: seed, state: list.state, recordId: null, headId: null, status: 'draft', corrects: null, issues: [] }" :refresh="load" />
    <section class="business-list-panel" aria-labelledby="business-list-title" :aria-busy="loading">
      <div class="list-heading"><h2 id="business-list-title">저장된 업무</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
      <div class="field"><label for="business-filter">업무 상태</label><select id="business-filter" v-model="filter" :disabled="connection !== 'connected'"><option value="draft">임시 저장</option><option value="confirmed">확정</option><option value="archived">보관</option><option value="all">전체</option></select></div>
      <div class="field"><label for="business-kind">업무 종류</label><select id="business-kind" v-model="kind" :disabled="connection !== 'connected'"><option value="">전체</option><option value="catalog_price">사전 가격</option><option value="purchase_order">발주 업무</option><option value="receipt">실제 입고</option><option value="production">실제 생산</option><option value="sale">판매</option><option value="stocktake">실사</option><option value="waste">폐기 · 직원식</option><option value="correction">확정 기록 정정</option></select></div>
      <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p>
      <div v-if="!list" class="empty-state"><h3>업무 기록을 불러오고 있습니다</h3><p>앱에 연결되면 저장된 초안을 확인할 수 있습니다.</p></div>
      <div v-else-if="!list.items.length" class="empty-state"><h3>이 상태의 업무가 없습니다</h3><p>초안을 임시 저장하면 여기에서 이어서 작성할 수 있습니다.</p></div>
      <ul v-else class="business-list"><li v-for="item in list.items" :key="item.id"><NuxtLink :to="`/business/${item.id}`"><div><strong>{{ item.material_name ?? '재료 확인 필요' }}</strong><span class="status-label">{{ businessStatuses[item.status] }}</span></div><p>{{ item.kind === 'correction' ? '확정 기록 정정' : item.kind === 'stocktake' ? '실사' : item.kind === 'waste' ? '폐기 · 직원식' : item.kind === 'sale' ? (item.corrects_confirmation_id ? '판매 개수 정정' : '판매') : item.kind === 'purchase_order' ? '발주 업무' : item.kind === 'receipt' ? '실제 입고' : item.kind === 'production' ? '실제 생산' : item.corrects_confirmation_id ? '가격 정정' : '사전 가격' }} · {{ item.id.slice(0, 8) }}</p><p v-if="item.issue_count" class="business-needs-input">입력 필요 {{ item.issue_count }}개</p><time>{{ formatCreatedAt(item.updated_at) }} 수정</time></NuxtLink></li></ul>
    </section>
  </div>
</template>
