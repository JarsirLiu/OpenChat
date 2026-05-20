import { API_ERROR_CODES, CLIENT_ERROR_CODES } from '@openchat/protocol'

export interface ApiErrorPayload {
  code?: string
  message?: string
  category?: string
  retryable?: boolean
}

export class ApiError extends Error {
  status: number
  code: string | null
  category: string | null
  retryable: boolean

  constructor(
    message: string,
    options?: {
      status?: number
      code?: string | null
      category?: string | null
      retryable?: boolean
    },
  ) {
    super(message)
    this.status = options?.status ?? 500
    this.code = options?.code ?? null
    this.category = options?.category ?? null
    this.retryable = options?.retryable ?? false
  }
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value && typeof value === 'object')

export const readApiErrorPayload = async (
  response: Response,
): Promise<ApiErrorPayload | null> => {
  const payload = (await response.json().catch(() => null)) as unknown
  if (!isRecord(payload)) {
    return null
  }

  return {
    code: typeof payload.code === 'string' ? payload.code : undefined,
    message: typeof payload.message === 'string' ? payload.message : undefined,
    category: typeof payload.category === 'string' ? payload.category : undefined,
    retryable: typeof payload.retryable === 'boolean' ? payload.retryable : undefined,
  }
}

export const createApiError = async (
  response: Response,
  fallbackMessage: string,
): Promise<ApiError> => {
  const payload = await readApiErrorPayload(response)

  return new ApiError(payload?.message ?? fallbackMessage, {
    status: response.status,
    code: payload?.code ?? null,
    category: payload?.category ?? null,
    retryable: payload?.retryable ?? response.status >= 500,
  })
}

export const ensureOk = async (
  response: Response,
  fallbackMessage: string,
): Promise<Response> => {
  if (!response.ok) {
    throw await createApiError(response, fallbackMessage)
  }

  return response
}

export const toApiError = (
  error: unknown,
  fallbackMessage: string,
  status = 500,
): ApiError => {
  if (error instanceof ApiError) {
    return error
  }

  if (error instanceof Error) {
    return new ApiError(error.message, { status })
  }

  return new ApiError(fallbackMessage, { status })
}

export const isProviderConfigurationError = (
  error: ApiError | { code?: string | null } | null | undefined,
) =>
  error?.code === API_ERROR_CODES.providerApiKeyRequired.code ||
  error?.code === API_ERROR_CODES.providerAuthenticationFailed.code ||
  error?.code === API_ERROR_CODES.modelUnavailable.code ||
  error?.code === API_ERROR_CODES.toolUnavailable.code

export { API_ERROR_CODES, CLIENT_ERROR_CODES }
