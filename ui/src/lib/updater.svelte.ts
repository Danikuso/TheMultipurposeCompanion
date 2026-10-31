/**
 * Gestor reactivo de actualizaciones de TMC («TMC Glass HUD»).
 * Consulta el estado con el backend y coordina la descarga e instalación del instalador oficial.
 */

export interface UpdateStatus {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  release_name: string;
  release_url: string;
  download_url: string | null;
  asset_name: string | null;
  asset_size: number | null;
  release_notes: string;
  published_at: string | null;
  is_checking: boolean;
  is_downloading: boolean;
  download_progress: number; // 0.0 .. 1.0
  status_message: string | null;
  error: string | null;
}

export const updater = $state({
  status: {
    current_version: '',
    latest_version: '',
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
  } as UpdateStatus,
  isOpenModal: false,
  autoPrompted: false,
});

let pollTimer: ReturnType<typeof setInterval> | null = null;

export async function fetchUpdateStatus(): Promise<UpdateStatus | null> {
  try {
    const res = await fetch('/api/update/status');
    if (!res.ok) return null;
    const data = (await res.json()) as UpdateStatus;
    updater.status = data;

    // Si hay una actualización detectada al arrancar y no se ha mostrado aún, abrir modal
    if (data.has_update && !updater.autoPrompted) {
      updater.autoPrompted = true;
      updater.isOpenModal = true;
    }

    return data;
  } catch {
    return null;
  }
}

export async function checkForUpdates(): Promise<UpdateStatus | null> {
  try {
    updater.status.is_checking = true;
    updater.status.error = null;
    const res = await fetch('/api/update/check', { method: 'POST' });
    if (!res.ok) {
      updater.status.is_checking = false;
      const text = await res.text();
      updater.status.error = text || 'Error al comprobar actualizaciones';
      return null;
    }
    const data = (await res.json()) as UpdateStatus;
    updater.status = data;
    if (data.has_update) {
      updater.isOpenModal = true;
    }
    return data;
  } catch (e) {
    updater.status.is_checking = false;
    updater.status.error = e instanceof Error ? e.message : 'Error al conectar con el servidor';
    return null;
  }
}

export async function installUpdate(): Promise<boolean> {
  try {
    updater.status.is_downloading = true;
    updater.status.download_progress = 0.0;
    updater.status.status_message = 'Iniciando descarga del instalador...';
    updater.status.error = null;

    // Iniciar sondeo periódico de progreso mientras descarga
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = setInterval(async () => {
      try {
        const res = await fetch('/api/update/status');
        if (res.ok) {
          const st = (await res.json()) as UpdateStatus;
          updater.status.download_progress = st.download_progress;
          updater.status.status_message = st.status_message;
          if (!st.is_downloading && pollTimer) {
            clearInterval(pollTimer);
            pollTimer = null;
          }
          if (st.error) {
            updater.status.error = st.error;
            updater.status.is_downloading = false;
          }
        }
      } catch {
        // En caso de que el servidor ya se haya cerrado para dar paso al instalador
        if (pollTimer) {
          clearInterval(pollTimer);
          pollTimer = null;
        }
      }
    }, 400);

    const res = await fetch('/api/update/install', { method: 'POST' });
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }

    if (!res.ok) {
      updater.status.is_downloading = false;
      const text = await res.text();
      updater.status.error = text || 'Error al ejecutar la instalación';
      return false;
    }

    updater.status.is_downloading = false;
    updater.status.download_progress = 1.0;
    updater.status.status_message = 'Instalador iniciado. TMC se cerrará en breves instantes.';
    return true;
  } catch (e) {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
    updater.status.is_downloading = false;
    updater.status.error = e instanceof Error ? e.message : 'Error durante la actualización';
    return false;
  }
}

export function formatBytes(bytes?: number | null): string {
  if (!bytes || bytes <= 0) return '—';
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(1)} MB`;
}
