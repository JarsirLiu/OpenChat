import { useEffect, useMemo, useState } from 'react'
import type { ChatRuntimeV2State } from '@openchat/chat-core'
import type { ChatMessage, MessageContentPart, ThreadItem, ThreadTurn } from '@openchat/protocol'
import { Bot, ChevronDown, Download, ImageIcon, Sparkles, Loader2, AlertCircle } from 'lucide-react'
import { AssistantActionsBar } from './chat/AssistantActionsBar'
import { AssistantFrame } from './chat/AssistantFrame'
import { ContentLoading } from './chat/parts/ContentLoading'
import { Image } from './chat/parts/Image'
import { Reasoning } from './chat/parts/Reasoning'
import { Text } from './chat/parts/Text'
import { getImageExtension, getMediaUrl } from './chat/mediaUrl'

interface ThreadConversationProps {
  state: ChatRuntimeV2State
  optimisticUserPreviews?: Record<
    string,
    {
      sessionId: string
      text: string
      createdAt: string
    }
  >
}

const isTextPart = (part: MessageContentPart): part is Extract<MessageContentPart, { type: 'text' }> =>
  part.type === 'text'

const isImagePart = (
  part: MessageContentPart,
): part is Extract<MessageContentPart, { type: 'image' }> => part.type === 'image'

const getUserText = (item: Extract<ThreadItem, { type: 'userMessage' }>) =>
  item.content.filter(isTextPart)

const getUserImages = (item: Extract<ThreadItem, { type: 'userMessage' }>) =>
  item.content.filter(isImagePart)

const getReasoningText = (item: Extract<ThreadItem, { type: 'reasoning' }>) =>
  item.content.join('\n\n').trim()

const isUserMessageItem = (item: ThreadItem): item is Extract<ThreadItem, { type: 'userMessage' }> =>
  item.type === 'userMessage'

const isAssistantSideItem = (item: ThreadItem): item is Exclude<ThreadItem, { type: 'userMessage' }> =>
  item.type !== 'userMessage'

const isAgentMessageItem = (
  item: ThreadItem,
): item is Extract<ThreadItem, { type: 'agentMessage' }> => item.type === 'agentMessage'

const toAssistantChatMessage = (
  item: Extract<ThreadItem, { type: 'agentMessage' }>,
): ChatMessage => ({
  id: item.id,
  role: 'assistant',
  turnId: item.turnId,
  status: item.status,
  content: item.text.trim()
    ? [
        {
          type: 'text',
          text: item.text,
        },
      ]
    : [],
  createdAt: item.createdAt,
  updatedAt: item.updatedAt,
})

const formatDuration = (durationMs: number | null) => {
  if (durationMs == null || durationMs < 0) {
    return null
  }

  if (durationMs < 1000) {
    return `${durationMs} ms`
  }

  return `${(durationMs / 1000).toFixed(durationMs >= 10_000 ? 0 : 1)} s`
}

const parseTimestamp = (value?: string) => {
  if (!value) {
    return null
  }

  const parsed = Date.parse(value)
  return Number.isNaN(parsed) ? null : parsed
}

const IMAGE_GEN_LOADING_MESSAGES = ['正在构思作图', '正在尝试作图', '图片快做好了'] as const

const createDownloadBaseName = (itemId: string) => {
  const normalizedId = itemId.replace(/[^a-zA-Z0-9_-]/g, '').slice(-12)
  return normalizedId ? `openchat-image-${normalizedId}` : `openchat-image-${Date.now()}`
}

const downloadImageAsset = async (url: string, filename: string) => {
  try {
    const response = await fetch(url, { credentials: 'same-origin' })
    if (!response.ok) {
      throw new Error(`download failed: ${response.status}`)
    }

    const blob = await response.blob()
    const objectUrl = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = objectUrl
    anchor.download = filename
    anchor.click()
    window.setTimeout(() => URL.revokeObjectURL(objectUrl), 1000)
    return
  } catch {
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = filename
    anchor.target = '_blank'
    anchor.rel = 'noreferrer'
    anchor.click()
  }
}

const AssistantAvatar = () => (
  <div className="flex h-8 w-8 items-center justify-center overflow-hidden rounded-full border border-gray-100 bg-white shadow-sm dark:border-gray-800 dark:bg-gray-900">
    <img
      src="/openchat-logo-3d.webp"
      alt="OpenChat"
      className="h-6 w-6 object-contain"
      onError={(event) => {
        event.currentTarget.style.display = 'none'
        event.currentTarget.nextElementSibling?.classList.remove('hidden')
      }}
    />
    <Bot className="hidden h-4 w-4 text-gray-700 dark:text-gray-300" />
  </div>
)

const ImageGenerationBlock = ({
  item,
}: {
  item: Extract<ThreadItem, { type: 'imageGeneration' }>
}) => {
  const [now, setNow] = useState(() => Date.now())
  const [expanded, setExpanded] = useState(true)
  const [downloadingUrl, setDownloadingUrl] = useState<string | null>(null)
  const startedAtMs = parseTimestamp(item.createdAt)
  const completedAtMs = parseTimestamp(item.updatedAt)
  const downloadBaseName = createDownloadBaseName(item.id)
  const durationText = useMemo(() => {
    if (!startedAtMs) {
      return null
    }

    if (item.status === 'in_progress') {
      return formatDuration(Math.max(0, now - startedAtMs))
    }

    if (completedAtMs) {
      return formatDuration(Math.max(0, completedAtMs - startedAtMs))
    }

    return null
  }, [completedAtMs, item.status, now, startedAtMs])
  const loadingStatusText = useMemo(() => {
    if (item.status !== 'in_progress' || !startedAtMs) {
      return IMAGE_GEN_LOADING_MESSAGES[0]
    }

    const step = Math.floor(Math.max(0, now - startedAtMs) / 2400)
    return IMAGE_GEN_LOADING_MESSAGES[step % IMAGE_GEN_LOADING_MESSAGES.length]
  }, [item.status, now, startedAtMs])

  useEffect(() => {
    if (item.status !== 'in_progress' || !startedAtMs) {
      return
    }

    const timer = window.setInterval(() => {
      setNow(Date.now())
    }, 200)

    return () => {
      window.clearInterval(timer)
    }
  }, [item.status, startedAtMs])

  useEffect(() => {
    if (item.status === 'in_progress') {
      setExpanded(true)
    }
  }, [item.status])

  const handleDownload = async (asset: (typeof item.images)[number], index: number) => {
    const url = getMediaUrl(asset)
    const extension = getImageExtension(asset)
    setDownloadingUrl(url)
    try {
      await downloadImageAsset(url, `${downloadBaseName}-${index + 1}.${extension}`)
    } finally {
      setDownloadingUrl((current) => (current === url ? null : current))
    }
  }

  return (
    <div className="lc-image-gen-shell">
      <button
        type="button"
        className="lc-image-gen-summary"
        onClick={() => setExpanded((open) => !open)}
        aria-expanded={expanded}
      >
        <div className="lc-image-gen-summary-main">
          <span className="lc-image-gen-summary-icon">
            <Sparkles size={13} />
          </span>
          <span className="lc-image-gen-summary-title">图片生成</span>
          <span className="lc-image-gen-summary-status">
            {item.status === 'failed'
              ? '失败'
              : item.status === 'in_progress'
                ? '正在绘制...'
                : item.images.length > 0
                  ? `${item.images.length} 张图片`
                  : '已完成'}
          </span>
        </div>
        <div className="lc-image-gen-summary-side">
          {durationText ? <span className="lc-image-gen-summary-time">{durationText}</span> : null}
          <ChevronDown
            className={`lc-image-gen-summary-chevron ${expanded ? 'is-open' : ''}`}
            size={14}
            strokeWidth={2.5}
          />
        </div>
      </button>

      {expanded ? (
        <div className="lc-image-gen-body">
          {item.images.length > 0 ? (
            <div className={`lc-image-gen-grid ${item.images.length === 1 ? 'is-single' : ''}`}>
              {item.images.map((asset, index) => (
                <div
                  key={`${asset.objectKey ?? asset.url}:${index}`}
                  className="lc-image-gen-image-tile"
                >
                  <a
                    className="lc-image-gen-image-link"
                    href={getMediaUrl(asset)}
                    target="_blank"
                    rel="noreferrer"
                  >
                    <img
                      className="lc-image-gen-image"
                      src={getMediaUrl(asset)}
                      alt={`image result ${index + 1}`}
                    />
                  </a>
                  <button
                    type="button"
                    className="lc-image-gen-download-button"
                    onClick={() => void handleDownload(asset, index)}
                    disabled={downloadingUrl === getMediaUrl(asset)}
                    aria-label={`下载第 ${index + 1} 张原图`}
                  >
                    <Download size={14} />
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <div className={`lc-image-gen-placeholder ${item.status === 'failed' ? 'is-failed' : 'is-loading'}`}>
              {item.status === 'failed' ? (
                <div className="lc-image-gen-failed-card">
                  <div className="lc-image-gen-failed-icon">
                    <ImageIcon size={22} />
                  </div>
                  <div className="lc-image-gen-failed-text">图片生成失败</div>
                </div>
              ) : (
                <>
                  <div className="lc-image-gen-placeholder-glow" />
                  <div className="lc-image-gen-loading-card">
                    <div className="lc-image-gen-spinner-container">
                      <Loader2 className="lc-image-gen-loading-spinner animate-spin" size={24} />
                    </div>
                    <div className="lc-image-gen-loading-timer">{durationText || '0.0 s'}</div>
                    <div className="lc-image-gen-loading-text">{loadingStatusText}</div>
                  </div>
                </>
              )}
            </div>
          )}
        </div>
      ) : null}
    </div>
  )
}

const renderAssistantItem = (item: ThreadItem) => {
  if (item.type === 'reasoning') {
    return (
      <Reasoning
        key={item.id}
        reasoning={getReasoningText(item)}
        isActive={item.status === 'in_progress'}
      />
    )
  }

  if (item.type === 'agentMessage') {
    return (
      <div key={item.id} className="lc-message-content text-[14px] leading-relaxed text-gray-800 dark:text-gray-200">
        <Text
          text={item.text}
          isCreatedByUser={false}
          showCursor={item.status === 'in_progress'}
        />
      </div>
    )
  }

  if (item.type === 'assistantPlaceholder') {
    return <ContentLoading key={item.id} label="准备响应中" startedAt={item.createdAt ?? undefined} />
  }

  if (item.type === 'imageGeneration') {
    return <ImageGenerationBlock key={item.id} item={item} />
  }

  return null
}

const getTurnFailureMessage = (turn: ThreadTurn) => {
  if (turn.status !== 'failed' && turn.status !== 'interrupted') {
    return null
  }

  return turn.terminalReason?.message?.trim() ||
    (turn.status === 'interrupted' ? '响应已中止' : '响应失败，请重试')
}

export function ThreadConversation({
  state,
  optimisticUserPreviews = {},
}: ThreadConversationProps) {
  const turns = [...state.turns]
  const hasTimeline = turns.length > 0

  if (!hasTimeline) {
    return (
      <div className="flex w-full flex-col space-y-5 px-2 py-3 lg:space-y-6 lg:px-0 lg:py-4" />
    )
  }

  return (
    <div className="flex w-full flex-col space-y-5 px-2 py-3 lg:space-y-6 lg:px-0 lg:py-4">
      {turns.map((turn) => {
        const orderedItems = [...turn.items].sort((left, right) => left.seq - right.seq)
        const userItems = orderedItems.filter(isUserMessageItem)
        const assistantItems = orderedItems.filter(isAssistantSideItem)
        const latestAgentMessage = [...assistantItems].reverse().find(isAgentMessageItem) ?? null
        const hasStreamingTextOrReasoning = assistantItems.some(
          (item) =>
            item.status === 'in_progress' &&
            (item.type === 'agentMessage' || item.type === 'reasoning'),
        )
        const turnFailureMessage = getTurnFailureMessage(turn)

        return (
          <div key={turn.id} className="flex w-full flex-col">
            {userItems.length === 0 && optimisticUserPreviews[turn.id]?.text ? (
              <article
                key={`optimistic-user:${turn.id}`}
                className="flex w-full px-2 py-2 lg:px-0"
              >
                <div className="ml-auto flex max-w-[82%] flex-col items-end gap-2 lg:max-w-[75%]">
                  <div className="lc-user-message-bubble rounded-2xl bg-gray-100 px-4 py-2.5 text-[14px] leading-relaxed text-gray-900 shadow-sm dark:bg-gray-800 dark:text-gray-100">
                    <div className="lc-message-content">
                      <Text
                        text={optimisticUserPreviews[turn.id].text}
                        isCreatedByUser
                        showCursor={false}
                      />
                    </div>
                  </div>
                </div>
              </article>
            ) : null}
            {userItems.map((item) => {
              const textParts = getUserText(item)
              const imageParts = getUserImages(item)
              return (
                <article key={item.id} className="flex w-full px-2 py-2 lg:px-0">
                  <div className="ml-auto flex max-w-[82%] flex-col items-end gap-2 lg:max-w-[75%]">
                    {imageParts.length > 0 ? (
                      <div className="lc-user-message-images">
                        {imageParts.map((part, index) => (
                          <Image key={`${item.id}:image:${index}`} url={part.url} alt={part.alt} />
                        ))}
                      </div>
                    ) : null}
                    <div className="lc-user-message-bubble rounded-2xl bg-gray-100 px-4 py-2.5 text-[14px] leading-relaxed text-gray-900 shadow-sm dark:bg-gray-800 dark:text-gray-100">
                      <div className="lc-message-content">
                        {textParts.map((part, index) => (
                          <Text
                            key={`${item.id}:text:${index}`}
                            text={part.text}
                            isCreatedByUser
                            showCursor={item.status === 'in_progress'}
                          />
                        ))}
                      </div>
                    </div>
                  </div>
                </article>
              )
            })}
            {assistantItems.length > 0 || turn.status === 'running' || turnFailureMessage ? (
              <AssistantFrame
                footer={
                  latestAgentMessage ? (
                    <div className="mt-1">
                      <AssistantActionsBar message={toAssistantChatMessage(latestAgentMessage)} />
                    </div>
                  ) : null
                }
              >
                {turn.status === 'running' &&
                assistantItems.length === 0 &&
                !hasStreamingTextOrReasoning ? (
                  <ContentLoading label="准备响应中" startedAt={turn.startedAt ?? undefined} />
                ) : null}
                {assistantItems.map((item) => renderAssistantItem(item))}
                {turnFailureMessage ? (
                  <div className="flex items-start gap-2 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] leading-relaxed text-red-700 dark:border-red-900/50 dark:bg-red-950/30 dark:text-red-200">
                    <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
                    <span>{turnFailureMessage}</span>
                  </div>
                ) : null}
              </AssistantFrame>
            ) : null}
          </div>
        )
      })}
    </div>
  )
}
