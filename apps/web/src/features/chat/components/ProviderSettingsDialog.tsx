import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'
import {
  AlertCircle,
  Check,
  CheckCircle2,
  ChevronDown,
  Eye,
  EyeOff,
  LoaderCircle,
  Plus,
  Trash2,
  X,
} from 'lucide-react'
import clsx from 'clsx'
import { AuthError, authenticatedFetch } from '../../../lib/auth'
import { createApiError, ensureOk, toApiError } from '../../../lib/apiError'
import type { UserCustomModel, UserProviderApiKey, UserProviderApiKeySecret } from '../types'

interface ProviderSettingsDialogProps {
  isOpen: boolean
  onClose: () => void
  onSaved: () => Promise<void> | void
  onUnauthorized: () => void
  autoFocusApiKey?: boolean
}

type ProviderFormState = {
  apiKey: string
  maskedApiKey: string
  hasStoredApiKey: boolean
}

type ModelDraftState = {
  modelName: string
  modelType: UserCustomModel['model_type']
  baseUrl: string
  apiKey: string
}

type FeedbackState = {
  type: 'success' | 'error'
  message: string
}

const DEFAULT_PROVIDER_KEY = 'openai'
const CUSTOM_MODEL_TYPE_OPTIONS: Array<{
  value: UserCustomModel['model_type']
  label: string
}> = [
  { value: 'text', label: '文本' },
  { value: 'multimodal', label: '多模态' },
  { value: 'image', label: '图片生成' },
]

const secretInputClassName =
  'h-11 w-full rounded-xl border border-gray-200 bg-white px-4 pr-11 text-[14px] text-gray-900 outline-none transition focus:border-gray-300 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-100 dark:focus:border-gray-500'

export function ProviderSettingsDialog({
  isOpen,
  onClose,
  onSaved,
  onUnauthorized,
  autoFocusApiKey = false,
}: ProviderSettingsDialogProps) {
  const closeButtonRef = useRef<HTMLButtonElement | null>(null)
  const [form, setForm] = useState<ProviderFormState>({
    apiKey: '',
    maskedApiKey: '',
    hasStoredApiKey: false,
  })
  const [draft, setDraft] = useState<ModelDraftState>({
    modelName: '',
    modelType: 'text',
    baseUrl: '',
    apiKey: '',
  })
  const [customModels, setCustomModels] = useState<UserCustomModel[]>([])
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [revealingProviderApiKey, setRevealingProviderApiKey] = useState(false)
  const [creating, setCreating] = useState(false)
  const [deletingModelId, setDeletingModelId] = useState<string | null>(null)
  const [feedback, setFeedback] = useState<FeedbackState | null>(null)
  const [modelTypeMenuOpen, setModelTypeMenuOpen] = useState(false)
  const [providerApiKeyVisible, setProviderApiKeyVisible] = useState(false)
  const [customModelApiKeyVisible, setCustomModelApiKeyVisible] = useState(false)
  const [customModelsExpanded, setCustomModelsExpanded] = useState(false)
  const modelTypePanelRef = useRef<HTMLDivElement | null>(null)
  const providerApiKeyInputRef = useRef<HTMLInputElement | null>(null)

  useEffect(() => {
    if (!isOpen) {
      return
    }

    let active = true

    const loadSettings = async () => {
      setLoading(true)
      setFeedback(null)

      try {
        const [providerResponse, customModelsResponse] = await Promise.all([
          authenticatedFetch('/api/user-provider-api-keys'),
          authenticatedFetch('/api/custom-models'),
        ])

        if (!providerResponse.ok || !customModelsResponse.ok) {
          throw await createApiError(
            !providerResponse.ok ? providerResponse : customModelsResponse,
            'Failed to load model access settings',
          )
        }

        const [providerPayload, customPayload] = (await Promise.all([
          providerResponse.json(),
          customModelsResponse.json(),
        ])) as [UserProviderApiKey[], UserCustomModel[]]

        if (!active) {
          return
        }

        const currentSetting =
          providerPayload.find((item) => item.provider_key === DEFAULT_PROVIDER_KEY) ?? null

        setForm({
          apiKey: '',
          maskedApiKey: currentSetting?.masked_api_key?.trim() ?? '',
          hasStoredApiKey: currentSetting?.has_api_key ?? false,
        })
        setCustomModels(customPayload)
      } catch (error) {
        if (error instanceof AuthError && error.status === 401) {
          onUnauthorized()
          return
        }
        if (!active) {
          return
        }
        setFeedback({
          type: 'error',
          message: toApiError(error, 'Failed to load model access settings').message,
        })
      } finally {
        if (active) {
          setLoading(false)
        }
      }
    }

    void loadSettings()

    return () => {
      active = false
    }
  }, [isOpen, onUnauthorized])

  useEffect(() => {
    if (!modelTypeMenuOpen) return
    const handlePointerDown = (event: MouseEvent) => {
      if (!modelTypePanelRef.current?.contains(event.target as Node)) {
        setModelTypeMenuOpen(false)
      }
    }
    window.addEventListener('mousedown', handlePointerDown)
    return () => window.removeEventListener('mousedown', handlePointerDown)
  }, [modelTypeMenuOpen])

  useEffect(() => {
    if (!feedback) {
      return
    }

    const timeout = window.setTimeout(() => {
      setFeedback(null)
    }, feedback.type === 'success' ? 2500 : 4500)

    return () => window.clearTimeout(timeout)
  }, [feedback])

  useEffect(() => {
    if (!isOpen || loading || !autoFocusApiKey) {
      return
    }

    const frameId = window.requestAnimationFrame(() => {
      providerApiKeyInputRef.current?.focus()
      providerApiKeyInputRef.current?.select()
    })

    return () => window.cancelAnimationFrame(frameId)
  }, [autoFocusApiKey, isOpen, loading])

  const handleClose = () => {
    const activeElement = document.activeElement
    if (activeElement instanceof HTMLElement) {
      activeElement.blur()
    }
    closeButtonRef.current?.blur()
    onClose()
  }

  const sortedCustomModels = useMemo(
    () =>
      [...customModels].sort((left, right) =>
        left.model_name.localeCompare(right.model_name, 'zh-CN'),
      ),
    [customModels],
  )

  const updateForm = (patch: Partial<ProviderFormState>) => {
    setForm((current) => ({ ...current, ...patch }))
    setFeedback(null)
  }

  const updateDraft = (patch: Partial<ModelDraftState>) => {
    setDraft((current) => ({ ...current, ...patch }))
    setFeedback(null)
  }

  const revealProviderApiKey = async () => {
    if (providerApiKeyVisible) {
      setProviderApiKeyVisible(false)
      return
    }

    if (form.apiKey.trim()) {
      setProviderApiKeyVisible(true)
      return
    }

    if (!form.hasStoredApiKey) {
      setProviderApiKeyVisible(true)
      return
    }

    setRevealingProviderApiKey(true)
    setFeedback(null)

    try {
      const response = await ensureOk(
        await authenticatedFetch(
          `/api/user-provider-api-keys/${encodeURIComponent(DEFAULT_PROVIDER_KEY)}`,
        ),
        'Failed to load API key',
      )

      const payload = (await response.json()) as UserProviderApiKeySecret
      setForm((current) => ({
        ...current,
        apiKey: payload.api_key,
      }))
      setProviderApiKeyVisible(true)
    } catch (error) {
      if (error instanceof AuthError && error.status === 401) {
        onUnauthorized()
        return
      }
      setFeedback({
        type: 'error',
        message: toApiError(error, 'Failed to load API key').message,
      })
    } finally {
      setRevealingProviderApiKey(false)
    }
  }

  const providerApiKeyDisplayValue = form.apiKey || form.maskedApiKey
  const providerApiKeyMasked =
    Boolean(providerApiKeyDisplayValue) &&
    !providerApiKeyVisible &&
    (Boolean(form.apiKey) || Boolean(form.maskedApiKey))

  const saveProvider = async () => {
    setSaving(true)
    setFeedback(null)

    try {
      const response = await ensureOk(await authenticatedFetch('/api/user-provider-api-keys', {
        method: 'PUT',
        body: JSON.stringify({
          provider_key: DEFAULT_PROVIDER_KEY,
          api_key: form.apiKey.trim() || undefined,
        }),
      }), 'Failed to save API key')

      const payload = (await response.json()) as UserProviderApiKey
      setForm((current) => ({
        ...current,
        apiKey: current.apiKey.trim(),
        maskedApiKey: payload.masked_api_key?.trim() ?? current.maskedApiKey,
        hasStoredApiKey: payload.has_api_key,
      }))
      setProviderApiKeyVisible(false)
      setFeedback({
        type: 'success',
        message: 'API Key 已保存',
      })
      await onSaved()
    } catch (error) {
      if (error instanceof AuthError && error.status === 401) {
        onUnauthorized()
        return
      }
      setFeedback({
        type: 'error',
        message: toApiError(error, 'Failed to save API key').message,
      })
    } finally {
      setSaving(false)
    }
  }

  const createCustomModel = async () => {
    if (!draft.modelName.trim()) {
      setFeedback({
        type: 'error',
        message: '请先填写模型名称',
      })
      return
    }
    if (!draft.apiKey.trim()) {
      setFeedback({
        type: 'error',
        message: '请先填写 API Key',
      })
      return
    }

    setCreating(true)
    setFeedback(null)

    try {
      const response = await ensureOk(await authenticatedFetch('/api/custom-models', {
        method: 'POST',
        body: JSON.stringify({
          model_name: draft.modelName.trim(),
          type: draft.modelType,
          base_url: draft.baseUrl.trim(),
          api_key: draft.apiKey.trim(),
        }),
      }), 'Failed to create custom model')

      const payload = (await response.json()) as UserCustomModel
      setCustomModels((current) => [...current, payload])
      setDraft((current) => ({ ...current, modelName: '', apiKey: '' }))
      setFeedback({
        type: 'success',
        message: '自定义模型已添加，并已进入可选列表',
      })
      await onSaved()
    } catch (error) {
      if (error instanceof AuthError && error.status === 401) {
        onUnauthorized()
        return
      }
      setFeedback({
        type: 'error',
        message: toApiError(error, 'Failed to create custom model').message,
      })
    } finally {
      setCreating(false)
    }
  }

  const deleteCustomModel = async (modelConfigId: string) => {
    setDeletingModelId(modelConfigId)
    setFeedback(null)

    try {
      await ensureOk(await authenticatedFetch(
        `/api/custom-models/${encodeURIComponent(modelConfigId)}`,
        {
          method: 'DELETE',
        },
      ), 'Failed to delete custom model')

      setCustomModels((current) =>
        current.filter((item) => item.model_config_id !== modelConfigId),
      )
      setFeedback({
        type: 'success',
        message: '自定义模型已删除',
      })
      await onSaved()
    } catch (error) {
      if (error instanceof AuthError && error.status === 401) {
        onUnauthorized()
        return
      }
      setFeedback({
        type: 'error',
        message: toApiError(error, 'Failed to delete custom model').message,
      })
    } finally {
      setDeletingModelId(null)
    }
  }

  return (
    <>
      {isOpen ? (
        <div
          className="fixed inset-0 z-30 bg-black/20 lg:hidden"
          onClick={handleClose}
          aria-hidden="true"
        />
      ) : null}

      <aside
        className={clsx(
          'z-40 flex h-full shrink-0 overflow-hidden border-l border-gray-100 bg-white transition-all duration-300 ease-out dark:border-gray-800 dark:bg-[#151515]',
          isOpen
            ? 'fixed inset-y-0 right-0 w-full sm:w-[420px] lg:static lg:w-[420px]'
            : 'w-0 border-l-0',
        )}
      >
        <div
          className={clsx(
            'flex h-full w-[420px] flex-col',
            isOpen ? 'opacity-100' : 'pointer-events-none opacity-0',
          )}
        >
          <div className="flex items-center justify-between border-b border-gray-100 px-5 py-3 dark:border-gray-800">
            <div className="flex items-center">
              <div className="relative px-1 py-1 text-sm font-semibold text-gray-900 dark:text-white after:absolute after:-bottom-[13px] after:left-0 after:right-0 after:h-0.5 after:bg-gray-900 dark:after:bg-white">
                参数
              </div>
            </div>
            <button
              ref={closeButtonRef}
              type="button"
              onClick={handleClose}
              className="rounded-md p-1.5 text-gray-400 transition hover:bg-gray-100 hover:text-gray-700 dark:hover:bg-gray-800 dark:hover:text-gray-200"
              aria-label="Close settings"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto">
            {loading ? (
              <div className="flex min-h-[280px] items-center justify-center gap-3 px-5 text-sm text-gray-500 dark:text-gray-400">
                <LoaderCircle className="h-4 w-4 animate-spin" />
                <span>正在读取配置…</span>
              </div>
            ) : (
              <div className="divide-y divide-gray-100 dark:divide-gray-800">
                <section className="px-5 py-5">
                  <div className="mb-4 flex items-start justify-between gap-4">
                    <div>
                      <h3 className="text-[15px] font-semibold text-gray-900 dark:text-white">
                        API Key 设置
                      </h3>
                      <p className="mt-1 text-[13px] leading-6 text-gray-500 dark:text-gray-400">
                        请先前往{' '}
                        <a
                          href="https://colorect.tech/tokens"
                          target="_blank"
                          rel="noreferrer"
                          className="font-medium text-gray-900 underline underline-offset-2 dark:text-white"
                        >
                          colorect.tech/tokens
                        </a>{' '}
                        获取 API Key，充值后在此填写，并使用模型。
                      </p>
                    </div>
                  </div>

                  <form
                    className="space-y-4"
                    onSubmit={(event) => {
                      event.preventDefault()
                      void saveProvider()
                    }}
                  >
                    <label className="block">
                      <div className="mb-2 flex items-center justify-between text-[13px] font-medium text-gray-700 dark:text-gray-300">
                        <span>API Key</span>
                        {form.hasStoredApiKey ? (
                          <span className="text-[12px] font-normal text-emerald-600 dark:text-emerald-400">
                            已保存
                          </span>
                        ) : null}
                      </div>
                      {!form.hasStoredApiKey ? (
                        <div className="mb-3 rounded-xl border border-gray-100 bg-gray-50 px-3 py-2 text-[12px] leading-5 text-gray-500 dark:border-gray-800 dark:bg-[#171717] dark:text-gray-400">
                          保存 API Key 后即可使用模型。
                        </div>
                      ) : null}
                      <div className="relative">
                        <input
                          ref={providerApiKeyInputRef}
                          name="provider_api_key"
                          autoComplete="off"
                          spellCheck={false}
                          type="text"
                          value={providerApiKeyDisplayValue}
                          onChange={(event) =>
                            updateForm({
                              apiKey: event.target.value,
                              maskedApiKey: '',
                            })
                          }
                          placeholder="请输入你购买的 API Key"
                          data-secret-masked={providerApiKeyMasked ? 'true' : 'false'}
                          className={secretInputClassName}
                          style={
                            providerApiKeyMasked
                              ? ({ WebkitTextSecurity: 'disc' } as CSSProperties)
                              : undefined
                          }
                        />
                        <button
                          type="button"
                          onClick={() => void revealProviderApiKey()}
                          disabled={revealingProviderApiKey}
                          className="absolute inset-y-0 right-0 inline-flex w-11 items-center justify-center text-gray-400 transition hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300"
                          aria-label={providerApiKeyVisible ? '隐藏 API Key' : '显示 API Key'}
                        >
                          {revealingProviderApiKey ? (
                            <LoaderCircle className="h-4 w-4 animate-spin" />
                          ) : providerApiKeyVisible ? (
                            <EyeOff className="h-4 w-4" />
                          ) : (
                            <Eye className="h-4 w-4" />
                          )}
                        </button>
                      </div>
                    </label>

                    <div className="flex items-center justify-end">
                      <button
                        type="submit"
                        disabled={saving}
                        className="inline-flex h-9 items-center gap-2 rounded-lg border border-gray-200 bg-white px-4 text-[13px] font-medium text-gray-700 transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-200 dark:hover:bg-gray-800"
                      >
                        {saving ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" /> : null}
                        <span>{saving ? '保存中' : '保存'}</span>
                      </button>
                    </div>
                  </form>
                </section>

                <section className="px-5 py-5">
                  <div className="mb-1">
                    <button
                      type="button"
                      onClick={() => setCustomModelsExpanded((expanded) => !expanded)}
                      className="flex w-full items-center justify-between gap-3 rounded-xl px-1 py-1 text-left transition hover:bg-gray-50 dark:hover:bg-gray-800/60"
                      aria-expanded={customModelsExpanded}
                    >
                      <div>
                        <h3 className="text-[15px] font-semibold text-gray-900 dark:text-white">
                          自定义模型
                        </h3>
                        <p className="mt-1 text-[13px] leading-6 text-gray-500 dark:text-gray-400">
                          高级用户可在这里接入额外模型配置。
                        </p>
                      </div>
                      <ChevronDown
                        className={clsx(
                          'h-4 w-4 flex-shrink-0 text-gray-400 transition-transform',
                          customModelsExpanded && 'rotate-180',
                        )}
                      />
                    </button>
                  </div>

                  <div className={clsx('space-y-3 pt-3', !customModelsExpanded && 'hidden')}>
                    <div className="space-y-3">
                      <form
                        className="space-y-3"
                        onSubmit={(event) => {
                          event.preventDefault()
                          void createCustomModel()
                        }}
                      >
                        <label className="block">
                          <div className="mb-2 text-[13px] font-medium text-gray-700 dark:text-gray-300">
                            模型名称
                          </div>
                          <input
                            type="text"
                            value={draft.modelName}
                            onChange={(event) => updateDraft({ modelName: event.target.value })}
                            placeholder="如 gpt-4.1-mini 或 gemini-2.5-flash"
                            className="h-11 w-full rounded-xl border border-gray-200 bg-white px-4 text-[14px] text-gray-900 outline-none transition focus:border-gray-300 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-100 dark:focus:border-gray-500"
                          />
                        </label>

                        <label className="block">
                          <div className="mb-2 text-[13px] font-medium text-gray-700 dark:text-gray-300">
                            Base URL
                          </div>
                          <input
                            type="url"
                            value={draft.baseUrl}
                            onChange={(event) => updateDraft({ baseUrl: event.target.value })}
                            placeholder="请输入 OpenAI 兼容 Base URL"
                            className="h-11 w-full rounded-xl border border-gray-200 bg-white px-4 text-[14px] text-gray-900 outline-none transition focus:border-gray-300 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-100 dark:focus:border-gray-500"
                          />
                        </label>

                        <div className="flex items-end gap-2">
                          <div className="relative min-w-0 flex-1" ref={modelTypePanelRef}>
                            <div className="mb-2 text-[13px] font-medium text-gray-700 dark:text-gray-300">
                              模型能力
                            </div>
                            <button
                              type="button"
                              onClick={() => setModelTypeMenuOpen((open) => !open)}
                              className="flex h-11 w-full items-center justify-between rounded-xl border border-gray-200 bg-white px-4 text-[14px] text-gray-900 transition hover:bg-gray-50 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-100 dark:hover:bg-gray-800"
                            >
                              <span>
                                {CUSTOM_MODEL_TYPE_OPTIONS.find((item) => item.value === draft.modelType)
                                  ?.label ?? '选择能力'}
                              </span>
                              <ChevronDown className="h-4 w-4 text-gray-400" />
                            </button>

                            {modelTypeMenuOpen ? (
                              <div className="absolute left-0 top-full z-50 mt-2 w-full overflow-hidden rounded-xl border border-gray-100 bg-white shadow-[0_10px_40px_rgba(0,0,0,0.08)] dark:border-gray-700 dark:bg-[#1e1e1e] dark:shadow-[0_10px_40px_rgba(0,0,0,0.4)]">
                                {CUSTOM_MODEL_TYPE_OPTIONS.map((option) => {
                                  const active = draft.modelType === option.value
                                  return (
                                    <button
                                      key={option.value}
                                      type="button"
                                      className={clsx(
                                        'flex w-full items-center justify-between px-4 py-3 text-left transition-colors',
                                        active
                                          ? 'bg-gray-50 dark:bg-gray-800'
                                          : 'hover:bg-gray-50 dark:hover:bg-gray-800',
                                      )}
                                      onClick={() => {
                                        updateDraft({ modelType: option.value })
                                        setModelTypeMenuOpen(false)
                                      }}
                                    >
                                      <div className="min-w-0">
                                        <div className="text-[14px] text-gray-900 dark:text-gray-100">
                                          {option.label}
                                        </div>
                                      </div>
                                      {active ? (
                                        <Check className="h-4 w-4 flex-shrink-0 text-blue-600 dark:text-blue-400" />
                                      ) : null}
                                    </button>
                                  )
                                })}
                              </div>
                            ) : null}
                          </div>

                          <label className="min-w-0 flex-1">
                            <div className="mb-2 text-[13px] font-medium text-gray-700 dark:text-gray-300">
                              API Key
                            </div>
                            <div className="relative">
                              <input
                                name="custom_model_api_key"
                                autoComplete="off"
                                spellCheck={false}
                                type="text"
                                value={draft.apiKey}
                                onChange={(event) => updateDraft({ apiKey: event.target.value })}
                                placeholder="请输入这个自定义模型专属的 API Key"
                                data-secret-masked={customModelApiKeyVisible ? 'false' : 'true'}
                                className={secretInputClassName}
                                style={
                                  customModelApiKeyVisible
                                    ? undefined
                                    : ({ WebkitTextSecurity: 'disc' } as CSSProperties)
                                }
                              />
                              <button
                                type="button"
                                onClick={() => setCustomModelApiKeyVisible((visible) => !visible)}
                                className="absolute inset-y-0 right-0 inline-flex w-11 items-center justify-center text-gray-400 transition hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300"
                                aria-label={
                                  customModelApiKeyVisible ? '隐藏 API Key' : '显示 API Key'
                                }
                              >
                                {customModelApiKeyVisible ? (
                                  <EyeOff className="h-4 w-4" />
                                ) : (
                                  <Eye className="h-4 w-4" />
                                )}
                              </button>
                            </div>
                          </label>

                          <button
                            type="submit"
                            disabled={creating}
                            className="inline-flex h-11 items-center gap-1.5 rounded-xl border border-gray-200 bg-white px-4 text-[13px] font-medium text-gray-700 transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-200 dark:hover:bg-gray-800"
                          >
                            {creating ? (
                              <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
                            ) : (
                              <Plus className="h-3.5 w-3.5" />
                            )}
                            <span>添加</span>
                          </button>
                        </div>
                      </form>
                    </div>

                    <div className="rounded-2xl border border-gray-100 bg-gray-50/70 dark:border-gray-800 dark:bg-[#171717]">
                      {customModels.length === 0 ? (
                        <div className="px-4 py-4 text-[13px] text-gray-400">
                          还没有配置自定义模型
                        </div>
                      ) : (
                        <div className="divide-y divide-gray-100 dark:divide-gray-800">
                          {sortedCustomModels.map((model) => (
                            <div
                              key={model.model_config_id}
                              className="flex items-center justify-between gap-3 px-4 py-3"
                            >
                              <div className="min-w-0">
                                <div className="flex min-w-0 items-center gap-2">
                                  <div className="truncate text-[14px] text-gray-900 dark:text-gray-100">
                                    {model.model_name}
                                  </div>
                                  <span className="rounded-full border border-gray-200 px-2 py-0.5 text-[11px] text-gray-500 dark:border-gray-700 dark:text-gray-400">
                                    {CUSTOM_MODEL_TYPE_OPTIONS.find((item) => item.value === model.model_type)
                                      ?.label ?? model.model_type}
                                  </span>
                                </div>
                                <div className="mt-1 truncate text-[12px] text-gray-400">
                                  {model.base_url}
                                </div>
                              </div>
                              <button
                                type="button"
                                onClick={() => void deleteCustomModel(model.model_config_id)}
                                className="rounded-md p-1.5 text-gray-400 transition hover:bg-red-50 hover:text-red-500 dark:hover:bg-red-900/20"
                                aria-label={`Delete ${model.model_name}`}
                              >
                                {deletingModelId === model.model_config_id ? (
                                  <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
                                ) : (
                                  <Trash2 className="h-3.5 w-3.5" />
                                )}
                              </button>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                </section>

                {feedback ? (
                  <section className="px-5 py-4">
                    <div
                      className={clsx(
                        'flex items-start gap-2 rounded-xl border px-3 py-3 text-[13px]',
                        feedback.type === 'success'
                          ? 'border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900/40 dark:bg-emerald-950/30 dark:text-emerald-300'
                          : 'border-red-200 bg-red-50 text-red-700 dark:border-red-900/40 dark:bg-red-950/30 dark:text-red-300',
                      )}
                    >
                      {feedback.type === 'success' ? (
                        <CheckCircle2 className="mt-0.5 h-4 w-4 flex-shrink-0" />
                      ) : (
                        <AlertCircle className="mt-0.5 h-4 w-4 flex-shrink-0" />
                      )}
                      <span>{feedback.message}</span>
                    </div>
                  </section>
                ) : null}
              </div>
            )}
          </div>
        </div>
      </aside>
    </>
  )
}
