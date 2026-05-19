export const resolveModelIconKey = (
  modelKey: string | null | undefined,
  provider: string | null | undefined,
  modelName?: string | null,
) => {
  const fingerprint = `${modelName ?? ''} ${modelKey ?? ''} ${provider ?? ''}`.toLowerCase()

  if (fingerprint.includes('claude')) {
    return 'claude'
  }
  if (fingerprint.includes('gemini')) {
    return 'gemini'
  }
  if (fingerprint.includes('gpt') || fingerprint.includes('codex') || fingerprint.includes('openai')) {
    return 'openai'
  }

  return modelKey ?? 'openai'
}
