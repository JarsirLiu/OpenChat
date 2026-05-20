import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { createMarkdownComponents } from './markdownComponents'

interface MarkdownProps {
  content: string
  className?: string
}

export function Markdown({ content, className }: MarkdownProps) {
  return (
    <div className={className ? `lc-markdown ${className}` : 'lc-markdown'}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={createMarkdownComponents()}>
        {content}
      </ReactMarkdown>
    </div>
  )
}
