const SUPPORTED_IMAGE_MIME_TYPES = ['image/png', 'image/jpeg', 'image/webp'] as const

export const SUPPORTED_IMAGE_ACCEPT = '.png,.jpg,.jpeg,.webp'

export const isSupportedImageMimeType = (mimeType: string) =>
  SUPPORTED_IMAGE_MIME_TYPES.includes(mimeType.toLowerCase() as (typeof SUPPORTED_IMAGE_MIME_TYPES)[number])

export const filterSupportedImageFiles = (files: File[]) =>
  files.filter((file) => isSupportedImageMimeType(file.type))

export const getUnsupportedImageMessage = () =>
  '当前仅支持 PNG、JPG/JPEG、WebP 图片格式'
