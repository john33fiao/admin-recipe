import { readonly, shallowRef } from 'vue'
import type { CatalogState } from '../types/material.ts'

const current = shallowRef<CatalogState | null>(null)
const changed = shallowRef(false)
let generation = 0
export const currentAppState = readonly(current)
export const restoredStateNotice = readonly(changed)
export const currentStateGeneration = () => generation
export function catalogState(value: unknown): CatalogState | null {
  if (!value || typeof value !== 'object') return null
  const state = value as Record<string, unknown>
  return typeof state.epoch === 'string' && typeof state.revision === 'string' && /^\d+$/.test(state.revision) ? { epoch: state.epoch, revision: state.revision } : null
}
export function observeCurrentState(state: CatalogState, started: number): boolean {
  if (current.value?.epoch === state.epoch) {
    if (BigInt(state.revision) > BigInt(current.value.revision)) current.value = { ...state }
    return true
  }
  if (started !== generation) return false
  if (current.value) changed.value = true
  current.value = { ...state }; generation += 1
  return true
}
export function acknowledgeRestoredState() { changed.value = false }
export function announceRestore() {
  if (typeof window === 'undefined') return
  try { localStorage.setItem('sunchang.restored.v1', crypto.randomUUID()) } catch { /* Other tabs also verify state when they reconnect. */ }
}
