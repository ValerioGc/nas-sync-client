import { expect, it } from 'vitest';
import { fileName, formatMoment } from '@/services/format';

it('formats a moment in the language of the client, or says never', () => {
  expect(formatMoment('2026-10-16T10:00:00Z', 'en', 'Never')).toMatch(/10\/16\/26/);
  expect(formatMoment(null, 'it', 'Mai')).toBe('Mai');

  expect(formatMoment('0001-01-01T00:00:00Z', 'it', 'Mai')).toBe('Mai');
  expect(formatMoment('not a date', 'it', 'Mai')).toBe('Mai');
});

it('keeps the last part of a path of any system', () => {
  expect(fileName('docs/report.pdf')).toBe('report.pdf');
  expect(fileName('C:\\Users\\me\\photo.jpg')).toBe('photo.jpg');
  expect(fileName('')).toBe('');
});
