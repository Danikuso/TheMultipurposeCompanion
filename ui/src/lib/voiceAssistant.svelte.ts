// Motor del Asistente de Voz y Reconocimiento de Cabina («TMC Glass HUD»)
import { getLocale } from './i18n';

export type ListeningState = 'inactive' | 'listening' | 'processing' | 'speaking';
export type ListeningMode = 'continuous' | 'ptt';

export interface VoiceCommandResponse {
  intent: string;
  action_executed?: string | null;
  spoken_text: string;
  clipboard_copied?: string | null;
  data?: Record<string, unknown>;
}

export interface VoiceAttackStatus {
  installed: boolean;
  running: boolean;
  profile_available: boolean;
  install_path?: string | null;
}

class VoiceAssistantService {
  enabled = $state(false);
  mode = $state<ListeningMode>('continuous');
  state = $state<ListeningState>('inactive');
  lastQuery = $state<string | null>(null);
  lastResponse = $state<VoiceCommandResponse | null>(null);
  voiceAttackStatus = $state<VoiceAttackStatus | null>(null);
  error = $state<string | null>(null);

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private recognition: any = null;
  private isBrowserSupported = false;
  private resetTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    if (typeof window !== 'undefined') {
      const savedEnabled = localStorage.getItem('tmc.voice.enabled');
      this.enabled = savedEnabled === 'true';

      const savedMode = localStorage.getItem('tmc.voice.mode');
      if (savedMode === 'ptt' || savedMode === 'continuous') {
        this.mode = savedMode;
      }

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const SpeechRec = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
      if (SpeechRec) {
        this.isBrowserSupported = true;
        try {
          this.recognition = new SpeechRec();
          this.recognition.continuous = true;
          this.recognition.interimResults = false;
          this.recognition.maxAlternatives = 1;

          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          this.recognition.onresult = (event: any) => {
            const last = event.results.length - 1;
            const transcript = event.results[last][0].transcript.trim();
            if (transcript.length > 0) {
              void this.sendQuery(transcript);
            }
          };

          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          this.recognition.onerror = (event: any) => {
            if (event.error !== 'no-speech') {
              this.error = event.error;
            }
            if (this.state === 'listening' && this.mode === 'ptt') {
              this.state = 'inactive';
            }
          };

          this.recognition.onend = () => {
            if (this.enabled && this.mode === 'continuous' && this.state === 'listening') {
              // Reinicio automático de escucha continua
              try {
                this.recognition.start();
              } catch {
                // ignorar si ya arrancó
              }
            } else if (this.state === 'listening') {
              this.state = 'inactive';
            }
          };
        } catch {
          this.isBrowserSupported = false;
        }
      }
    }
  }

  get isSupported(): boolean {
    return this.isBrowserSupported;
  }

  toggleEnabled(): void {
    this.setEnabled(!this.enabled);
  }

  setEnabled(val: boolean): void {
    this.enabled = val;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('tmc.voice.enabled', String(val));
    }
    if (val) {
      if (this.mode === 'continuous') {
        this.startListening();
      }
    } else {
      this.stopListening();
    }
  }

  setMode(mode: ListeningMode): void {
    this.mode = mode;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('tmc.voice.mode', mode);
    }
    if (this.enabled) {
      if (mode === 'continuous') {
        this.startListening();
      } else {
        this.stopListening();
      }
    }
  }

  startListening(): void {
    if (!this.enabled) return;
    this.error = null;
    this.state = 'listening';

    if (this.recognition) {
      try {
        const locale = getLocale();
        const langMap: Record<string, string> = {
          es: 'es-ES',
          en: 'en-US',
          fr: 'fr-FR',
          de: 'de-DE',
          pt: 'pt-PT',
        };
        this.recognition.lang = langMap[locale] ?? 'es-ES';
        this.recognition.start();
      } catch {
        // Puede fallar si ya está escuchando
      }
    }
  }

  stopListening(): void {
    if (this.state === 'listening') {
      this.state = 'inactive';
    }
    if (this.recognition) {
      try {
        this.recognition.stop();
      } catch {
        // ignorar
      }
    }
  }

  async sendQuery(queryText: string, localeOverride?: string): Promise<VoiceCommandResponse | null> {
    const text = queryText.trim();
    if (!text) return null;

    if (this.resetTimer) {
      clearTimeout(this.resetTimer);
      this.resetTimer = null;
    }

    this.lastQuery = text;
    this.state = 'processing';
    this.error = null;

    try {
      const locale = localeOverride ?? getLocale();
      const res = await fetch('/api/voice/intent', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          query: text,
          locale,
        }),
      });

      if (!res.ok) {
        throw new Error(`voice-intent-failed-${res.status}`);
      }

      const data: VoiceCommandResponse = await res.json();
      this.lastResponse = data;
      this.state = 'speaking';

      // Transición automática tras finalizar la locución del copiloto
      const durationMs = Math.max(2000, Math.min(8000, (data.spoken_text?.length ?? 10) * 60));
      this.resetTimer = setTimeout(() => {
        if (this.enabled && this.mode === 'continuous') {
          this.state = 'listening';
        } else {
          this.state = 'inactive';
        }
        this.resetTimer = null;
      }, durationMs);

      return data;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      this.state = this.enabled && this.mode === 'continuous' ? 'listening' : 'inactive';
      return null;
    }
  }

  async fetchVoiceAttackStatus(): Promise<VoiceAttackStatus | null> {
    try {
      const res = await fetch('/api/voice/voiceattack');
      if (res.ok) {
        const data: VoiceAttackStatus = await res.json();
        this.voiceAttackStatus = data;
        return data;
      }
    } catch {
      // Ignorar errores de red
    }
    return null;
  }
}

export const voiceAssistant = new VoiceAssistantService();
