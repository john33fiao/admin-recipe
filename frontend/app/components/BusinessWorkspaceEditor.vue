<script setup lang="ts">
import type { BusinessDetail } from '~/types/business'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { BusinessEditorHost, WorkspacePayload } from '~/types/workspace'
defineProps<{ identity: string; initial: BusinessEditorInitial<WorkspacePayload>; host: BusinessEditorHost; refresh: () => Promise<BusinessDetail | null> }>()
</script>
<template>
  <BusinessSaleEditor v-if="initial.payload.kind === 'sale'" :identity="identity" :initial="{ ...initial, payload: initial.payload }" :refresh="refresh" :host="host" />
  <BusinessProductionEditor v-else-if="initial.payload.kind === 'production'" :identity="identity" :initial="{ ...initial, payload: initial.payload }" :refresh="refresh" :host="host" />
  <BusinessStockEditor v-else :identity="identity" :initial="{ ...initial, payload: initial.payload }" :refresh="refresh" :host="host" />
</template>
