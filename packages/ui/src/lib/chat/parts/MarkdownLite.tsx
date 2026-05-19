import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

interface MarkdownLiteProps {
  content: string
}

export function MarkdownLite({ content }: MarkdownLiteProps) {
  return (
    <div className="lc-markdown">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{content}</ReactMarkdown>
    </div>
  )
}
