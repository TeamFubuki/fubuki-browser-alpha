export function matchesSearchTerms(query: string, terms: readonly string[]) {
  const needle = query.trim().toLocaleLowerCase();
  return !needle || terms.join(' ').toLocaleLowerCase().includes(needle);
}
