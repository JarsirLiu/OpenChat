import { readLocalApiCache, writeLocalApiCache } from '../../lib/localApiCache'
import type { UserCustomModel, UserProviderApiKey } from './types'

const PROVIDER_KEYS_CACHE_KEY = 'chat:provider-key-status'
const CUSTOM_MODELS_CACHE_KEY = 'chat:custom-models'
const SETTINGS_CACHE_TTL_MS = 60_000

export const readCachedProviderKeys = (userId: string | null | undefined) =>
  readLocalApiCache<UserProviderApiKey[]>(userId, PROVIDER_KEYS_CACHE_KEY, SETTINGS_CACHE_TTL_MS)

export const writeCachedProviderKeys = (
  userId: string | null | undefined,
  payload: UserProviderApiKey[],
) => {
  writeLocalApiCache(userId, PROVIDER_KEYS_CACHE_KEY, payload)
}

export const readCachedCustomModels = (userId: string | null | undefined) =>
  readLocalApiCache<UserCustomModel[]>(userId, CUSTOM_MODELS_CACHE_KEY, SETTINGS_CACHE_TTL_MS)

export const writeCachedCustomModels = (
  userId: string | null | undefined,
  payload: UserCustomModel[],
) => {
  writeLocalApiCache(userId, CUSTOM_MODELS_CACHE_KEY, payload)
}
