export const MAX_IMAGE_SIZE = 1920
export const MAX_IMAGE_BYTES = 3 * 1024 * 1024

const COMPRESSIBLE_IMAGE_MIME_TYPES = new Set(['image/jpeg', 'image/png', 'image/webp'])
const JPEG_QUALITY = 0.85
const MIN_IMAGE_SIZE = 100
const DOWNSCALE_FACTOR = 0.8

const dataUrlToFile = (dataUrl: string, name: string) => {
  const [header, payload] = dataUrl.split(',')
  const mimeType = header?.split(':')[1]?.split(';')[0] || 'image/png'
  const binary = window.atob(payload || '')
  const bytes = new Uint8Array(binary.length)

  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }

  return new File([bytes], name, { type: mimeType })
}

const renderCompressedImage = ({
  image,
  maxSize,
  type,
}: {
  image: HTMLImageElement
  maxSize: number
  type: string
}) => {
  let width = image.width
  let height = image.height

  if (width > maxSize || height > maxSize) {
    if (width >= height) {
      height = Math.round((maxSize / width) * height)
      width = maxSize
    } else {
      width = Math.round((maxSize / height) * width)
      height = maxSize
    }
  }

  const canvas = document.createElement('canvas')
  const context = canvas.getContext('2d')
  if (!context) {
    return null
  }

  canvas.width = width
  canvas.height = height
  context.drawImage(image, 0, 0, image.width, image.height, 0, 0, width, height)

  if (type === 'image/jpeg') {
    return canvas.toDataURL('image/jpeg', JPEG_QUALITY)
  }

  return canvas.toDataURL('image/png')
}

const shouldSkipCompression = (file: File, image: HTMLImageElement) =>
  image.width <= MAX_IMAGE_SIZE &&
  image.height <= MAX_IMAGE_SIZE &&
  file.size <= MAX_IMAGE_BYTES

export const compressImageFile = async (file: File) =>
  new Promise<File>((resolve) => {
    if (!COMPRESSIBLE_IMAGE_MIME_TYPES.has(file.type)) {
      resolve(file)
      return
    }

    const image = new Image()
    const objectUrl = URL.createObjectURL(file)

    const cleanup = () => {
      URL.revokeObjectURL(objectUrl)
    }

    image.addEventListener('load', () => {
      try {
        if (shouldSkipCompression(file, image)) {
          resolve(file)
          return
        }

        let nextMaxSize = MAX_IMAGE_SIZE
        let current = file

        while (nextMaxSize >= MIN_IMAGE_SIZE) {
          const dataUrl = renderCompressedImage({
            image,
            maxSize: nextMaxSize,
            type: file.type,
          })

          if (!dataUrl) {
            resolve(file)
            return
          }

          current = dataUrlToFile(dataUrl, file.name)
          if (current.size <= MAX_IMAGE_BYTES) {
            resolve(current)
            return
          }

          nextMaxSize = Math.round(nextMaxSize * DOWNSCALE_FACTOR)
        }

        resolve(current)
      } finally {
        cleanup()
      }
    })

    image.addEventListener('error', () => {
      cleanup()
      resolve(file)
    })

    image.src = objectUrl
  })

export const compressImageFiles = async (files: File[]) =>
  Promise.all(files.map((file) => compressImageFile(file)))
