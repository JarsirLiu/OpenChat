interface MarkdownNode {
  type?: string
  children?: MarkdownNode[]
  identifier?: string
  label?: string
  url?: string
  title?: string | null
  value?: string
}

const textNode = (value: string): MarkdownNode => ({ type: 'text', value })

const paragraphNode = (value: string): MarkdownNode => ({
  type: 'paragraph',
  children: [textNode(value)],
})

const stringifyDefinition = (node: MarkdownNode) => {
  const label = node.label || node.identifier || ''
  const url = node.url || ''
  const title = node.title ? ` "${node.title}"` : ''

  return `[${label}]: ${url}${title}`.trim()
}

const exposeInvisibleNodes = (node: MarkdownNode) => {
  if (!Array.isArray(node.children)) {
    return
  }

  const nextChildren: MarkdownNode[] = []
  for (const child of node.children) {
    nextChildren.push(child)

    if (child.type === 'definition') {
      nextChildren.push(paragraphNode(stringifyDefinition(child)))
      continue
    }

    exposeInvisibleNodes(child)
  }

  node.children = nextChildren
}

export function remarkExposeInvisibleMarkdown() {
  return (tree: MarkdownNode) => {
    exposeInvisibleNodes(tree)
  }
}
