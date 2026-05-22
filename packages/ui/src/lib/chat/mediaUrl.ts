type MediaReference = {
  url: string
  objectKey?: string | null
  mimeType?: string | null
}

const encodeObjectKey = (objectKey: string) =>
  objectKey
    .replace(/\\/g, '/')
    .replace(/^\/+/, '')
    .replace(/^api\/media\//, '')
    .split('/')
    .filter(Boolean)
    .map(encodeURIComponent)
    .join('/')

export const getMediaUrl = (media: MediaReference) => {
  const objectKey = media.objectKey?.trim()
  if (objectKey) {
    return `/api/media/${encodeObjectKey(objectKey)}`
  }

  return media.url
}

export const getImageExtension = (media: MediaReference) => {
  const mimeType = media.mimeType?.trim().toLowerCase()
  if (mimeType === 'image/jpeg' || mimeType === 'image/jpg') {
    return 'jpg'
  }
  if (mimeType === 'image/webp') {
    return 'webp'
  }
  if (mimeType === 'image/gif') {
    return 'gif'
  }

  return media.url.match(/\.([a-zA-Z0-9]+)(?:[?#]|$)/)?.[1] ?? 'png'
}
