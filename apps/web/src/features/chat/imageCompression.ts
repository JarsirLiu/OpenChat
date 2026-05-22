export const MAX_IMAGE_SIZE = 1600
export const MAX_IMAGE_BYTES = 1536 * 1024

const COMPRESSIBLE_IMAGE_MIME_TYPES = new Set(['image/jpeg', 'image/png', 'image/webp'])
const JPEG_QUALITY = 0.82
const WEBP_QUALITY = 0.82
const MIN_IMAGE_SIZE = 100
const DOWNSCALE_FACTOR = 0.8

const getOutputType = (file: File) =>
  file.type === 'image/webp' ? 'image/webp' : 'image/jpeg'

const getOutputName = (file: File, type: string) => {
  const extension = type === 'image/webp' ? 'webp' : 'jpg'
  const baseName = file.name.replace(/\.[^.]+$/, '')
  return `${baseName || 'image'}.${extension}`
}

const renderCompressedImage = async ({
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

  const quality = type === 'image/webp' ? WEBP_QUALITY : JPEG_QUALITY
  return new Promise<Blob | null>((resolve) => {
    canvas.toBlob(resolve, type, quality)
  })
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

    image.addEventListener('load', async () => {
      try {
        if (shouldSkipCompression(file, image)) {
          resolve(file)
          return
        }

        let nextMaxSize = MAX_IMAGE_SIZE
        let current: File = file
        const outputType = getOutputType(file)

        while (nextMaxSize >= MIN_IMAGE_SIZE) {
          const blob = await renderCompressedImage({
            image,
            maxSize: nextMaxSize,
            type: outputType,
          })

          if (!blob) {
            resolve(file)
            return
          }

          current = new File([blob], getOutputName(file, outputType), { type: outputType })
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
