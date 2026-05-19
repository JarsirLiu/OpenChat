import { Markdown } from './Markdown'
import { MarkdownLite } from './MarkdownLite'
import { EmptyText } from './EmptyText'

interface TextProps {
  text: string
  isCreatedByUser: boolean
  showCursor: boolean
}

export function Text({ text, isCreatedByUser, showCursor }: TextProps) {
  const trimmed = text.trim()

  if (!trimmed && showCursor) {
    return <EmptyText />
  }

  if (isCreatedByUser) {
    return <MarkdownLite content={text} />
  }

  return <Markdown content={text} />
}
