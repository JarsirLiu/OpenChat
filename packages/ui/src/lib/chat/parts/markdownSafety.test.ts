import { describe, expect, it } from 'vitest'
import { remarkExposeInvisibleMarkdown } from './markdownSafety'

describe('remarkExposeInvisibleMarkdown', () => {
  it('keeps link definitions available as visible paragraphs', () => {
    const tree = {
      type: 'root',
      children: [
        {
          type: 'definition',
          identifier: 'source',
          label: 'source',
          url: 'https://example.com',
          title: 'Example',
        },
        {
          type: 'paragraph',
          children: [{ type: 'text', value: '正文' }],
        },
      ],
    }

    remarkExposeInvisibleMarkdown()(tree)

    expect(tree.children).toEqual([
      {
        type: 'definition',
        identifier: 'source',
        label: 'source',
        url: 'https://example.com',
        title: 'Example',
      },
      {
        type: 'paragraph',
        children: [{ type: 'text', value: '[source]: https://example.com "Example"' }],
      },
      {
        type: 'paragraph',
        children: [{ type: 'text', value: '正文' }],
      },
    ])
  })

  it('does not rewrite already visible text nodes', () => {
    const tree = {
      type: 'root',
      children: [
        {
          type: 'paragraph',
          children: [{ type: 'text', value: '地方性法规由本级制定。' }],
        },
      ],
    }

    remarkExposeInvisibleMarkdown()(tree)

    expect(tree.children).toEqual([
      {
        type: 'paragraph',
        children: [{ type: 'text', value: '地方性法规由本级制定。' }],
      },
    ])
  })
})
