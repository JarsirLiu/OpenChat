import type { ToolMedia } from '@openchat/protocol'

import { ToolIcon, getToolIconType } from './ToolIcon'
import { ProgressText } from './ProgressText'
import { ToolCallInfo } from './ToolCallInfo'

interface ImageGenProps {
  toolName: string
  argumentsText: string
  resultText: string
  media: ToolMedia[]
  status: string
  isExpanded: boolean
  onToggle: () => void
}

export function ImageGen({
  toolName,
  argumentsText,
  resultText,
  media,
  status,
  isExpanded,
  onToggle,
}: ImageGenProps) {
  const iconType = getToolIconType(toolName)
  const isInProgress = status === 'in_progress'
  const imageMedia = media.filter((asset) => asset.kind === 'image' && asset.url)

  return (
    <section className="lc-tool-card lc-image-tool-card">
      <div className="lc-tool-summary">
        <ToolIcon type={iconType} isAnimating={isInProgress} />
        <ProgressText
          label={toolName}
          status={status}
          isExpanded={isExpanded}
          onClick={onToggle}
        />
      </div>
      {isExpanded ? (
        <div className="lc-tool-body">
          {imageMedia.length > 0 ? (
            <div className="lc-tool-image-grid">
              {imageMedia.map((asset, index) => (
                <a
                  key={`${asset.url}:${index}`}
                  className="lc-tool-image-link"
                  href={asset.url}
                  target="_blank"
                  rel="noreferrer"
                >
                  <img
                    className="lc-tool-image"
                    src={asset.url}
                    alt={`${toolName} result ${index + 1}`}
                  />
                </a>
              ))}
            </div>
          ) : null}
          <ToolCallInfo input={argumentsText} output={resultText} status={status} />
        </div>
      ) : null}
    </section>
  )
}
