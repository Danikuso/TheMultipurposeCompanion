// Motor del Asistente de Voz y Reconocimiento de Cabina («TMC Glass HUD»)
import { getLocale } from './i18n';
import { whisperEngine, type AudioRecorderHandle, type WhisperModelStatus } from './whisperEngine';

export type ListeningState = 'inactive' | 'listening' | 'processing' | 'speaking';
export type ListeningMode = 'continuous' | 'ptt';
export type SttEngineKind = 'whisper' | 'native';

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
  sttEngine = $state<SttEngineKind>('whisper');
  state = $state<ListeningState>('inactive');
  lastQuery = $state<string | null>(null);
  lastResponse = $state<VoiceCommandResponse | null>(null);
  voiceAttackStatus = $state<VoiceAttackStatus | null>(null);
  error = $state<string | null>(null);

  // Estado del modelo neuronal local Whisper AI (~39MB)
  whisperStatus = $state<WhisperModelStatus>('unloaded');
  whisperProgress = $state(0);

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private recognition: any = null;
  private isBrowserSupported = false;
  private resetTimer: ReturnType<typeof setTimeout> | null = null;
  private activeRecorder: AudioRecorderHandle | null = null;

  constructor() {
    if (typeof window !== 'undefined') {
      const savedEnabled = localStorage.getItem('tmc.voice.enabled');
      this.enabled = savedEnabled === 'true';

      const savedMode = localStorage.getItem('tmc.voice.mode');
      if (savedMode === 'ptt' || savedMode === 'continuous') {
        this.mode = savedMode;
      }

      const savedEngine = localStorage.getItem('tmc.voice.stt_engine');
      if (savedEngine === 'whisper' || savedEngine === 'native') {
        this.sttEngine = savedEngine;
      }

      // Soporte nativo como respaldo secundario opcional
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
            if (this.enabled && this.mode === 'continuous' && this.state === 'listening' && this.sttEngine === 'native') {
              try {
                this.recognition.start();
              } catch {
                // ignorar
              }
            } else if (this.state === 'listening' && this.sttEngine === 'native') {
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
    return true; // Whisper AI local soporta todos los navegadores/WebViews con Web Audio API
  }

  get isRecordingAudio(): boolean {
    return this.activeRecorder !== null;
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
      if (this.sttEngine === 'whisper' && !whisperEngine.isReady()) {
        void this.preloadWhisperModel();
      }
      if (this.mode === 'continuous' && this.sttEngine === 'native') {
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
    if (this.enabled && this.sttEngine === 'native') {
      if (mode === 'continuous') {
        this.startListening();
      } else {
        this.stopListening();
      }
    }
  }

  setSttEngine(engine: SttEngineKind): void {
    this.sttEngine = engine;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('tmc.voice.stt_engine', engine);
    }
    if (engine === 'whisper') {
      this.stopListening();
      void this.preloadWhisperModel();
    }
  }

  async preloadWhisperModel(): Promise<boolean> {
    this.whisperStatus = 'loading';
    const ok = await whisperEngine.loadModel((pct) => {
      this.whisperProgress = pct;
    });
    this.whisperStatus = whisperEngine.status;
    return ok;
  }

  async startListening(): Promise<void> {
    if (!this.enabled) return;
    this.error = null;

    if (this.sttEngine === 'whisper') {
      // Motor neuronal Whisper AI on-device
      try {
        if (!whisperEngine.isReady()) {
          this.whisperStatus = 'loading';
          const ready = await whisperEngine.loadModel((pct) => {
            this.whisperProgress = pct;
          });
          this.whisperStatus = whisperEngine.status;
          if (!ready) {
            this.error = whisperEngine.error || 'error-loading-whisper';
            return;
          }
        }

        this.state = 'listening';
        this.activeRecorder = await whisperEngine.startRecording();
      } catch (err) {
        this.error = err instanceof Error ? err.message : String(err);
        this.state = 'inactive';
      }
    } else if (this.recognition) {
      // Motor nativo del navegador como fallback
      this.state = 'listening';
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

  async stopListening(): Promise<void> {
    if (this.sttEngine === 'whisper' && this.activeRecorder) {
      const recorder = this.activeRecorder;
      this.activeRecorder = null;
      this.state = 'processing';

      try {
        const samples = await recorder.stop();
        if (samples.length > 8000) { // Al menos 0.5s de audio grabado
          const text = await whisperEngine.transcribe(samples, getLocale());
          if (text && text.trim().length > 0) {
            await this.sendQuery(text);
            return;
          }
        }
      } catch (err) {
        this.error = err instanceof Error ? err.message : String(err);
      }

      this.state = 'inactive';
    } else {
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
  }

  abortListening(): void {
    if (this.activeRecorder) {
      this.activeRecorder.abort();
      this.activeRecorder = null;
    }
    this.stopListening();
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

      const durationMs = Math.max(2000, Math.min(8000, (data.spoken_text?.length ?? 10) * 60));
      this.resetTimer = setTimeout(() => {
        if (this.enabled && this.mode === 'continuous' && this.sttEngine === 'native') {
          this.state = 'listening';
        } else {
          this.state = 'inactive';
        }
        this.resetTimer = null;
      }, durationMs);

      return data;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
      this.state = this.enabled && this.mode === 'continuous' && this.sttEngine === 'native' ? 'listening' : 'inactive';
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
