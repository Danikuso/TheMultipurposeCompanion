// Motor Neuronal de Reconocimiento de Voz On-Device con Whisper AI («TMC Glass HUD»)
// Modelo ultraligero (~39MB) optimizado para inferencia local offline y privada.

export type WhisperModelStatus = 'unloaded' | 'loading' | 'ready' | 'error';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let pipelineInstance: any = null;

export interface AudioRecorderHandle {
  stop: () => Promise<Float32Array>;
  abort: () => void;
}

/** Options controlling VAD silence detection and hard limits during microphone capture. */
export interface RecordingOptions {
  silenceThreshold?: number;
  silenceDurationMs?: number;
  maxDurationMs?: number;
  onSilenceDetected?: () => void;
  onAudioLevel?: (level: number) => void;
}

const HALLUCINATION_PATTERNS: RegExp[] = [
  /^[\s\p{P}\p{S}♪♫]+$/u,
  /^[[(]\s*(?:blank[_\s]?audio|silence|silencio|m[uú]sica|music|applause|aplausos|laughter|risas|inaudible|noise|ruido)\s*[\])]$/iu,
  /^(?:gracias por ver(?:\s+el\s+v[ií]deo)?|thank(?:s|\s+you)\s+for\s+watching|merci\s+d['’]avoir\s+regard[eé]|danke\s+f[uü]r['’]?s\s+zuschauen|obrigado\s+por\s+assistir)\.?$/iu,
  /^(?:subt[ií]tulos?\s+(?:por|realizados)|subtitled?\s+by|amara\.org|suscr[ií]bete|subscribe)\b/iu,
];

/**
 * Returns true if a Whisper transcript matches common silent/noise hallucination phrases.
 */
export function isWhisperHallucination(text: string): boolean {
  const normalized = text.trim().replace(/\s+/g, ' ');
  if (normalized.length < 2) {
    return true;
  }
  return HALLUCINATION_PATTERNS.some((pattern) => pattern.test(normalized));
}

/**
 * Trims leading/trailing near-silence and gently normalizes low-level speech peaks for Whisper.
 */
export function preprocessAudioSamples(samples: Float32Array): Float32Array {
  if (samples.length === 0) {
    return new Float32Array(0);
  }

  const silenceFloor = 0.006;
  const paddingSamples = 400; // ~25 ms at 16 kHz to preserve initial/final consonants

  let firstActive = -1;
  let lastActive = -1;
  for (let i = 0; i < samples.length; i += 1) {
    if (Math.abs(samples[i]) >= silenceFloor) {
      if (firstActive === -1) {
        firstActive = i;
      }
      lastActive = i;
    }
  }

  if (firstActive === -1 || lastActive === -1) {
    return new Float32Array(0);
  }

  const start = Math.max(0, firstActive - paddingSamples);
  const end = Math.min(samples.length, lastActive + 1 + paddingSamples);
  const trimmed = samples.slice(start, end);

  let peak = 0;
  for (let i = 0; i < trimmed.length; i += 1) {
    const abs = Math.abs(trimmed[i]);
    if (abs > peak) {
      peak = abs;
    }
  }

  if (peak <= silenceFloor) {
    return new Float32Array(0);
  }

  if (peak < 0.6) {
    // Cap amplification so quiet background hiss is not scaled excessively
    const gain = Math.min(12, 0.85 / peak);
    const normalized = new Float32Array(trimmed.length);
    for (let i = 0; i < trimmed.length; i += 1) {
      normalized[i] = Math.max(-1, Math.min(1, trimmed[i] * gain));
    }
    return normalized;
  }

  return trimmed;
}

export class WhisperEngine {
  status: WhisperModelStatus = 'unloaded';
  loadProgress = 0;
  error: string | null = null;
  private loadPromise: Promise<boolean> | null = null;
  private loadGeneration = 0;

  isReady(): boolean {
    return this.status === 'ready' && pipelineInstance !== null;
  }

  async loadModel(onProgress?: (progressPct: number) => void): Promise<boolean> {
    if (this.isReady()) return true;
    if (this.loadPromise) return this.loadPromise;

    const generation = ++this.loadGeneration;
    this.status = 'loading';
    this.error = null;
    this.loadProgress = 0;

    this.loadPromise = (async () => {
      try {
        const { pipeline, env } = await import('@huggingface/transformers');
        env.allowLocalModels = false;

        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const progressCallback = (info: any) => {
          if (generation !== this.loadGeneration) return;
          if (info.status === 'progress' && typeof info.progress === 'number') {
            this.loadProgress = Math.round(info.progress);
            if (onProgress) onProgress(this.loadProgress);
          }
        };

        // Modelo Whisper-tiny multilingüe cuantizado (~39MB):
        // Soporta español, inglés, francés, alemán y portugués con latencia baja en CPU
        const loadedPipeline = await pipeline(
          'automatic-speech-recognition',
          'onnx-community/whisper-tiny',
          {
            dtype: {
              encoder_model: 'q8',
              decoder_model_merged: 'q8',
            },
            progress_callback: progressCallback,
          },
        );

        if (generation !== this.loadGeneration) {
          if (typeof loadedPipeline?.dispose === 'function') {
            void loadedPipeline.dispose().catch(() => {});
          }
          return false;
        }

        pipelineInstance = loadedPipeline;
        this.status = 'ready';
        this.loadProgress = 100;
        return true;
      } catch (err) {
        if (generation === this.loadGeneration) {
          this.status = 'error';
          this.error = err instanceof Error ? err.message : String(err);
        }
        return false;
      } finally {
        if (generation === this.loadGeneration) {
          this.loadPromise = null;
        }
      }
    })();

    return this.loadPromise;
  }

  /** Releases the on-device ONNX Whisper pipeline from memory to reclaim RAM. */
  unloadModel(): void {
    this.loadGeneration += 1;
    const current = pipelineInstance;
    pipelineInstance = null;
    this.loadPromise = null;
    this.status = 'unloaded';
    this.loadProgress = 0;
    this.error = null;
    if (current && typeof current.dispose === 'function') {
      void current.dispose().catch(() => {});
    }
  }

  async transcribe(audioSamples: Float32Array, language = 'es'): Promise<string> {
    const processed = preprocessAudioSamples(audioSamples);
    if (processed.length < 2400) {
      return '';
    }

    if (!this.isReady()) {
      const loaded = await this.loadModel();
      if (!loaded || !pipelineInstance) {
        throw new Error(this.error || 'whisper-model-not-ready');
      }
    }

    const langMap: Record<string, string> = {
      es: 'spanish',
      en: 'english',
      fr: 'french',
      de: 'german',
      pt: 'portuguese',
    };
    const targetLang = langMap[language] || 'spanish';

    const output = await pipelineInstance(processed, {
      language: targetLang,
      task: 'transcribe',
    });

    const text = (typeof output === 'string' ? output : output?.text || '').trim();
    if (isWhisperHallucination(text)) {
      return '';
    }
    return text;
  }

  async startRecording(options: RecordingOptions = {}): Promise<AudioRecorderHandle> {
    if (typeof navigator === 'undefined' || !navigator.mediaDevices?.getUserMedia) {
      throw new Error('media-devices-not-supported');
    }

    const silenceThreshold = options.silenceThreshold ?? 0.012;
    const silenceDurationMs = options.silenceDurationMs ?? 1500;
    const maxDurationMs = options.maxDurationMs ?? 12000;

    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const AudioCtx = window.AudioContext || (window as any).webkitAudioContext;
    const audioContext = new AudioCtx({ sampleRate: 16000 });
    const source = audioContext.createMediaStreamSource(stream);

    // Búfer mono para captura de voz
    const processor = audioContext.createScriptProcessor(4096, 1, 1);
    const chunks: Float32Array[] = [];

    const startedAt = Date.now();
    let lastSpeechAt = startedAt;
    let speechDetected = false;
    let silenceTriggered = false;
    let stopped = false;

    const triggerAutoStop = () => {
      if (silenceTriggered || stopped) return;
      silenceTriggered = true;
      options.onSilenceDetected?.();
    };

    const maxTimer = setTimeout(triggerAutoStop, maxDurationMs);

    processor.onaudioprocess = (e) => {
      if (stopped) return;
      const channel = e.inputBuffer.getChannelData(0);
      chunks.push(new Float32Array(channel));

      let sumSq = 0;
      for (let i = 0; i < channel.length; i += 1) {
        sumSq += channel[i] * channel[i];
      }
      const rms = Math.sqrt(sumSq / Math.max(1, channel.length));
      options.onAudioLevel?.(Math.min(1, rms * 10));

      const now = Date.now();
      if (rms >= silenceThreshold) {
        speechDetected = true;
        lastSpeechAt = now;
      }

      if (now - startedAt >= maxDurationMs) {
        triggerAutoStop();
      } else if (speechDetected && now - lastSpeechAt >= silenceDurationMs) {
        triggerAutoStop();
      }
    };

    source.connect(processor);
    processor.connect(audioContext.destination);

    const cleanup = async () => {
      if (stopped) return;
      stopped = true;
      clearTimeout(maxTimer);
      options.onAudioLevel?.(0);
      try {
        source.disconnect();
        processor.disconnect();
        stream.getTracks().forEach((track) => track.stop());
        await audioContext.close();
      } catch {
        // ignorar
      }
    };

    return {
      stop: async () => {
        await cleanup();
        const total = chunks.reduce((acc, c) => acc + c.length, 0);
        const result = new Float32Array(total);
        let offset = 0;
        for (const chunk of chunks) {
          result.set(chunk, offset);
          offset += chunk.length;
        }
        return result;
      },
      abort: () => {
        void cleanup();
      },
    };
  }
}

export const whisperEngine = new WhisperEngine();
