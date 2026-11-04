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
