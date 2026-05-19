import type { ChatMessage } from '@openchat/protocol'
import { Image } from './parts/Image'
import { Text } from './parts/Text'

interface ToolResultBlockProps {
  message: ChatMessage
}

export function ToolResultBlock({ message }: ToolResultBlockProps) {
  const statusLabel = message.status === 'failed' ? '工具执行失败' : '工具输出'

  return (
    <div className="lc-tool-result">
      <div className="lc-tool-result-label">{statusLabel}</div>
      <div className="lc-tool-result-content">
        {message.content.length === 0 ? (
          <p className="lc-notice lc-notice-muted lc-notice-inline">暂无工具输出。</p>
        ) : (
          message.content.map((part, index) =>
            part.type === 'text' ? (
              <div key={`${message.id}:${index}`}>
                <Text text={part.text} isCreatedByUser={false} showCursor={false} />
              </div>
            ) : (
              <Image key={`${message.id}:${index}`} url={part.url} alt={part.alt} />
            ),
          )
        )}
      </div>
    </div>
  )
}
