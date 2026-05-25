const SUPPORTED_IMAGE_MIME_TYPES = ['image/png', 'image/jpeg', 'image/webp'] as const
const SUPPORTED_DOCUMENT_MIME_TYPES = [
  'text/plain',
  'text/markdown',
  'application/pdf',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
] as const

export const SUPPORTED_IMAGE_ACCEPT = '.png,.jpg,.jpeg,.webp,.txt,.md,.markdown,.pdf,.docx'
export const MAX_UPLOAD_BYTES = 100 * 1024 * 1024
export const MAX_UPLOAD_REQUEST_BYTES = 120 * 1024 * 1024
export const MAX_FILES_PER_UPLOAD = 12

export const formatUploadSizeLimit = (bytes = MAX_UPLOAD_BYTES) =>
  `${Math.floor(bytes / 1024 / 1024)}MB`

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

export const getTooManyFilesMessage = () =>
  `一次最多上传 ${MAX_FILES_PER_UPLOAD} 个文件，请分批上传`

export const getFileTooLargeMessage = (fileName: string, bytes = MAX_UPLOAD_BYTES) =>
  `文件「${fileName}」过大，单个文件请控制在 ${formatUploadSizeLimit(bytes)} 以内；文档内容会自动截断到模型上下文范围`

export const getUploadRequestTooLargeMessage = (bytes = MAX_UPLOAD_REQUEST_BYTES) =>
  `本次上传文件总量过大，请控制在 ${formatUploadSizeLimit(bytes)} 以内或分批上传`
