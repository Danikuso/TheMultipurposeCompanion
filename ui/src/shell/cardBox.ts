import { getContext, setContext } from 'svelte';

/** Caja que el marco de tarjeta impone a su contenido: alto fijo (o automático) y modo edición. */
export interface CardBox { readonly height: number | null; readonly editing: boolean }

const KEY = Symbol('cardBox');

export function setCardBox(box: CardBox): void {
  setContext(KEY, box);
}

export function getCardBox(): CardBox | undefined {
  return getContext<CardBox | undefined>(KEY);
}

/**
 * Píxeles disponibles para una lista dentro del cuerpo con scroll de un panel de alto fijo:
 * alto visible del cuerpo menos lo que ocupa el contenido encima y debajo de la lista.
 * Devuelve null si la lista está oculta (detalles plegados) y no se puede medir.
 */
export function listSpace(list: HTMLElement, body: HTMLElement): number | null {
  const l = list.getBoundingClientRect();
  if (l.height === 0) return null;
  const b = body.getBoundingClientRect();
  const top = l.top - b.top - body.clientTop + body.scrollTop;
  const below = body.scrollHeight - top - l.height;
  return Math.floor(body.clientHeight - top - below);
}
