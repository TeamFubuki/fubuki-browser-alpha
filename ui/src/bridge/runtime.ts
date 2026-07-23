import { validateBridgeResponse } from './validation';

export type NativeQuery = {
  request: string;
  onSuccess: (response: string) => void;
  onFailure: (code: number, message: string) => void;
};

export type NativeQueryExecutor = (query: NativeQuery) => number | void;
export type NativeQueryCanceler = (requestId: number) => void;

export const BRIDGE_TIMEOUT_MS = 10_000;

export function invokeNativeBridge<T>(
  cefQuery: NativeQueryExecutor,
  method: string,
  params: Record<string, unknown> = {},
  timeoutMs = BRIDGE_TIMEOUT_MS,
  cefQueryCancel?: NativeQueryCanceler,
): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    let settled = false;
    let requestId: number | undefined;
    const timeout = globalThis.setTimeout(() => {
      if (settled) return;
      settled = true;
      if (requestId !== undefined) {
        try {
          cefQueryCancel?.(requestId);
        } catch {
          // Cancellation is best-effort; the timeout remains the useful error.
        }
      }
      reject(
        new Error(
          `Native bridge request "${method}" timed out after ${timeoutMs}ms`,
        ),
      );
    }, timeoutMs);
    const finish = (callback: () => void) => {
      if (settled) return;
      settled = true;
      globalThis.clearTimeout(timeout);
      callback();
    };

    try {
      const queryId = cefQuery({
        request: JSON.stringify({
          version: 0,
          bridgeVersion: '1',
          method,
          params,
        }),
        onSuccess: (response) => {
          finish(() => {
            try {
              const parsed = JSON.parse(response) as unknown;
              if (
                typeof parsed === 'object' &&
                parsed !== null &&
                'ok' in parsed &&
                parsed.ok === false
              ) {
                const message =
                  'error' in parsed && typeof parsed.error === 'string'
                    ? parsed.error
                    : 'Native bridge request failed';
                reject(
                  new Error(`Native bridge request "${method}": ${message}`),
                );
                return;
              }
              resolve(validateBridgeResponse(method, parsed) as T);
            } catch (error) {
              const reason =
                error instanceof Error ? error.message : String(error);
              reject(
                new Error(`Bridge response for "${method}" failed: ${reason}`),
              );
            }
          });
        },
        onFailure: (code, message) =>
          finish(() =>
            reject(
              new Error(
                `Native bridge request "${method}" failed: ${code}: ${message}`,
              ),
            ),
          ),
      });
      if (typeof queryId === 'number' && Number.isInteger(queryId)) {
        requestId = queryId;
      }
    } catch (error) {
      finish(() => reject(error));
    }
  });
}

export function notifyBridgeListeners(
  eventName: string,
  listeners: Iterable<(payload: unknown) => void>,
  payload: unknown,
  reportError: (message: string, error: unknown) => void,
): void {
  for (const listener of listeners) {
    try {
      listener(payload);
    } catch (error) {
      reportError(`Event listener failed for "${eventName}"`, error);
    }
  }
}
