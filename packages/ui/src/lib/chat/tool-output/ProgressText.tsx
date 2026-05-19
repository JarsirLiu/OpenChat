import { ChevronDown } from 'lucide-react'

interface ProgressTextProps {
  label: string
  status: string
  isExpanded: boolean
  onClick: () => void
}

export function ProgressText({ label, status, isExpanded, onClick }: ProgressTextProps) {
  const statusLabel =
    status === 'in_progress' ? '调用中' : status === 'failed' ? '失败' : '已完成'

  return (
    <button
      type="button"
      className="lc-progress-text"
      role="button"
      onClick={onClick}
      aria-expanded={isExpanded}
    >
      <span className="lc-progress-label">{label}</span>
      <span className="text-xs text-text-secondary">{statusLabel}</span>
      <ChevronDown
        className={`lc-progress-chevron ${isExpanded ? 'is-open' : ''}`}
        aria-hidden="true"
      />
    </button>
  )
}
