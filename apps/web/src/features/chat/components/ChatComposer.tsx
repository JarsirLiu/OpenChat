import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type FormEvent, type KeyboardEvent } from 'react'
import { ArrowUp, Check, ChevronDown, ImagePlus, Plus, Search, Square, X } from 'lucide-react'
import clsx from 'clsx'
import {
  filterSupportedImageFiles,
  SUPPORTED_IMAGE_ACCEPT,
} from '../imageUpload'
import { resolveModelIconKey } from '../modelIcon'
import { ModelAvatar } from './ModelAvatar'
import type {
  ModelMenuItem,
  UploadedImageAttachment,
} from '../types'

const modelTypeLabel = (modelType: ModelMenuItem['modelType']) => {
  switch (modelType) {
    case 'multimodal':
      return '多模态'
    case 'text':
      return '文本'
    default:
      return null
  }
}

export function ChatComposer({
  value,
  pending,
  disabled,
  placeholder,
  imageTools,
  selectedImageToolKey,
  selectedImageToolLabel,
  selectedImageToolAvailable,
  models,
  selectedModelKey,
  selectedModelLabel,
  selectedModelProvider,
  imageToolLoading,
  modelLoading,
  attachments,
  canUploadImages,
  onChange,
  onClearImageTool,
  onRemoveAttachment,
  onSelectImageTool,
  onSelectModel,
  onInterrupt,
  onSubmit,
  onUploadImages,
}: {
  value: string
  pending: boolean
  disabled: boolean
  placeholder: string
  imageTools: ModelMenuItem[]
  selectedImageToolKey: string | null
  selectedImageToolLabel: string
  selectedImageToolAvailable: boolean
  models: ModelMenuItem[]
  selectedModelKey: string | null
  selectedModelLabel: string
  selectedModelProvider: string
  imageToolLoading: boolean
  modelLoading: boolean
  attachments: UploadedImageAttachment[]
  canUploadImages: boolean
  onChange: (value: string) => void
  onClearImageTool: () => void
  onRemoveAttachment: (attachmentId: string) => void
  onSelectImageTool: (toolKey: string | null) => void
  onSelectModel: (modelKey: string) => void
  onInterrupt: () => void
  onSubmit: () => void
  onUploadImages: (files: File[]) => Promise<void>
}) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  const fileInputRef = useRef<HTMLInputElement | null>(null)
  const modelPanelRef = useRef<HTMLDivElement | null>(null)
  const imageToolPanelRef = useRef<HTMLDivElement | null>(null)
  const [modelMenuOpen, setModelMenuOpen] = useState(false)
  const [imageToolMenuOpen, setImageToolMenuOpen] = useState(false)
  const [modelQuery, setModelQuery] = useState('')
  const [imageToolQuery, setImageToolQuery] = useState('')
  const [attachmentNoticeVisible, setAttachmentNoticeVisible] = useState(false)

  useEffect(() => {
    const textarea = textareaRef.current
    if (!textarea) return
    textarea.style.height = '0px'
    textarea.style.height = `${Math.min(textarea.scrollHeight, window.innerHeight * 0.35)}px`
  }, [value])

  useEffect(() => {
    if (!modelMenuOpen) return
    const handlePointerDown = (event: MouseEvent) => {
      if (!modelPanelRef.current?.contains(event.target as Node)) {
        setModelMenuOpen(false)
      }
    }
    window.addEventListener('mousedown', handlePointerDown)
    return () => window.removeEventListener('mousedown', handlePointerDown)
  }, [modelMenuOpen])

  useEffect(() => {
    if (!imageToolMenuOpen) return
    const handlePointerDown = (event: MouseEvent) => {
      if (!imageToolPanelRef.current?.contains(event.target as Node)) {
        setImageToolMenuOpen(false)
      }
    }
    window.addEventListener('mousedown', handlePointerDown)
    return () => window.removeEventListener('mousedown', handlePointerDown)
  }, [imageToolMenuOpen])

  useEffect(() => {
    if (!attachmentNoticeVisible) {
      return
    }

    const timeout = window.setTimeout(() => {
      setAttachmentNoticeVisible(false)
    }, 3500)

    return () => window.clearTimeout(timeout)
  }, [attachmentNoticeVisible])

  const filteredModels = useMemo(() => {
    const query = modelQuery.trim().toLowerCase()
    if (!query) return models
    return models.filter((model) =>
      `${model.label} ${model.provider} ${model.meta ?? ''}`.toLowerCase().includes(query),
    )
  }, [modelQuery, models])

  const filteredImageTools = useMemo(() => {
    const query = imageToolQuery.trim().toLowerCase()
    if (!query) return imageTools
    return imageTools.filter((tool) =>
      `${tool.label} ${tool.provider} ${tool.meta ?? ''}`.toLowerCase().includes(query),
    )
  }, [imageToolQuery, imageTools])

  const selectedModel = useMemo(
    () => models.find((model) => model.key === selectedModelKey) ?? null,
    [models, selectedModelKey],
  )
  const selectedImageTool = useMemo(
    () => imageTools.find((tool) => tool.key === selectedImageToolKey) ?? null,
    [imageTools, selectedImageToolKey],
  )
  const selectedModelAvailable = selectedModel?.available !== false
  const selectedModelSupportsImageInputs =
    selectedModel?.inputModalities?.some((modality) => {
      const normalized = modality.toLowerCase()
      return normalized === 'image' || normalized === 'vision'
    }) ?? false

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (pending) {
      onInterrupt()
      return
    }
    if (disabled || (!value.trim() && attachments.length === 0)) return
    onSubmit()
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== 'Enter' || event.shiftKey || event.nativeEvent.isComposing) return
    event.preventDefault()
    if (disabled || pending || (!value.trim() && attachments.length === 0)) return
    onSubmit()
  }

  const handlePaste = async (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const files = Array.from(event.clipboardData.files ?? [])
    const supportedFiles = filterSupportedImageFiles(files)

    if (supportedFiles.length === 0) {
      if (files.length > 0) {
        event.preventDefault()
        setAttachmentNoticeVisible(true)
      }
      return
    }

    if (!canUploadImages) {
      setAttachmentNoticeVisible(true)
      return
    }

    event.preventDefault()
    await onUploadImages(supportedFiles)
  }

  const canSend =
    !disabled &&
    !pending &&
    selectedModelAvailable &&
    (value.trim().length > 0 || attachments.length > 0)

  return (
    <form onSubmit={handleSubmit} className="mx-auto w-full max-w-[800px]">
      <div className="relative overflow-visible rounded-[24px] border border-gray-200 bg-white shadow-[0_10px_30px_rgba(0,0,0,0.06)] dark:border-gray-700 dark:bg-[#1e1e1e] dark:shadow-[0_10px_30px_rgba(0,0,0,0.28)]">
        {attachments.length > 0 ? (
          <div className="flex flex-wrap gap-2 px-4 pb-2 pt-4">
            {attachments.map((attachment) => (
              <div
                key={attachment.id}
                className="relative h-12 w-12 overflow-hidden rounded-lg border border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-900"
              >
                <img
                  src={attachment.url}
                  alt={attachment.name}
                  className="h-full w-full object-cover"
                />
                <button
                  type="button"
                  onClick={() => onRemoveAttachment(attachment.id)}
                  className="absolute right-1 top-1 flex h-5 w-5 items-center justify-center rounded-full bg-black/70 text-white transition hover:bg-black/80"
                  aria-label={`移除 ${attachment.name}`}
                >
                  <X className="h-3 w-3" strokeWidth={2.5} />
                </button>
              </div>
            ))}
          </div>
        ) : null}

        <textarea
          ref={textareaRef}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={(event) => {
            void handlePaste(event)
          }}
          disabled={disabled}
          rows={1}
          placeholder={placeholder}
          className="min-h-[56px] max-h-[35vh] w-full resize-none bg-transparent px-4 pb-2 pt-4 text-[15px] leading-[1.75] text-gray-800 outline-none placeholder-gray-400 dark:text-gray-100 dark:placeholder-gray-500 sm:text-[14px]"
        />

        <div className="flex flex-nowrap items-center justify-between gap-2 px-3 pb-3 pt-1">
          <div className="flex min-w-0 flex-1 flex-nowrap items-center gap-1 overflow-hidden sm:overflow-visible">
            <input
              ref={fileInputRef}
              type="file"
              accept={SUPPORTED_IMAGE_ACCEPT}
              multiple
              className="hidden"
              onChange={(event) => {
                const files = Array.from(event.target.files ?? [])
                event.currentTarget.value = ''
                if (files.length === 0) return
                const supportedFiles = filterSupportedImageFiles(files)
                if (supportedFiles.length === 0) {
                  setAttachmentNoticeVisible(true)
                  return
                }
                void onUploadImages(supportedFiles)
              }}
            />
            <button
              type="button"
              disabled={!canUploadImages || pending}
              onClick={() => fileInputRef.current?.click()}
              className={clsx(
                'flex h-8 w-8 items-center justify-center rounded-full border transition-colors',
                canUploadImages && !pending
                  ? 'border-gray-200 text-gray-500 hover:bg-gray-50 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-800'
                  : 'cursor-not-allowed border-gray-200 bg-gray-50 text-gray-300 dark:border-gray-700 dark:bg-gray-900 dark:text-gray-600',
              )}
              aria-label="上传附件"
              title={canUploadImages ? '上传附件' : '当前模型不支持图像输入'}
            >
              <Plus className="h-4 w-4" strokeWidth={2.4} />
            </button>
            <div className="relative" ref={imageToolPanelRef}>
              <button
                type="button"
                onClick={() => setImageToolMenuOpen((open) => !open)}
                className={clsx(
                  'flex h-8 items-center gap-1.5 rounded-full border px-3 text-[13px] font-medium transition-colors',
                  selectedImageToolKey
                    ? selectedImageToolAvailable
                      ? 'border-emerald-200 bg-emerald-50 text-emerald-700 hover:bg-emerald-100 dark:border-emerald-900/60 dark:bg-emerald-950/50 dark:text-emerald-300 dark:hover:bg-emerald-900/40'
                      : 'border-amber-200 bg-amber-50 text-amber-700 hover:bg-amber-100 dark:border-amber-900/60 dark:bg-amber-950/50 dark:text-amber-300 dark:hover:bg-amber-900/40'
                    : 'border-gray-200 text-gray-500 hover:bg-gray-50 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-800',
                )}
                aria-label="Image tools"
              >
                {selectedImageToolKey ? (
                  <ModelAvatar
                    model={
                      selectedImageTool?.iconKey ??
                      resolveModelIconKey(
                        selectedImageToolKey,
                        selectedImageTool?.provider ?? 'openai',
                        selectedImageToolLabel,
                      )
                    }
                    size={16}
                  />
                ) : (
                  <ImagePlus className="h-4 w-4" strokeWidth={2} />
                )}
                <span className="max-w-[32vw] truncate sm:max-w-[132px]">
                  {selectedImageToolKey ? selectedImageToolLabel : '图片工具'}
                </span>
                <ChevronDown className="h-3.5 w-3.5 flex-shrink-0" strokeWidth={2} />
              </button>

              {imageToolMenuOpen && (
                <div className="fixed bottom-[calc(env(safe-area-inset-bottom)+84px)] left-3 z-50 w-[min(60vw,236px)] overflow-hidden rounded-[18px] border border-gray-100 bg-white shadow-[0_16px_36px_rgba(0,0,0,0.12)] dark:border-gray-700 dark:bg-[#1e1e1e] dark:shadow-[0_16px_36px_rgba(0,0,0,0.34)] sm:absolute sm:bottom-full sm:left-0 sm:mb-2 sm:w-[320px] sm:max-w-[calc(100vw-32px)]">
                  <div className="flex h-[40px] items-center gap-2 border-b border-gray-100 px-3 dark:border-gray-800 sm:h-[48px] sm:px-4">
                    <Search className="h-4 w-4 flex-shrink-0 text-gray-400" />
                    <input
                      value={imageToolQuery}
                      onChange={(event) => setImageToolQuery(event.target.value)}
                      placeholder="搜索图片工具..."
                      autoFocus
                      className="flex-1 bg-transparent text-[12px] text-gray-800 outline-none placeholder-gray-400 dark:text-gray-200 sm:text-[14px]"
                    />
                  </div>

                  <div className="border-b border-gray-100 p-2 dark:border-gray-800">
                    <button
                      type="button"
                      onClick={() => {
                        onClearImageTool()
                        setImageToolMenuOpen(false)
                        setImageToolQuery('')
                      }}
                      className={clsx(
                        'flex w-full items-center justify-between rounded-xl px-2.5 py-2 text-left transition-colors hover:bg-gray-50 dark:hover:bg-gray-800',
                        !selectedImageToolKey && 'bg-gray-50 dark:bg-gray-800',
                      )}
                    >
                      <div className="flex min-w-0 flex-col">
                        <span className="text-[12px] font-medium text-gray-800 dark:text-gray-200 sm:text-[14px]">
                          不使用图片工具
                        </span>
                        <span className="mt-0.5 text-[10px] leading-4 text-gray-400 sm:text-[12px] sm:leading-5">
                          当前只进行普通对话，不触发生图工具
                        </span>
                      </div>
                      {!selectedImageToolKey ? (
                        <Check className="h-4 w-4 flex-shrink-0 text-blue-600 dark:text-blue-400" strokeWidth={2.5} />
                      ) : null}
                    </button>
                  </div>

                  <div className="max-h-[min(30vh,220px)] overflow-y-auto p-2 sm:max-h-[360px]">
                    {filteredImageTools.length === 0 && (
                      <div className="px-3 py-5 text-center text-[11px] leading-5 text-gray-400">
                        没有找到匹配的图片工具
                      </div>
                    )}
                    {filteredImageTools.map((tool) => {
                      const active = tool.key === selectedImageToolKey
                      const available = tool.available !== false
                      return (
                        <button
                          key={tool.key}
                          type="button"
                          disabled={!available}
                          title={!available ? tool.unavailableReason ?? '当前不可用' : undefined}
                          className={clsx(
                            'flex w-full items-center justify-between rounded-xl px-2.5 py-2 text-left transition-colors',
                            active
                              ? 'bg-gray-50 dark:bg-gray-800'
                              : 'hover:bg-gray-50 dark:hover:bg-gray-800',
                            !available && 'cursor-not-allowed opacity-60',
                          )}
                          onClick={() => {
                            if (!available) return
                            onSelectImageTool(tool.key)
                            setImageToolMenuOpen(false)
                            setImageToolQuery('')
                          }}
                        >
                          <div className="flex min-w-0 items-center gap-2">
                            <ModelAvatar
                              model={tool.iconKey ?? resolveModelIconKey(tool.key, tool.provider, tool.meta)}
                              size={16}
                            />
                            <div className="flex min-w-0 flex-col">
                              <span className="truncate text-[12px] font-medium text-gray-800 dark:text-gray-200 sm:text-[14px] sm:font-normal">
                                {tool.label}
                              </span>
                              {!available ? (
                                <span className="truncate text-[10px] leading-4 text-amber-600 dark:text-amber-300 sm:text-[12px] sm:leading-5">
                                  {tool.unavailableReason ?? '当前账户不可用'}
                                </span>
                              ) : null}
                            </div>
                          </div>
                          {active ? (
                            <Check
                              className="h-4 w-4 flex-shrink-0 text-blue-600 dark:text-blue-400"
                              strokeWidth={2.5}
                            />
                          ) : null}
                        </button>
                      )
                    })}
                  </div>
                </div>
              )}
            </div>
          </div>

          <div className="flex min-w-0 flex-shrink-0 items-center gap-2 self-auto">
            <div className="relative" ref={modelPanelRef}>
              <button
                type="button"
                onClick={() => setModelMenuOpen((open) => !open)}
                disabled={modelLoading || models.length === 0}
                className={clsx(
                  'flex h-8 min-w-0 max-w-[34vw] items-center gap-1.5 rounded-full border px-3 text-[13px] font-medium transition-colors disabled:opacity-50 sm:max-w-none',
                  selectedModelAvailable
                    ? 'border-gray-200 text-gray-600 hover:bg-gray-50 dark:border-gray-700 dark:text-gray-300 dark:hover:bg-gray-800'
                    : 'border-amber-200 bg-amber-50 text-amber-700 hover:bg-amber-100 dark:border-amber-900/60 dark:bg-amber-950/50 dark:text-amber-300 dark:hover:bg-amber-900/40',
                )}
                title={
                  !selectedModelAvailable
                    ? selectedModel?.unavailableReason ?? '当前模型不可用'
                    : undefined
                }
              >
                <ModelAvatar
                  model={resolveModelIconKey(selectedModelKey, selectedModelProvider, selectedModelLabel)}
                  size={20}
                />
                <span className="min-w-0 max-w-[20vw] truncate sm:max-w-[160px]">{selectedModelLabel}</span>
                <ChevronDown className="h-3.5 w-3.5 flex-shrink-0 text-gray-400" strokeWidth={2} />
              </button>

              {modelMenuOpen && (
                <div className="fixed bottom-[calc(env(safe-area-inset-bottom)+84px)] right-3 z-50 w-[min(58vw,224px)] overflow-hidden rounded-[18px] border border-gray-100 bg-white shadow-[0_16px_36px_rgba(0,0,0,0.12)] dark:border-gray-700 dark:bg-[#1e1e1e] dark:shadow-[0_16px_36px_rgba(0,0,0,0.34)] sm:absolute sm:bottom-full sm:right-0 sm:mb-2 sm:w-[340px] sm:max-w-[calc(100vw-32px)]">
                  <div className="flex h-[40px] items-center gap-2 border-b border-gray-100 px-3 dark:border-gray-800 sm:h-[48px] sm:px-4">
                    <Search className="h-4 w-4 flex-shrink-0 text-gray-400" />
                    <input
                      value={modelQuery}
                      onChange={(event) => setModelQuery(event.target.value)}
                      placeholder="搜索模型..."
                      autoFocus
                      className="flex-1 bg-transparent text-[12px] text-gray-800 outline-none placeholder-gray-400 dark:text-gray-200 sm:text-[14px]"
                    />
                  </div>
                  <div className="max-h-[min(28vh,208px)] overflow-y-auto p-2 sm:max-h-[360px]">
                    {filteredModels.length === 0 && (
                      <div className="px-3 py-5 text-center text-[11px] leading-5 text-gray-400">
                        没有找到匹配的模型
                      </div>
                    )}
                    {filteredModels.map((model) => {
                      const active = model.key === selectedModelKey
                      const available = model.available !== false
                      const typeLabel = modelTypeLabel(model.modelType)
                      return (
                        <button
                          key={model.key}
                          type="button"
                          disabled={!available}
                          className={clsx(
                            'flex w-full items-center justify-between rounded-xl px-2.5 py-2 text-left transition-colors',
                            active
                              ? 'bg-gray-50 dark:bg-gray-800'
                              : 'hover:bg-gray-50 dark:hover:bg-gray-800',
                            !available && 'cursor-not-allowed opacity-60',
                          )}
                          title={!available ? model.unavailableReason ?? '当前模型不可用' : undefined}
                          onClick={() => {
                            if (!available) return
                            onSelectModel(model.key)
                            setModelMenuOpen(false)
                            setModelQuery('')
                          }}
                        >
                          <div className="flex min-w-0 items-center gap-2">
                            <ModelAvatar
                              model={model.iconKey ?? resolveModelIconKey(model.key, model.provider, model.meta)}
                              size={16}
                            />
                            <div className="flex min-w-0 flex-col">
                              <div className="flex min-w-0 items-center gap-2">
                                <span className="truncate text-[12px] font-medium text-gray-800 dark:text-gray-200 sm:text-[14px] sm:font-normal">
                                  {model.label}
                                </span>
                                {typeLabel ? (
                                  <span className="rounded-full border border-gray-200 px-1.5 py-0.5 text-[9px] text-gray-500 dark:border-gray-700 dark:text-gray-400 sm:px-2 sm:text-[11px]">
                                    {typeLabel}
                                  </span>
                                ) : null}
                              </div>
                              {!available ? (
                                <span className="truncate text-[10px] leading-4 text-amber-600 dark:text-amber-300 sm:text-[12px] sm:leading-5">
                                  {model.unavailableReason ?? '当前账户不可用'}
                                </span>
                              ) : null}
                            </div>
                          </div>
                          {active ? (
                            <Check
                              className="h-4 w-4 flex-shrink-0 text-blue-600 dark:text-blue-400"
                              strokeWidth={2.5}
                            />
                          ) : null}
                        </button>
                      )
                    })}
                  </div>
                </div>
              )}
            </div>

            <button
              type="submit"
              disabled={!canSend && !pending}
              className={clsx(
                'flex h-8 w-8 items-center justify-center rounded-full transition-all',
                pending || canSend
                  ? 'bg-black text-white hover:opacity-80 dark:bg-white dark:text-black'
                  : 'cursor-not-allowed bg-gray-100 text-gray-400 dark:bg-gray-800',
              )}
              aria-label={pending ? 'Stop' : 'Send message'}
              title={pending ? '停止生成' : '发送消息'}
            >
              {pending ? (
                <Square className="h-3.5 w-3.5" strokeWidth={2.5} />
              ) : (
                <ArrowUp className="h-4 w-4" strokeWidth={2.5} />
              )}
            </button>
          </div>
        </div>

        {attachmentNoticeVisible ? (
          <div className="border-t border-amber-100 bg-amber-50 px-4 py-2 text-[12px] text-amber-700 dark:border-amber-900/40 dark:bg-amber-950/30 dark:text-amber-300">
            当前模型不支持图像输入，请切换到多模态模型或移除图片附件
          </div>
        ) : null}
      </div>


    </form>
  )
}
