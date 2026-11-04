const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** UUID en la forma canónica (minúsculas con guiones), la única que acepta el servidor. */
export function isUuid(s: unknown): s is string {
  return typeof s === 'string' && UUID.test(s);
}

/**
 * UUID v4. `crypto.randomUUID` solo existe en contextos seguros; los clientes LAN abren la UI por
 * http://<ip>, así que se recurre a `crypto.getRandomValues`, que sí está disponible allí.
 */
export function uuidv4(): string {
  if (typeof crypto.randomUUID === 'function') return crypto.randomUUID();
  const b = crypto.getRandomValues(new Uint8Array(16));
  b[6] = (b[6] & 0x0f) | 0x40;
  b[8] = (b[8] & 0x3f) | 0x80;
  const h = Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}
