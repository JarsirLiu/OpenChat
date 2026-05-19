interface ImageProps {
  url: string
  alt: string
}

export function Image({ url, alt }: ImageProps) {
  return <img className="lc-message-image" src={url} alt={alt} />
}
