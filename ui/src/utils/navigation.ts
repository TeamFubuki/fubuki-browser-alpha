const SCHEME_PATTERN = /^[a-z][a-z0-9+.-]*:/i;
const IPV4_PATTERN = /^(\d{1,3}\.){3}\d{1,3}$/;
const BRACKETED_IPV6_PATTERN = /^\[[0-9a-f:.]+\](?::\d{1,5})?(?:[/?#]|$)/i;
const ASCII_DOMAIN_PATTERN = /^([a-z0-9-]+\.)+[a-z0-9-]+$/i;
const LOCAL_HOSTNAME_WITH_PORT_PATTERN = /^[a-z0-9-]+:\d{1,5}(?:[/?#]|$)/i;

export function containsWhitespace(value: string): boolean {
  return /\s/.test(value);
}

export function containsNonAscii(value: string): boolean {
  return /[^\u0000-\u007f]/.test(value);
}

function stripPathQueryFragment(value: string): string {
  return value.split(/[/?#]/, 1)[0] ?? value;
}

function stripPort(authority: string): string {
  if (authority.startsWith("[")) return authority;
  const match = authority.match(/^(.*):(\d{1,5})$/);
  if (!match) return authority;
  const port = Number(match[2]);
  return port >= 0 && port <= 65535 ? match[1] : authority;
}

function hostPart(value: string): string {
  return stripPort(stripPathQueryFragment(value));
}

function looksLikeIpv4Address(host: string): boolean {
  if (!IPV4_PATTERN.test(host)) return false;
  return host.split(".").every((part) => Number(part) <= 255);
}

function looksLikeDomain(host: string, value: string): boolean {
  if (!host.includes(".")) return false;
  if (containsNonAscii(value)) return true;
  return ASCII_DOMAIN_PATTERN.test(host);
}

function looksLikeUrlInput(value: string): boolean {
  if (containsWhitespace(value)) return false;
  if (LOCAL_HOSTNAME_WITH_PORT_PATTERN.test(value) || BRACKETED_IPV6_PATTERN.test(value)) return true;
  if (SCHEME_PATTERN.test(value)) return true;
  const host = hostPart(value);
  return host.toLowerCase() === "localhost" || looksLikeIpv4Address(host) || looksLikeDomain(host, value);
}

export function shouldTreatAsSearch(input: string): boolean {
  const value = input.trim();
  if (!value) return false;
  return !looksLikeUrlInput(value);
}

export function normalizeOmniboxInput(input: string): { kind: "search" | "url"; value: string } {
  const value = input.trim();
  return { kind: shouldTreatAsSearch(value) ? "search" : "url", value };
}
