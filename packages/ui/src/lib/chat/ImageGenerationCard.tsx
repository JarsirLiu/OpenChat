import type { ToolCallViewModel } from '@openchat/chat-core'
import type { ToolCallSummary } from '@openchat/protocol'
import { Download, ImageIcon, Sparkles } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'

import { getToolMedia } from './toolCallMeta'
import { getImageExtension, getMediaUrl } from './mediaUrl'

interface ImageGenerationCardProps {
  toolCall: ToolCallSummary
  liveState?: ToolCallViewModel
  startedAt?: string
  completedAt?: string
}

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

const sanitizeDownloadName = (value: string) =>
  value
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001F]/g, '-')
    .replace(/\s+/g, '-')
    .slice(0, 60) || 'openchat-image'

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

const parseArguments = (argumentsText?: string) => {
  if (!argumentsText?.trim()) {
    return null
  }

  try {
    const parsed = JSON.parse(argumentsText) as Record<string, unknown>
    return parsed && typeof parsed === 'object' ? parsed : null
  } catch {
    return null
  }
}

const readPrompt = (toolCall: ToolCallSummary, liveState?: ToolCallViewModel) => {
  const parsed = parseArguments(liveState?.argumentsText ?? toolCall.argumentsText)
  const prompt = parsed?.prompt
  return typeof prompt === 'string' ? prompt.trim() : ''
}

const readMeta = (toolCall: ToolCallSummary, liveState?: ToolCallViewModel) => {
  const parsed = parseArguments(liveState?.argumentsText ?? toolCall.argumentsText)
  const size = typeof parsed?.size === 'string' ? parsed.size : null
  const quality = typeof parsed?.quality === 'string' ? parsed.quality : null
  const n = typeof parsed?.n === 'number' ? parsed.n : null

  return [size, quality, n ? `${n} 张` : null].filter(Boolean) as string[]
}

export function ImageGenerationCard({
  toolCall,
  liveState,
  startedAt,
  completedAt,
}: ImageGenerationCardProps) {
  const [now, setNow] = useState(() => Date.now())
  const [downloadingUrl, setDownloadingUrl] = useState<string | null>(null)
  const label = toolCall.displayName ?? toolCall.name
  const status = liveState?.status ?? toolCall.status ?? 'completed'
  const prompt = readPrompt(toolCall, liveState)
  const meta = readMeta(toolCall, liveState)
  const images = getToolMedia(toolCall, liveState).filter((item) => item.kind === 'image' && item.url)
  const downloadBaseName = sanitizeDownloadName(prompt || label || 'openchat-image')

  const startedAtMs = parseTimestamp(startedAt)
  const completedAtMs = parseTimestamp(completedAt)
  const durationText = useMemo(() => {
    if (!startedAtMs) {
      return null
    }

    if (status === 'in_progress') {
      return formatDuration(Math.max(0, now - startedAtMs))
    }

    if (completedAtMs) {
      return formatDuration(Math.max(0, completedAtMs - startedAtMs))
    }

    return null
  }, [completedAtMs, now, startedAtMs, status])

  useEffect(() => {
    if (status !== 'in_progress' || !startedAtMs) {
      return
    }

    const timer = window.setInterval(() => {
      setNow(Date.now())
    }, 200)

    return () => {
      window.clearInterval(timer)
    }
  }, [startedAtMs, status])

  const handleDownload = async (asset: (typeof images)[number], index: number) => {
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
    <article className="lc-image-gen-row">
      <div className="lc-image-gen-gutter" aria-hidden="true">
        <div className="lc-image-gen-badge">
          <Sparkles size={15} strokeWidth={2.2} />
        </div>
      </div>
      <div className="lc-image-gen-main">
        <div className="lc-image-gen-heading">
          <span className="lc-image-gen-title">图片生成</span>
          <span className="lc-image-gen-subtitle">{label}</span>
        </div>

        <section className="lc-image-gen-card">
          <div className="lc-image-gen-preview">
            {images.length > 0 ? (
              <div className={`lc-image-gen-grid ${images.length === 1 ? 'is-single' : ''}`}>
                {images.map((asset, index) => (
                  <div key={`${asset.objectKey ?? asset.url}:${index}`} className="lc-image-gen-image-tile">
                    <a
                      className="lc-image-gen-image-link"
                      href={getMediaUrl(asset)}
                      target="_blank"
                      rel="noreferrer"
                    >
                      <img
                        className="lc-image-gen-image"
                        src={getMediaUrl(asset)}
                        alt={`${label} result ${index + 1}`}
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
              <div className="lc-image-gen-placeholder">
                <div className="lc-image-gen-placeholder-icon">
                  <ImageIcon size={22} strokeWidth={2} />
                </div>
                <div className="lc-image-gen-placeholder-text">
                  {status === 'failed'
                    ? '图片生成失败'
                    : status === 'in_progress'
                      ? '正在生成图片'
                      : '等待图片结果'}
                </div>
              </div>
            )}

            {durationText ? <div className="lc-image-gen-timer">{durationText}</div> : null}
          </div>

          {prompt ? (
            <p className="lc-image-gen-prompt" title={prompt}>
              {prompt}
            </p>
          ) : null}

          {meta.length > 0 ? (
            <div className="lc-image-gen-meta">
              {meta.map((item) => (
                <span key={item} className="lc-image-gen-meta-chip">
                  {item}
                </span>
              ))}
            </div>
          ) : null}
        </section>
      </div>
    </article>
  )
}
