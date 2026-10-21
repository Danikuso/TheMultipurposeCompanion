export type MaterialCategory = 'Raw' | 'Manufactured' | 'Encoded';
export interface MaterialRow { key: string; count: number; name_localised?: string | null; name_en?: string | null; known?: boolean }
export interface InventoryTopic { by_category: Partial<Record<MaterialCategory, Record<string, MaterialRow[]>>>; caps: Record<string, number>; at_max: string[]; snapshot: boolean }
export type LockerSection = 'items' | 'components' | 'consumables' | 'data';
export interface LockerTopic { revision: number; counts: Record<LockerSection, number> }
export interface LockerRow { key: string; name_localised: string | null; count: number }
export interface SessionTopic { totals: Record<string, { in: number; out: number }>; recent: { key: string; delta: number; origin: string; lost: number; at: string }[] }
export interface PageRequest { resource: 'locker' | 'goals'; revision: number; section: string; commander: string }
export interface EngineeringPage<T> { revision: number; section: string; cursor: number; next_cursor: number | null; total: number; items: T[] }
