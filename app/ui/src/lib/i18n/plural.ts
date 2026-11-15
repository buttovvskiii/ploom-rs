const ruRules = new Intl.PluralRules("ru");

/** Russian noun form for a count: `ru(21, "стик", "стика", "стиков")` → "стик". */
export function ru(n: number, one: string, few: string, many: string): string {
  switch (ruRules.select(n)) {
    case "one":
      return one;
    case "few":
      return few;
    case "many":
      return many;
    default:
      return few;
  }
}

export function en(n: number, one: string, other: string): string {
  return n === 1 ? one : other;
}
