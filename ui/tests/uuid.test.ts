import { afterEach, describe, expect, test, vi } from 'vitest';
import { isUuid, uuidv4 } from '../src/lib/uuid';

const V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

afterEach(() => { vi.unstubAllGlobals(); });

describe('uuid', () => {
  test('uuidv4 genera la forma canónica v4', () => {
    for (let i = 0; i < 50; i++) expect(uuidv4()).toMatch(V4);
  });

  test('respaldo con getRandomValues cuando no hay randomUUID', () => {
    const getRandomValues = vi.fn((a: Uint8Array) => { a.fill(0xff); return a; });
    vi.stubGlobal('crypto', { getRandomValues });
    const id = uuidv4();
    expect(getRandomValues).toHaveBeenCalledTimes(1);
    expect(id).toBe('ffffffff-ffff-4fff-bfff-ffffffffffff');
    expect(id).toMatch(V4);
  });

  test('respaldo con ceros fija versión y variante', () => {
    vi.stubGlobal('crypto', { getRandomValues: (a: Uint8Array) => a.fill(0) });
    expect(uuidv4()).toBe('00000000-0000-4000-8000-000000000000');
  });

  test('isUuid solo acepta la forma canónica en minúsculas', () => {
    expect(isUuid('11111111-1111-4111-8111-111111111111')).toBe(true);
    expect(isUuid('ABCDEFAB-CDEF-4BCD-8BCD-ABCDEFABCDEF')).toBe(false);
    expect(isUuid('basura')).toBe(false);
    expect(isUuid('urn:uuid:11111111-1111-4111-8111-111111111111')).toBe(false);
    expect(isUuid(null)).toBe(false);
    expect(isUuid(42)).toBe(false);
  });
});
