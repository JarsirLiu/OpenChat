import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { createMarkdownComponents } from './markdownComponents'
import { remarkExposeInvisibleMarkdown } from './markdownSafety'

interface MarkdownLiteProps {
  content: string
}

export function MarkdownLite({ content }: MarkdownLiteProps) {
  return (
    <div className="lc-markdown">
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkExposeInvisibleMarkdown]}
        components={createMarkdownComponents()}
      >
        {content}
      </ReactMarkdown>
    </div>
  )
}
