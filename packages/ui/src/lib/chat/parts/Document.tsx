import { FileText } from 'lucide-react'

const formatFileSize = (bytes: number) => {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return ''
  }
  if (bytes < 1024) {
    return `${bytes} B`
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`
  }
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

const getDocumentTypeLabel = (mimeType: string) => {
  const normalized = mimeType.toLowerCase()
  if (normalized.includes('pdf')) return 'PDF'
  if (normalized.includes('wordprocessingml')) return 'DOCX'
  if (normalized.startsWith('text/markdown')) return 'MD'
  if (normalized.startsWith('text/')) return 'TXT'
  return 'FILE'
}

export function Document({
  url,
  name,
  mimeType,
  sizeBytes,
}: {
  url: string
  name: string
  mimeType: string
  sizeBytes: number
}) {
  const size = formatFileSize(sizeBytes)

  return (
    <a
      href={url}
      target="_blank"
      rel="noreferrer"
      className="flex max-w-full items-center gap-2 rounded-xl border border-gray-200 bg-white px-3 py-2 text-left text-gray-800 shadow-sm transition hover:bg-gray-50 dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100 dark:hover:bg-gray-800"
    >
      <span className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg bg-blue-50 text-blue-600 dark:bg-blue-950/60 dark:text-blue-300">
        <FileText className="h-4 w-4" strokeWidth={2.2} />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[13px] font-medium">{name}</span>
        <span className="mt-0.5 block text-[11px] text-gray-500 dark:text-gray-400">
          {[getDocumentTypeLabel(mimeType), size].filter(Boolean).join(' · ')}
        </span>
      </span>
    </a>
  )
}
