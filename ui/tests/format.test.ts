import { afterEach, expect, test, vi } from 'vitest';
import { setLocale } from '../src/lib/i18n';
import { decimal, integer, ly, relativeAge } from '../src/lib/format';

afterEach(() => {
  vi.restoreAllMocks();
  setLocale('es');
});

test('number formatters are reused per locale and options', () => {
  setLocale('es');
  integer(1);
  decimal(1.5);
  decimal(1.25, 2);
  ly(3);
  const spy = vi.spyOn(Intl, 'NumberFormat');
  for (let i = 0; i < 20; i++) {
    integer(i * 1000);
    decimal(i / 3);
    decimal(i / 3, 2);
    ly(i);
  }
  expect(spy).not.toHaveBeenCalled();
});

test('reused formatters follow the active locale', () => {
  setLocale('es');
  expect(integer(1234)).toBe('1.234');
  expect(decimal(1.5)).toBe('1,5');
  setLocale('en');
  expect(integer(1234)).toBe('1,234');
  expect(decimal(1.5)).toBe('1.5');
  setLocale('es');
  expect(integer(1234)).toBe('1.234');
});

test('light-year units follow the same locale as their numbers', () => {
  for (const [lang, expected] of [['es', '3,00 al'], ['en', '3.00 ly'], ['fr', '3,00 al'], ['de', '3,00 Lj'], ['pt', '3,00 al']]) {
    setLocale(lang);
    expect(ly(3)).toBe(expected);
    expect(ly(null)).toBe('—');
  }
});

test('relative time formatter is reused per locale', () => {
  setLocale('es');
  const base = Date.parse('2030-01-10T00:00:00Z');
  relativeAge('2030-01-09T23:55:00Z', base);
  const spy = vi.spyOn(Intl, 'RelativeTimeFormat');
  for (let i = 0; i < 10; i++) relativeAge('2030-01-09T22:00:00Z', base);
  expect(spy).not.toHaveBeenCalled();
});
