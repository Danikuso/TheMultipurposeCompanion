/** `tab` solo admite las disposiciones temáticas; las pestañas propias llevan nombre del usuario y se omiten. */
export function editTab(tabKey: string | null): Record<string, string> {
  return tabKey?.startsWith('layout:') ? { tab: tabKey } : {};
}
