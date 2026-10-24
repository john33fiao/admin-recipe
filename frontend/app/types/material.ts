export interface Material {
  id: string
  name: string
  unit: string
  created_at: string
}

export interface MaterialInput {
  name: string
  unit: string
}

export interface PendingMaterial extends MaterialInput {
  request_id: string
}
