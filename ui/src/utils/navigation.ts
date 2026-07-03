const SCHEME_PATTERN = /^[a-z][a-z0-9+.-]*:/i;
const HOSTLIKE_PATTERN =
  /^(localhost|(\d{1,3}\.){3}\d{1,3}|\[[0-9a-f:]+\]|([a-z0-9-]+\.)+[a-z]{2,})(:\d+)?(\/|$|\?|#)/i;

export function containsWhitespace(value: string): boolean {
  return /\s/.test(value);
}

export function containsNonAscii(value: string): boolean {
  return /[^\u0000-\u007f]/.test(value);
}

export function shouldTreatAsSearch(input: string): boolean {
  const value = input.trim();
  if (!value) return false;
  if (containsWhitespace(value) || containsNonAscii(value)) return true;
  if (SCHEME_PATTERN.test(value)) return false;
  return !HOSTLIKE_PATTERN.test(value);
}

export function normalizeOmniboxInput(input: string): { kind: "search" | "url"; value: string } {
  const value = input.trim();
  return { kind: shouldTreatAsSearch(value) ? "search" : "url", value };
}
