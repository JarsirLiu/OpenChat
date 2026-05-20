import errorCodes from '../../../config/error-codes.json'

type ErrorCodeSpec = {
  code: string
  category: string
  retryable: boolean
}

type ErrorCodeCatalog = {
  apiErrors: Record<string, ErrorCodeSpec>
  clientErrors: Record<string, ErrorCodeSpec>
}

const catalog = errorCodes as ErrorCodeCatalog

export const API_ERROR_CODES = catalog.apiErrors
export const CLIENT_ERROR_CODES = catalog.clientErrors

export type ApiErrorSpecKey = keyof typeof API_ERROR_CODES
export type ClientErrorSpecKey = keyof typeof CLIENT_ERROR_CODES
