import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { createMarkdownComponents } from './markdownComponents'

interface MarkdownLiteProps {
  content: string
}

export function MarkdownLite({ content }: MarkdownLiteProps) {
  return (
    <div className="lc-markdown">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={createMarkdownComponents()}>
        {content}
      </ReactMarkdown>
    </div>
  )
}
