export interface CatalogModel {
  model_config_id: string
  provider: string
  display_provider: string
  model: string
  display_name: string
  source: string
  type: 'text' | 'multimodal' | 'image' | 'video'
  input_modalities: string[]
  official: boolean
  custom: boolean
  available: boolean
  unavailable_reason?: string | null
}

export interface CatalogTool {
  model_config_id: string
  id: string
  provider: string
  display_provider: string
  model: string
  source: string
  type: string
  display_name: string
  available: boolean
  enabled: boolean
  unavailable_reason?: string | null
}

export interface UserProviderApiKey {
  provider_key: string
  has_api_key: boolean
  created_at: string
  updated_at: string
}

export interface UserCustomModel {
  model_config_id: string
  model_name: string
  model_type: 'text' | 'multimodal' | 'image'
  base_url: string
  has_api_key: boolean
  created_at: string
  updated_at: string
}

export interface UploadedImageAttachment {
  id: string
  url: string
  name: string
  mime_type: string
  size_bytes: number
}

export interface SessionPreference {
  textModelId: string | null
  imageToolKey: string | null
}

export type SessionPreferenceMap = Record<string, SessionPreference>

export type ModelMenuItem = {
  key: string
  provider: string
  label: string
  meta?: string
  iconKey?: string
  modelType?: CatalogModel['type']
  inputModalities?: string[]
  available?: boolean
  unavailableReason?: string | null
}
