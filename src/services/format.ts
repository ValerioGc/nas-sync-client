import type { Locale } from '@/types';

/**
 * Formats a localized timestamp, using the fallback for missing or sentinel dates.
 */
export function formatMoment(iso: string | null, locale: Locale, never: string) {
  const date = iso ? new Date(iso) : null;
  if (!date || Number.isNaN(date.getTime()) || date.getUTCFullYear() < 2000) {
    return never;
  }
  return new Intl.DateTimeFormat(locale, { dateStyle: 'short', timeStyle: 'short' }).format(date);
}

export function fileName(path: string) {
  return path.split(/[\\/]/).findLast(Boolean) ?? path;
}
