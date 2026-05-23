const SUPPORTED_IMAGE_MIME_TYPES = ['image/png', 'image/jpeg', 'image/webp'] as const
const SUPPORTED_DOCUMENT_MIME_TYPES = [
  'text/plain',
  'text/markdown',
  'application/pdf',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
] as const

export const SUPPORTED_IMAGE_ACCEPT = '.png,.jpg,.jpeg,.webp,.txt,.md,.markdown,.pdf,.docx'

export const isSupportedImageMimeType = (mimeType: string) =>
  SUPPORTED_IMAGE_MIME_TYPES.includes(mimeType.toLowerCase() as (typeof SUPPORTED_IMAGE_MIME_TYPES)[number])

export const filterSupportedImageFiles = (files: File[]) =>
  files.filter((file) => isSupportedImageMimeType(file.type))

export const isDocumentFile = (file: File) => {
  const mimeType = file.type.toLowerCase()
  const name = file.name.toLowerCase()
  return (
    SUPPORTED_DOCUMENT_MIME_TYPES.includes(mimeType as (typeof SUPPORTED_DOCUMENT_MIME_TYPES)[number]) ||
    name.endsWith('.txt') ||
    name.endsWith('.md') ||
    name.endsWith('.markdown') ||
    name.endsWith('.pdf') ||
    name.endsWith('.docx')
  )
}

export const filterSupportedAttachmentFiles = (files: File[]) =>
  files.filter((file) => isSupportedImageMimeType(file.type) || isDocumentFile(file))

export const getUnsupportedImageMessage = () =>
  '当前支持 PNG、JPG/JPEG、WebP 图片，以及 TXT、Markdown、PDF、DOCX 文档'
