import type { LayoutDoc } from '../shell/layoutModel';

export interface ProfileSummary { device_id: string; name: string; updated_at: string }
export interface Profile extends ProfileSummary { doc: unknown }

export class ApiError extends Error {
  constructor(public status: number, public code: string) {
    super(`layout-api-${status}-${code}`);
  }
}

const BASE = '/api/ui/layouts';
const enc = encodeURIComponent;
const JSON_HEADERS = { 'content-type': 'application/json' };

async function fail(res: Response): Promise<never> {
  let code = 'unknown';
  try {
    const body = (await res.json()) as { error?: unknown };
    if (typeof body.error === 'string') code = body.error;
  } catch { /* cuerpo no JSON */ }
  throw new ApiError(res.status, code);
}

async function send<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init);
  if (!res.ok) return fail(res);
  return (await res.json()) as T;
}

export function listProfiles(): Promise<ProfileSummary[]> {
  return send<ProfileSummary[]>(BASE);
}

export async function getProfile(id: string): Promise<Profile | null> {
  const res = await fetch(`${BASE}/${enc(id)}`);
  if (res.status === 404) return null;
  if (!res.ok) return fail(res);
  return (await res.json()) as Profile;
}

export function putProfile(id: string, name: string, doc: LayoutDoc): Promise<Profile> {
  return send<Profile>(`${BASE}/${enc(id)}`, { method: 'PUT', headers: JSON_HEADERS, body: JSON.stringify({ name, doc }) });
}

export function renameProfile(id: string, name: string): Promise<Profile> {
  return send<Profile>(`${BASE}/${enc(id)}`, { method: 'PATCH', headers: JSON_HEADERS, body: JSON.stringify({ name }) });
}

export async function deleteProfile(id: string): Promise<void> {
  const res = await fetch(`${BASE}/${enc(id)}`, { method: 'DELETE' });
  if (!res.ok) await fail(res);
}

export function copyProfile(targetId: string, sourceId: string, name: string): Promise<Profile> {
  return send<Profile>(`${BASE}/${enc(targetId)}/copy-from/${enc(sourceId)}`, {
    method: 'POST', headers: JSON_HEADERS, body: JSON.stringify({ name }),
  });
}

export async function getDesktopDeviceId(): Promise<string | null> {
  const res = await fetch('/api/ui/device/desktop');
  if (!res.ok) return null;
  const body = (await res.json()) as { device_id?: unknown };
  return typeof body.device_id === 'string' ? body.device_id : null;
}
