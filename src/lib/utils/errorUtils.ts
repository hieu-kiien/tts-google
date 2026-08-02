import type { AppErrorCode, CommandError } from "../types/tts";

const VALID_APP_ERROR_CODES: Set<string> = new Set([
  'AUTH_INVALID',
  'RATE_LIMITED',
  'DAILY_QUOTA_EXHAUSTED',
  'NETWORK_UNAVAILABLE',
  'VALIDATION_FAILED',
  'DATABASE_ERROR',
  'AUDIO_CORRUPT',
  'CONTENT_FILTERED',
  'FILE_SYSTEM_ERROR',
  'QUEUE_ERROR',
  'INTERNAL_ERROR',
]);

function toAppErrorCode(codeStr: unknown): AppErrorCode {
  if (typeof codeStr === 'string' && VALID_APP_ERROR_CODES.has(codeStr)) {
    return codeStr as AppErrorCode;
  }
  return 'INTERNAL_ERROR';
}

/**
 * Safely extract error message from unknown caught errors.
 */
export function getErrorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  if (err && typeof err === 'object') {
    const obj = err as Record<string, unknown>;
    if (typeof obj.message === 'string') return obj.message;
  }
  return String(err);
}

/**
 * Parse structured CommandError object returned from Tauri IPC AppError.
 */
export function parseCommandError(err: unknown): CommandError {
  if (err && typeof err === 'object') {
    const obj = err as Record<string, unknown>;
    if (typeof obj.code === 'string' && typeof obj.message === 'string') {
      return {
        code: toAppErrorCode(obj.code),
        message: obj.message,
        retryable: typeof obj.retryable === 'boolean' ? obj.retryable : false,
        diagnostic_id: typeof obj.diagnostic_id === 'string' ? obj.diagnostic_id : null,
      };
    }
    if (typeof obj.message === 'string') {
      return {
        code: "INTERNAL_ERROR",
        message: obj.message,
        retryable: false,
      };
    }
  }
  if (typeof err === 'string') {
    return {
      code: "INTERNAL_ERROR",
      message: err,
      retryable: false,
    };
  }
  return {
    code: "INTERNAL_ERROR",
    message: String(err),
    retryable: false,
  };
}
