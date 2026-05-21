import { isValidElement, type ComponentPropsWithoutRef, type ReactElement, type ReactNode } from 'react'

const getCodeLanguage = (className?: string) => {
  if (!className) {
    return null
  }

  const match = className.match(/language-([a-z0-9#+._-]+)/i)
  return match?.[1]?.toLowerCase() ?? null
}

const getPreLanguage = (children: ReactNode) => {
  if (!isValidElement(children)) {
    return null
  }

  const element = children as ReactElement<{ className?: string }>

  const className =
    typeof element.props.className === 'string'
      ? element.props.className
      : undefined

  return getCodeLanguage(className)
}

export const createMarkdownComponents = () => ({
  a: ({ href, ...props }: ComponentPropsWithoutRef<'a'>) => (
    <a
      {...props}
      href={href}
      target={href?.startsWith('#') ? undefined : '_blank'}
      rel={href?.startsWith('#') ? undefined : 'noreferrer noopener'}
    />
  ),
  img: ({ src, alt, ...props }: ComponentPropsWithoutRef<'img'>) => (
    <img
      {...props}
      src={src}
      alt={alt ?? ''}
      className="lc-message-image"
      loading="lazy"
    />
  ),
  code: ({ className, children, ...props }: ComponentPropsWithoutRef<'code'>) => {
    const language = getCodeLanguage(className)

    if (language) {
      return (
        <code {...props} className={className} data-language={language}>
          {children}
        </code>
      )
    }

    return (
      <code {...props} className={className}>
        {children}
      </code>
    )
  },
  input: (props: ComponentPropsWithoutRef<'input'>) => {
    if (props.type === 'checkbox') {
      return <input {...props} disabled className="lc-markdown-checkbox" />
    }

    return <input {...props} />
  },
  pre: ({ children, ...props }: ComponentPropsWithoutRef<'pre'>) => {
    const language = getPreLanguage(children)

    return (
      <div className="lc-markdown-code-block">
        <div className="lc-markdown-code-header">
          <span className="lc-markdown-code-language">{language ?? 'text'}</span>
        </div>
        <pre {...props}>{children}</pre>
      </div>
    )
  },
  table: (props: ComponentPropsWithoutRef<'table'>) => (
    <div className="lc-markdown-table-wrap">
      <table {...props} />
    </div>
  ),
})
