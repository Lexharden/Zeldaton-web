/** Replaces {key} tokens: fillTemplate('{n} hours', { n: 4 }) -> '4 hours'. */
export function fillTemplate(text: string, values: Record<string, string | number>): string {
  return text.replace(/\{(\w+)\}/g, (m, key: string) => (key in values ? String(values[key]) : m))
}

/** Splits "WATCH THE RACE" into ["WATCH THE", "RACE"] so the last word can be highlighted. */
export function splitLastWord(text: string): [string, string] {
  const i = text.lastIndexOf(' ')
  return i === -1 ? ['', text] : [text.slice(0, i), text.slice(i + 1)]
}
