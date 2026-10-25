import type { BulkKind, BulkSession, BulkRow, BulkItem, ReceiptInputPreview, BulkContext } from '~/types/bulk'
import type { CatalogState, Material } from '~/types/material'
import type { BusinessDetail } from '~/types/business'
import { ApiError, apiRequest } from '~/utils/api'
import { bulkFingerprint, makeBulkRow, restoreBulkSession, reviewBulk, setBulkCell } from '~/utils/bulk'
import { bulkLimits, parseBulkCsv, parseBulkPaste } from '~/utils/bulk-csv'
import { runBulk, inspectBulkDraft, applyBulkDraft } from '~/utils/bulk-runner'
import { currentAppState } from '~/utils/current-state'

const key = 'sunchang.bulk.session.v1'
const retainedPrefix = 'sunchang.bulk.retained.'
function fresh(kind: BulkKind): BulkSession { return { version: 1, kind, rows: [makeBulkRow(kind)], review: null, fingerprint: '', acceptedWarnings: false, duplicate: false } }
export function useBulkEntry() {
  const connection = useConnection()
  const storageFailure = useState('sunchang.bulk.storage-failure', () => '')
  const corrupt = useState('sunchang.bulk.corrupt', () => false)
  const session = useState<BulkSession>('sunchang.bulk.session', () => {
    try {
      const raw = sessionStorage.getItem(key)
      const value = restoreBulkSession(raw)
      if (raw && !value) { corrupt.value = true; storageFailure.value = '보관한 입력을 안전하게 읽지 못했습니다. 원문을 보존했습니다. 보존 입력에서 확인하거나 새 입력을 시작해 주세요.' }
      return value ?? fresh('material')
    } catch { storageFailure.value = '탭 저장소를 읽지 못했습니다. 입력을 닫기 전에 저장공간을 확인해 주세요.'; return fresh('material') }
  })
  const materials = ref<Material[]>([]), busy = useState('sunchang.bulk.busy', () => false)
  const failure = ref(''), notice = ref(''), loaded = ref(false)
  const draftUpdates = ref<Record<string, BusinessDetail>>({})
  const unresolved = computed(() => session.value.review?.items.some(item => !!item.pending && ['retry', 'blocked', ''].includes(item.recovery)) ?? false)
  const changedEpoch = computed(() => !!session.value.review && !!currentAppState.value && session.value.review.state.epoch !== currentAppState.value.epoch)
  const hasResults = computed(() => session.value.review?.items.some(item => !!item.result) ?? false)
  const ready = computed(() => !busy.value && !corrupt.value && !session.value.duplicate && !changedEpoch.value && !!session.value.review?.items.length && !session.value.review.items.some(item => !item.result && ['blocked', 'review', 'effect'].includes(item.recovery)) && !Object.keys(session.value.review.errors).length && session.value.fingerprint === bulkFingerprint(session.value.kind, session.value.rows) && (!session.value.review.warnings.length || session.value.acceptedWarnings))
  function persist() {
    if (corrupt.value) throw new Error('원입력을 먼저 보존해야 합니다.')
    try { sessionStorage.setItem(key, JSON.stringify(session.value)); storageFailure.value = '' }
    catch (error) { storageFailure.value = '입력·요청을 탭에 보관하지 못했습니다. 저장공간을 확인해 주세요. 전송 전 보관에 실패하면 등록하지 않습니다.'; throw error }
  }
  function retain() {
    const raw = corrupt.value ? sessionStorage.getItem(key) : JSON.stringify(session.value)
    if (raw) sessionStorage.setItem(`${retainedPrefix}${crypto.randomUUID()}.v1`, raw)
  }
  function saveDraft() { try { persist() } catch { /* Displayed beside the editor. */ } }
  watch(() => session.value.rows, () => { session.value.fingerprint = ''; session.value.acceptedWarnings = false; notice.value = ''; saveDraft() }, { deep: true, flush: 'sync' })
  watch(() => session.value.acceptedWarnings, saveDraft)
  let generation = 0
  async function load() {
    const token = ++generation
    if (connection.value !== 'connected') return
    try { const value = await apiRequest<Material[]>('/api/materials?archive=all'); if (token === generation) { materials.value = value; loaded.value = true } }
    catch { if (token === generation) failure.value = '재료 목록을 불러오지 못했습니다. 입력을 유지했습니다.' }
  }
  watch(connection, () => { void load() }, { immediate: true })
  const completed = (row: BulkRow) => !!session.value.review?.items.find(item => item.rowIds.includes(row.id) && item.result)
  const rowLocked = (row: BulkRow) => busy.value || corrupt.value || completed(row) || unresolved.value || changedEpoch.value
  function editCell(row: BulkRow, field: string, value: string) {
    if (rowLocked(row)) return
    setBulkCell(session.value.kind, row, field, value)
  }
  function addRow() {
    if (busy.value || unresolved.value || changedEpoch.value || corrupt.value || session.value.rows.length >= bulkLimits.rows) return
    session.value.rows.push(makeBulkRow(session.value.kind))
  }
  function removeRow(row: BulkRow) { if (!rowLocked(row)) session.value.rows = session.value.rows.filter(value => value.id !== row.id) }
  function start(kind: BulkKind, values?: Record<string, string>[], allowDuplicate = false) {
    if (busy.value || unresolved.value && !changedEpoch.value && !session.value.review?.items.some(item => item.recovery === 'blocked')) return false
    try {
      const rows = values?.map(value => makeBulkRow(kind, value)) ?? [makeBulkRow(kind)]
      const fingerprint = bulkFingerprint(kind, rows)
      let prior: BulkSession | null = null
      if (!allowDuplicate && values) {
        const candidates = [session.value, ...Array.from({ length: sessionStorage.length }, (_, i) => sessionStorage.key(i)).filter((value): value is string => !!value?.startsWith(retainedPrefix)).map(value => restoreBulkSession(sessionStorage.getItem(value))).filter((value): value is BulkSession => !!value)]
        prior = candidates.find(value => value.kind === kind && bulkFingerprint(kind, value.rows) === fingerprint && value.review?.items.some(item => item.result || item.pending)) ?? null
      }
      retain(); corrupt.value = false
      session.value = prior ? JSON.parse(JSON.stringify(prior)) : { ...fresh(kind), rows }
      session.value.duplicate = !!prior
      draftUpdates.value = {}
      persist(); failure.value = ''; notice.value = prior ? '같은 내용의 이전 입력을 찾았습니다. 결과와 미확인 요청을 이어서 확인하세요. 새 요청으로 중복 등록하지 않았습니다.' : '확인 전에는 재료·메뉴·입고를 등록하지 않습니다.'
      return true
    } catch { storageFailure.value = '이전 입력 보존을 완료하지 못해 새 입력으로 바꾸지 않았습니다.'; return false }
  }
  async function upload(file: File) {
    if (busy.value || unresolved.value || changedEpoch.value) return
    try {
      if (file.size > bulkLimits.bytes) throw new Error('CSV 파일은 1MiB 이내로 준비해 주세요.')
      if (!/\.csv$/i.test(file.name)) throw new Error('UTF-8 CSV 파일을 선택해 주세요. Excel에서는 CSV로 저장하세요.')
      const kind = session.value.kind
      const raw = new TextDecoder('utf-8', { fatal: true }).decode(await file.arrayBuffer())
      if (kind !== session.value.kind || busy.value || unresolved.value) return
      start(kind, parseBulkCsv(kind, raw))
    } catch (error) { failure.value = error instanceof Error ? error.message : 'CSV 파일을 읽지 못했습니다. 기존 입력을 유지했습니다.' }
  }
  function paste(raw: string) {
    if (busy.value || unresolved.value || changedEpoch.value) return
    try { start(session.value.kind, parseBulkPaste(session.value.kind, raw)) }
    catch (error) { failure.value = error instanceof Error ? error.message : '붙여넣을 내용을 확인해 주세요.' }
  }
  async function prepare() {
    if (busy.value || corrupt.value || unresolved.value || session.value.duplicate || connection.value !== 'connected') return false
    busy.value = true; failure.value = ''; notice.value = ''
    try {
      const state = await apiRequest<CatalogState>('/api/state')
      const choices = await apiRequest<Material[]>('/api/materials?archive=all')
      const old = session.value.review
      if (old && old.state.epoch !== state.epoch && old.items.some(item => item.draft || item.result || item.pending)) { failure.value = '복원 전 실행 결과와 원요청을 보존했습니다. 보존 입력과 현재 자료를 확인한 뒤 빈 새 입력에서 시작해 주세요.'; return false }
      if (old?.items.some(item => item.pending || item.draft || item.result)) retain()
      const saved = old?.items.filter(item => item.result) ?? []
      const savedRows = new Set(saved.flatMap(item => item.rowIds))
      const context: BulkContext = { state, materials: choices }
      const review = reviewBulk(session.value.kind, session.value.rows.filter(row => !savedRows.has(row.id)), context)
      const updates: Record<string, BusinessDetail> = {}
      for (const entry of review.items) {
        const previous = old?.items.find(item => item.id === entry.id && JSON.stringify(item.command) === JSON.stringify(entry.command))
        if (previous?.draft) {
          entry.draft = { ...previous.draft }
          const detail = await apiRequest<BusinessDetail>(`/api/business/${previous.draft.id}`)
          const outcome = inspectBulkDraft(entry, detail)
          if (outcome === 'confirmed') updates[entry.id] = detail
          if (outcome === 'changed' || outcome === 'archived') {
            updates[entry.id] = detail
            for (const rowId of entry.rowIds) (review.errors[rowId] ??= {}).stock_effect = entry.failure
          }
        }
      }
      review.items = [...saved, ...review.items]
      materials.value = choices
      if (!Object.keys(review.errors).length) {
        for (const entry of review.items) if (!entry.result && entry.command.kind === 'receipt') {
          const preview = await apiRequest<ReceiptInputPreview>('/api/receipts/preview', { state, data: entry.command.payload.data })
          entry.effect = preview.effect
          for (const issue of preview.issues) for (const rowId of entry.rowIds) (review.errors[rowId] ??= {})[issue.field.startsWith('stock_effect') ? 'stock_effect' : issue.field.startsWith('source') ? 'material_id' : issue.field] = issue.message
        }
      }
      draftUpdates.value = updates
      session.value.review = review
      session.value.fingerprint = bulkFingerprint(session.value.kind, session.value.rows)
      session.value.acceptedWarnings = false
      persist()
      notice.value = Object.keys(review.errors).length ? '오류를 수정한 뒤 등록 내용을 다시 확인해 주세요. 등록 요청은 보내지 않았습니다.' : '아래 내용과 참조를 확인한 뒤 등록해 주세요.'
      return !Object.keys(review.errors).length
    } catch (error) { failure.value = error instanceof ApiError ? error.message : '등록 내용을 확인하지 못했습니다. 입력을 유지했습니다.'; return false }
    finally { busy.value = false }
  }
  async function run() {
    if (!ready.value || connection.value !== 'connected') return false
    busy.value = true; failure.value = ''; notice.value = ''
    try {
      const success = await runBulk(session.value.review!, apiRequest, persist)
      notice.value = success ? '일괄 등록을 마쳤습니다. 아래 결과에서 실제 저장한 기록을 확인하세요.' : '등록이 멈췄습니다. 성공한 행은 유지하며 다시 보내지 않습니다. 표시된 원인과 요청 상태를 확인해 주세요.'
      if (success) await load()
      return success
    } finally { busy.value = false }
  }
  function acceptEffect(entry: BulkItem) {
    if (busy.value || entry.recovery !== 'effect' || !entry.preview?.can_confirm || entry.pending) return
    entry.effect = JSON.parse(JSON.stringify(entry.preview.effect)); entry.recovery = ''; entry.failure = ''; saveDraft()
    notice.value = '새 입고 효과를 확인했습니다. 이어서 등록하면 성공한 행을 생략합니다.'
  }
  function cancelReview() { if (!busy.value && !unresolved.value) { session.value.fingerprint = ''; session.value.acceptedWarnings = false; saveDraft(); notice.value = '확인을 취소했습니다. 작성한 값은 표에 남아 있습니다.' } }
  function separateDuplicate() {
    if (busy.value || unresolved.value) return
    start(session.value.kind, session.value.rows.map(row => ({ ...row.values })), true)
    notice.value = '같은 내용을 별도 새 입력으로 선택했습니다. 참조와 중복 여부를 다시 확인하고 등록해 주세요.'
  }
  function resumeDuplicate() { session.value.duplicate = false; saveDraft(); notice.value = '이전 입력의 처리 상태를 이어서 확인합니다. 성공한 행은 다시 보내지 않습니다.' }
  function applyCurrentDraft(entry: BulkItem) {
    const detail = draftUpdates.value[entry.id], row = session.value.rows.find(value => value.id === entry.rowIds[0])
    if (busy.value || entry.result || entry.pending || !session.value.review?.items.includes(entry) || !detail || !row || completed(row)) return
    if (detail.head.payload.kind === 'receipt' && detail.head.payload.data.source.kind === 'order') { notice.value = '발주에 연결된 입고 초안은 이 표에 적용할 수 없습니다. 초안 화면에서 계속해 주세요.'; return }
    try {
      if (!applyBulkDraft(entry, row, detail, retain)) return
      delete draftUpdates.value[entry.id]; session.value.fingerprint = ''; persist(); notice.value = '현재 초안의 내용을 표에 적용했습니다. 다시 확인한 뒤 이어서 등록해 주세요.'
    } catch { storageFailure.value = '기존 입력 보존을 완료하지 못해 현재 초안 적용을 중단했습니다.' }
  }
  return { session, connection, materials, loaded, busy, failure, notice, storageFailure, corrupt, unresolved, changedEpoch, hasResults, ready, rowLocked, editCell, addRow, removeRow, start, upload, paste, prepare, run, acceptEffect, cancelReview, separateDuplicate, resumeDuplicate, draftUpdates, applyCurrentDraft, load, saveDraft }
}
