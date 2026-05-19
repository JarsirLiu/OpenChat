export type ToolIconType =
  | 'generic'
  | 'image_gen'
  | 'tool'
  | 'tool_error'
  | 'tool_success'

interface ToolIconProps {
  type: ToolIconType
  isAnimating?: boolean
}

const ICON_MAP: Record<ToolIconType, string> = {
  generic: '◫',
  image_gen: '◩',
  tool: '◫',
  tool_error: '!',
  tool_success: '✓',
}

export function getToolIconType(name: string): ToolIconType {
  if (name === 'image_gen_oai' || name === 'image_edit_oai' || name === 'gemini_image_gen') {
    return 'image_gen'
  }
  return 'generic'
}

export function ToolIcon({ type, isAnimating = false }: ToolIconProps) {
  return (
    <span className={`lc-tool-icon lc-tool-icon-${type} ${isAnimating ? 'is-animating' : ''}`}>
      {ICON_MAP[type]}
    </span>
  )
}
