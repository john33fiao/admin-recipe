<script setup lang="ts">
import type { BulkKind, BulkRow } from '~/types/bulk'
import type { Material } from '~/types/material'
import { bulkColumns } from '~/utils/bulk-csv'
import { materialKinds } from '~/utils/catalog'
const props = defineProps<{ kind: BulkKind; rows: BulkRow[]; materials: Material[]; errors: Record<string, Record<string, string>>; locked: (row: BulkRow) => boolean }>()
const emit = defineEmits<{ edit: [row: BulkRow, field: string, value: string]; remove: [row: BulkRow]; inclusions: [row: BulkRow] }>()
const columns = computed(() => bulkColumns[props.kind])
const input = (row: BulkRow, field: string, event: Event) => emit('edit', row, field, (event.target as HTMLInputElement).value)
const issue = (row: BulkRow, field: string) => props.errors[row.id]?.[field]
const material = (row: BulkRow) => props.materials.find(item => item.id === row.values.material_id)
const fieldId = (row: BulkRow, field: string) => `bulk-${row.id}-${field}`
</script>
<template>
  <div class="bulk-table-scroll" role="region" aria-label="일괄 입력 편집표 · 가로로 스크롤할 수 있습니다" tabindex="0">
    <table class="bulk-table"><caption>등록 전 편집표 · {{ rows.length }}행</caption><thead><tr><th scope="col">행</th><th v-for="column in columns" :key="column.key" scope="col">{{ column.label }}<span v-if="column.required" class="required-label"> 필수</span></th><th scope="col">행 관리</th></tr></thead>
      <tbody><tr v-for="(row, index) in rows" :key="row.id" :data-bulk-row="index + 1"><th scope="row">{{ index + 1 }}</th><td v-for="column in columns" :key="column.key"><label class="sr-only" :for="fieldId(row, column.key)">{{ index + 1 }}행 {{ column.label }}</label>
          <select v-if="column.key === 'kind'" :id="fieldId(row, column.key)" :value="row.values[column.key]" :disabled="locked(row)" :aria-invalid="!!issue(row, column.key)" :aria-describedby="`${fieldId(row, column.key)}-error`" @change="input(row, column.key, $event)"><option value="">종류 선택</option><option v-for="(label, value) in materialKinds" :key="value" :value="value" :disabled="value === 'unclassified'">{{ label }}</option></select>
          <select v-else-if="column.key === 'material_id'" :id="fieldId(row, column.key)" :value="row.values[column.key]" :disabled="locked(row)" :aria-invalid="!!issue(row, column.key)" :aria-describedby="`${fieldId(row, column.key)}-error ${fieldId(row, column.key)}-ref`" @change="input(row, column.key, $event)"><option value="">{{ kind === 'menu' ? '구성 없음' : '재료 선택' }}</option><option v-if="row.values.material_id && !material(row)" :value="row.values.material_id">선택 재확인 · {{ row.values.material_id }}</option><option v-for="choice in materials" :key="choice.id" :value="choice.id" :disabled="choice.archived">{{ choice.name }} · {{ choice.unit }} · {{ choice.id.slice(0, 8) }}{{ choice.archived ? ' (보관됨)' : '' }}</option></select>
          <select v-else-if="column.key === 'stock_effect'" :id="fieldId(row, column.key)" :value="row.values[column.key]" :disabled="locked(row)" :aria-invalid="!!issue(row, column.key)" :aria-describedby="`${fieldId(row, column.key)}-error`" @change="input(row, column.key, $event)"><option value="">반영 여부 확인</option><option value="unreflected">아직 재고에 미반영</option><option value="reconciled">실사 포함량 확인</option><option value="needs_reconciliation">포함 여부 확인 필요</option></select>
          <textarea v-else-if="['description', 'note'].includes(column.key)" :id="fieldId(row, column.key)" :value="row.values[column.key]" rows="2" :readonly="locked(row)" :aria-invalid="!!issue(row, column.key)" :aria-describedby="`${fieldId(row, column.key)}-error`" @input="input(row, column.key, $event)" />
          <input v-else :id="fieldId(row, column.key)" :value="row.values[column.key]" :inputmode="['quantity','amount','selling_price','basis_count','included_quantity'].includes(column.key) ? 'decimal' : 'text'" :readonly="locked(row)" :aria-invalid="!!issue(row, column.key)" :aria-describedby="`${fieldId(row, column.key)}-error`" @input="input(row, column.key, $event)">
          <p v-if="column.key === 'material_id'" :id="`${fieldId(row, column.key)}-ref`" class="field-hint bulk-reference">{{ row.reference ? `확인한 버전 ${row.reference.material_version_id}` : '확인할 때 선택한 ID·버전을 고정합니다.' }}</p>
          <button v-if="column.key === 'material_id' && row.values.material_id" class="text-button" type="button" :disabled="locked(row)" :aria-label="`${index + 1}행 재료 현재 버전 적용`" @click="emit('edit', row, 'material_id', row.values.material_id!)">현재 버전 다시 확인</button>
          <p :id="`${fieldId(row, column.key)}-error`" class="field-error">{{ issue(row, column.key) }}</p>
        </td><td><button class="text-button" type="button" :disabled="locked(row)" :aria-label="`${index + 1}행 삭제`" @click="emit('remove', row)">삭제</button><button v-if="kind === 'receipt'" class="text-button" type="button" :disabled="locked(row) || !material(row)" :aria-label="`${index + 1}행 실사 포함 근거 편집`" @click="emit('inclusions', row)">실사 포함 근거</button><p v-if="row.selections.length" class="field-hint">선택 근거 {{ row.selections.length }}개</p></td></tr></tbody>
    </table>
  </div>
</template>
<style scoped>
.bulk-table-scroll { position: relative; width: 100%; max-width: 100%; overflow: auto; border: 1px solid var(--line); border-radius: 10px; background: var(--surface); }
.bulk-table { width: 100%; border-collapse: collapse; text-align: left; }
caption { padding: 16px; text-align: left; font-weight: 700; }
th, td { border-top: 1px solid var(--line); padding: 12px; vertical-align: top; }
thead { background: var(--soft); }
td { min-width: 190px; }
td:has(select[id$='material_id']) { min-width: 290px; }
td:first-of-type { min-width: 200px; }
td:last-child { min-width: 130px; }
.bulk-reference { max-width: 280px; overflow-wrap: anywhere; font-size: .75rem; }
.field-error { max-width: 280px; }
.required-label { display: block; font-size: .75rem; }
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; }
</style>
