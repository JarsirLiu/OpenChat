import { useEffect, useState } from 'react'

interface ContentLoadingProps {
  label?: string
  startedAt?: string
  showDots?: boolean
}

const ELAPSED_THRESHOLD_SECONDS = 2

export function ContentLoading({
  label = '正在生成回答',
  startedAt,
  showDots = true,
}: ContentLoadingProps) {
  const [elapsedSeconds, setElapsedSeconds] = useState(0)

  useEffect(() => {
    if (!startedAt) {
      setElapsedSeconds(0)
      return
    }

    const started = Date.parse(startedAt)
    if (Number.isNaN(started)) {
      setElapsedSeconds(0)
      return
    }

    const updateElapsed = () => {
      setElapsedSeconds(Math.max(0, Math.floor((Date.now() - started) / 1000)))
    }

    updateElapsed()
    const interval = window.setInterval(updateElapsed, 1000)

    return () => window.clearInterval(interval)
  }, [startedAt])

  return (
    <div className="lc-content-loading" aria-live="polite">
      <span className="lc-content-loading-label">{label}</span>
      {showDots ? (
        <span className="lc-content-loading-dots" aria-hidden="true">
          <span className="lc-loading-dot" />
          <span className="lc-loading-dot" style={{ animationDelay: '120ms' }} />
          <span className="lc-loading-dot" style={{ animationDelay: '240ms' }} />
        </span>
      ) : null}
      {elapsedSeconds >= ELAPSED_THRESHOLD_SECONDS ? (
        <span className="lc-content-loading-elapsed">({elapsedSeconds}s)</span>
      ) : null}
    </div>
  )
}
