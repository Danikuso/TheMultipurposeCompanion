/** Confirmación del navegador envuelta para poder simularla en los tests. */
export function askConfirm(message: string): boolean {
  return globalThis.confirm?.(message) ?? false;
}
