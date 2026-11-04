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
 * alto visible del cuerpo menos el contenido real encima y debajo de la lista.
 *
 * No usa scrollHeight (en los navegadores está acotado a >= clientHeight e incluiría la holgura) ni el
 * fondo del hijo directo del cuerpo (puede estar estirado con flex: 1). El contenido de debajo se suma
 * nivel a nivel: dentro de cada ancestro, lo que hay entre el fondo del hijo y el fondo del ancestro;
 * en el hijo directo del cuerpo, solo sus hermanos posteriores; y el padding inferior del cuerpo.
 * El resultado no depende del alto actual de la lista. Devuelve null si la lista está oculta.
 */
export function listSpace(list: HTMLElement, body: HTMLElement): number | null {
  const l = list.getBoundingClientRect();
  if (l.height === 0) return null;
  const b = body.getBoundingClientRect();
  const top = l.top - b.top - body.clientTop + body.scrollTop;
  let below = 0;
  let el: Element = list;
  let bottom = l.bottom;
  while (el.parentElement && el.parentElement !== body) {
    const parent = el.parentElement;
    if (parent.parentElement === body) {
      for (let s = el.nextElementSibling; s; s = s.nextElementSibling) below += s.getBoundingClientRect().height;
    } else {
      const parentBottom = parent.getBoundingClientRect().bottom;
      below += Math.max(0, parentBottom - bottom);
      bottom = Math.max(bottom, parentBottom);
    }
    el = parent;
  }
  if (el.parentElement !== body) return null;
  for (let s = el.nextElementSibling; s; s = s.nextElementSibling) below += s.getBoundingClientRect().height;
  below += parseFloat(getComputedStyle(body).paddingBottom) || 0;
  return Math.floor(body.clientHeight - top - below);
}
