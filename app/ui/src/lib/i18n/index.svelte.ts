import { en, type Messages } from "./en";
import { ru } from "./ru";

export type Locale = "en" | "ru";
export type { Messages };

export const LOCALES: readonly Locale[] = ["en", "ru"];
const DICTIONARIES: Record<Locale, Messages> = { en, ru };
const STORAGE_KEY = "ploom.locale";

function isLocale(value: unknown): value is Locale {
  return value === "en" || value === "ru";
}

function readStored(): Locale | null {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return isLocale(value) ? value : null;
  } catch {
    return null;
  }
}

function fromNavigator(): Locale {
  const languages = typeof navigator === "undefined" ? [] : navigator.languages;
  for (const tag of languages) {
    const base = tag.toLowerCase().split("-")[0];
    if (isLocale(base)) return base;
  }
  return "en";
}

class I18n {
  locale = $state<Locale>(readStored() ?? fromNavigator());
  readonly t: Messages = $derived(DICTIONARIES[this.locale]);

  /** BCP 47 tag for `Intl` formatters. */
  get tag(): string {
    return this.locale === "ru" ? "ru-RU" : "en-US";
  }

  set(locale: Locale): void {
    this.locale = locale;
    try {
      localStorage.setItem(STORAGE_KEY, locale);
    } catch {
      // Storage can be unavailable (private mode, sandboxed webview); the choice still applies.
    }
  }
}

export const i18n = new I18n();

$effect.root(() => {
  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });
});

/** Look up a backend-provided key in a localized table, falling back to the key itself. */
export function lookup<T extends Record<string, string>>(table: T, key: string): string {
  return Object.hasOwn(table, key) ? (table[key as keyof T] as string) : key;
}
