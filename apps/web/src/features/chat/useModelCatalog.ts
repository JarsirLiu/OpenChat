import { useEffect, useMemo, useState } from 'react'
import { AuthError, authenticatedFetch } from '../../lib/auth'
import { resolveModelIconKey } from './modelIcon'
import { imageToolKeyOf, readSessionPreferences } from './sessionPreferences'
import type { CatalogModel, CatalogTool, ModelMenuItem } from './types'

const DEFAULT_TEXT_MODEL_ID = 'openchat:gpt-5.4'

const compareTextModels = (left: CatalogModel, right: CatalogModel) => {
  const leftIsGpt = left.model.toLowerCase().startsWith('gpt')
  const rightIsGpt = right.model.toLowerCase().startsWith('gpt')

  if (leftIsGpt !== rightIsGpt) {
    return leftIsGpt ? -1 : 1
  }

  return left.display_name.localeCompare(right.display_name)
}

interface UseModelCatalogParams {
  currentUserId: string | null
  sessionId: string
  onUnauthorized: () => void
}

export function useModelCatalog({
  currentUserId,
  sessionId,
  onUnauthorized,
}: UseModelCatalogParams) {
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [textModels, setTextModels] = useState<CatalogModel[]>([])
  const [imageTools, setImageTools] = useState<CatalogTool[]>([])
  const [selectedTextModelId, setSelectedTextModelId] = useState<string | null>(
    readSessionPreferences(currentUserId)[sessionId]?.textModelId ?? null,
  )
  const [selectedImageToolKey, setSelectedImageToolKey] = useState<string | null>(
    readSessionPreferences(currentUserId)[sessionId]?.imageToolKey ?? null,
  )

  useEffect(() => {
    setSelectedTextModelId(readSessionPreferences(currentUserId)[sessionId]?.textModelId ?? null)
    setSelectedImageToolKey(readSessionPreferences(currentUserId)[sessionId]?.imageToolKey ?? null)
  }, [currentUserId, sessionId])

  const loadCatalog = async () => {
    if (!currentUserId) {
      setLoading(false)
      setTextModels([])
      setImageTools([])
      return
    }

    setLoading(true)
    setError(null)

    try {
      const [modelsResponse, toolsResponse] = await Promise.all([
        authenticatedFetch('/api/list_models'),
        authenticatedFetch('/api/list_tools'),
      ])

      if (!modelsResponse.ok || !toolsResponse.ok) {
        throw new Error('Failed to load OpenChat model catalog')
      }

      const [modelsPayload, toolsPayload] = (await Promise.all([
        modelsResponse.json(),
        toolsResponse.json(),
      ])) as [CatalogModel[], CatalogTool[]]

      const nextTextModels = modelsPayload
        .filter((model) => model.type === 'text' || model.type === 'multimodal')
        .sort(compareTextModels)
      const nextImageTools = toolsPayload.filter(
        (tool) => tool.type === 'image' && tool.available !== false,
      )

      setTextModels(nextTextModels)
      setImageTools(nextImageTools)

      setSelectedTextModelId((current) => {
        const sessionPreference =
          readSessionPreferences(currentUserId)[sessionId]?.textModelId ?? null
        const currentId =
          current ||
          sessionPreference ||
          import.meta.env.VITE_DEFAULT_TEXT_MODEL_ID ||
          DEFAULT_TEXT_MODEL_ID ||
          nextTextModels[0]?.model_config_id ||
          null
        if (nextTextModels.some((model) => model.model_config_id === currentId)) {
          return currentId
        }

        if (
          DEFAULT_TEXT_MODEL_ID &&
          nextTextModels.some((model) => model.model_config_id === DEFAULT_TEXT_MODEL_ID)
        ) {
          return DEFAULT_TEXT_MODEL_ID
        }

        return nextTextModels[0]?.model_config_id ?? null
      })

      setSelectedImageToolKey((current) => {
        const sessionPreference =
          readSessionPreferences(currentUserId)[sessionId]?.imageToolKey ?? null
        const envKey =
          import.meta.env.VITE_DEFAULT_IMAGE_TOOL_ID &&
          import.meta.env.VITE_DEFAULT_IMAGE_TOOL_CONFIG_ID
            ? `${import.meta.env.VITE_DEFAULT_IMAGE_TOOL_ID}::${import.meta.env.VITE_DEFAULT_IMAGE_TOOL_CONFIG_ID}`
            : null
        const currentKey = current || sessionPreference || envKey
        return currentKey && nextImageTools.some((tool) => imageToolKeyOf(tool) === currentKey)
          ? currentKey
          : null
      })
    } catch (error) {
      if (error instanceof AuthError && error.status === 401) {
        onUnauthorized()
      }
      setError(error instanceof Error ? error.message : 'Failed to load OpenChat model catalog')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    let active = true

    const run = async () => {
      await loadCatalog()
      if (!active) {
        return
      }
    }

    void run()

    return () => {
      active = false
    }
  }, [currentUserId, onUnauthorized, sessionId])

  const selectedTextModel = useMemo(
    () => textModels.find((model) => model.model_config_id === selectedTextModelId) ?? null,
    [selectedTextModelId, textModels],
  )
  const selectedImageTool = useMemo(
    () => imageTools.find((tool) => imageToolKeyOf(tool) === selectedImageToolKey) ?? null,
    [imageTools, selectedImageToolKey],
  )
  const textMenuItems = useMemo<ModelMenuItem[]>(
    () =>
      textModels.map((model) => ({
        key: model.model_config_id,
        provider: '',
        label: model.display_name,
        meta: model.model,
        modelType: model.type,
        inputModalities: model.input_modalities,
        iconKey: resolveModelIconKey(model.model_config_id, model.display_provider, model.model),
        available: model.available,
        unavailableReason: model.unavailable_reason,
      })),
    [textModels],
  )
  const imageMenuItems = useMemo<ModelMenuItem[]>(
    () =>
      imageTools.map((tool) => ({
        key: imageToolKeyOf(tool),
        provider: '',
        label: tool.display_name,
        meta: tool.model,
        iconKey: resolveModelIconKey(tool.id, tool.display_provider, tool.model),
        available: tool.available,
        unavailableReason: tool.unavailable_reason,
      })),
    [imageTools],
  )
  return {
    loading,
    error,
    textModels,
    imageTools,
    selectedTextModelId,
    setSelectedTextModelId,
    selectedImageToolKey,
    setSelectedImageToolKey,
    selectedTextModel,
    selectedImageTool,
    textMenuItems,
    imageMenuItems,
    refreshCatalog: loadCatalog,
  }
}
