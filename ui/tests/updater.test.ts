import { describe, it, expect, beforeEach, vi } from 'vitest';
import { updater, fetchUpdateStatus, checkForUpdates, installUpdate, formatBytes } from '../src/lib/updater.svelte';

describe('updater store and methods', () => {
  beforeEach(() => {
    updater.status = {
      current_version: '1.0.1',
      latest_version: '1.0.1',
      has_update: false,
      release_name: '',
      release_url: '',
      download_url: null,
      asset_name: null,
      asset_size: null,
      release_notes: '',
      published_at: null,
      is_checking: false,
      is_downloading: false,
      download_progress: 0,
      status_message: null,
      error: null,
    };
    updater.isOpenModal = false;
    updater.autoPrompted = false;
    vi.restoreAllMocks();
  });

  it('formatBytes formats file sizes correctly', () => {
    expect(formatBytes(null)).toBe('—');
    expect(formatBytes(0)).toBe('—');
    expect(formatBytes(13_958_210)).toBe('13.3 MB');
    expect(formatBytes(64_800_000)).toBe('61.8 MB');
  });

  it('fetchUpdateStatus reads status and opens modal on first prompt if update exists', async () => {
    const mockStatus = {
      current_version: '1.0.1',
      latest_version: '1.0.2',
      has_update: true,
      release_name: 'v1.0.2',
      release_url: 'https://github.com/Danikuso/TheMultipurposeCompanion/releases/tag/v1.0.2',
      download_url: 'https://github.com/.../TMC-Setup.exe',
      asset_name: 'TMC-Setup.exe',
      asset_size: 13958210,
      release_notes: 'Fixes and enhancements',
      published_at: '2026-10-31T01:00:00Z',
      is_checking: false,
      is_downloading: false,
      download_progress: 0,
      status_message: null,
      error: null,
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockStatus,
    } as unknown as Response);

    const result = await fetchUpdateStatus();
    expect(result).not.toBeNull();
    expect(updater.status.has_update).toBe(true);
    expect(updater.status.latest_version).toBe('1.0.2');
    expect(updater.isOpenModal).toBe(true);
    expect(updater.autoPrompted).toBe(true);
  });

  it('checkForUpdates queries backend check and opens modal if update found', async () => {
    const mockStatus = {
      current_version: '1.0.1',
      latest_version: '1.0.2',
      has_update: true,
      release_name: 'v1.0.2',
      release_url: 'https://github.com/...',
      download_url: 'https://github.com/.../TMC-Setup.exe',
      asset_name: 'TMC-Setup.exe',
      asset_size: 13958210,
      release_notes: 'Improvements',
      published_at: null,
      is_checking: false,
      is_downloading: false,
      download_progress: 0,
      status_message: null,
      error: null,
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockStatus,
    } as unknown as Response);

    const result = await checkForUpdates();
    expect(global.fetch).toHaveBeenCalledWith('/api/update/check', { method: 'POST' });
    expect(result?.has_update).toBe(true);
    expect(updater.isOpenModal).toBe(true);
  });

  it('installUpdate calls /api/update/install and tracks progress', async () => {
    global.fetch = vi.fn().mockImplementation((url: string) => {
      if (url === '/api/update/install') {
        return Promise.resolve({
          ok: true,
          json: async () => ({ ok: true, message: 'Installer started' }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: async () => ({
          ...updater.status,
          download_progress: 0.5,
          status_message: 'Descargando: 6.6 MB / 13.3 MB (50%)',
        }),
      });
    });

    const success = await installUpdate();
    expect(success).toBe(true);
    expect(global.fetch).toHaveBeenCalledWith('/api/update/install', { method: 'POST' });
  });
});
