interface ToolCallInfoProps {
  input?: string
  output?: string
  status?: string
}

export function ToolCallInfo({ input, output, status }: ToolCallInfoProps) {
  const isTerminalFailure = status === 'failed'

  return (
    <div className="lc-tool-call-info">
      {input ? (
        <>
          <p className="lc-tool-section-title">Arguments</p>
          <pre className="lc-tool-code">{input}</pre>
        </>
      ) : null}
      {output ? (
        <>
          <p className="lc-tool-section-title">Result</p>
          <pre className="lc-tool-code">{output}</pre>
        </>
      ) : isTerminalFailure ? (
        <p className="lc-notice lc-notice-error lc-notice-inline">Tool call failed.</p>
      ) : (
        <p className="lc-notice lc-notice-muted lc-notice-inline">Waiting for tool output…</p>
      )}
    </div>
  )
}
