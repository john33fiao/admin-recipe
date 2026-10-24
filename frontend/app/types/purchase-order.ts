import type { CatalogState } from './material'

export type OrderAction = 'place' | 'revise' | 'cancel'
export type OrderStatus = 'open' | 'received' | 'cancelled'
export interface DraftOrderFields { material_version_id: string | null; quantity: string; unit: string; supplier: string; expected_date: string; estimated_amount: string; note: string }
export type DraftOrderActionV1 =
  | ({ action: 'place'; material_id: string | null } & DraftOrderFields)
  | ({ action: 'revise'; order_id: string; expected_order_version_id: string } & DraftOrderFields)
  | { action: 'cancel'; order_id: string; expected_order_version_id: string; note: string }
export interface OrderPayload { kind: 'purchase_order'; version: 1; data: DraftOrderActionV1 }
export interface OrderProposal { status: 'open' | 'cancelled'; material_id: string; material_version_id: string; quantity: string; unit: string; input_quantity: string; input_unit: string; supplier: string | null; expected_date: string | null; estimated_amount: string | null; note: string }
export interface OrderVersion extends Omit<OrderProposal, 'status'> { id: string; order_id: string; previous_version_id: string | null; confirmation_id: string; business_record_id: string; action: OrderAction; status: OrderStatus; material_name: string; recorded_at: string; seq: string }
export interface OrderSummary extends Omit<OrderProposal, 'status' | 'note'> { id: string; head_id: string; status: OrderStatus; material_name: string; created_at: string; updated_at: string }
export interface OrderList { items: OrderSummary[]; state: CatalogState }
export interface OrderDetail { item: OrderSummary; head: OrderVersion; versions: OrderVersion[]; state: CatalogState }
export interface OrderBusinessResult { kind: 'purchase_order'; action: OrderAction; order_id: string; order_version_id: string; material_id: string; material_version_id: string }
export interface OrderBusinessEffect { kind: 'purchase_order'; action: OrderAction; order_id: string | null; before: OrderVersion | null; after: OrderProposal }
