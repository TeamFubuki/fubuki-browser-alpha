export function parseBridgeResponse<T>(response: string): T {
  const value = JSON.parse(response) as unknown;
  if (
    typeof value === 'object' &&
    value !== null &&
    'ok' in value &&
    (value as { ok: unknown }).ok === false
  ) {
    const message =
      'error' in value &&
      typeof (value as { error: unknown }).error === 'string'
        ? (value as { error: string }).error
        : 'Native bridge request failed';
    throw new Error(message);
  }
  return value as T;
}
