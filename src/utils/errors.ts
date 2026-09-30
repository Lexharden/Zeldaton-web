import { t } from '@/i18n'
import { ApiError } from '@/services/api/RaceApi'

export function errorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.kind === 'timeout') return t('states.timeout')
    if (error.kind === 'network') return t('states.network')
    return error.message
  }
  return error instanceof Error ? error.message : t('states.unknownError')
}
