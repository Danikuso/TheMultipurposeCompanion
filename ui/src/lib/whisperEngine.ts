// Motor Neuronal de Reconocimiento de Voz On-Device con Whisper AI («TMC Glass HUD»)
// Modelo ultraligero (~39MB) optimizado para inferencia local offline y privada.

export type WhisperModelStatus = 'unloaded' | 'loading' | 'ready' | 'error';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let pipelineInstance: any = null;

export interface AudioRecorderHandle {
  stop: () => Promise<Float32Array>;
  abort: () => void;
}

export class WhisperEngine {
  status: WhisperModelStatus = 'unloaded';
  loadProgress = 0;
  error: string | null = null;
  private loadPromise: Promise<boolean> | null = null;

  isReady(): boolean {
    return this.status === 'ready' && pipelineInstance !== null;
  }

  async loadModel(onProgress?: (progressPct: number) => void): Promise<boolean> {
    if (this.isReady()) return true;
    if (this.loadPromise) return this.loadPromise;

    this.status = 'loading';
    this.error = null;
    this.loadProgress = 0;

    this.loadPromise = (async () => {
      try {
        const { pipeline, env } = await import('@huggingface/transformers');
        env.allowLocalModels = false;

        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const progressCallback = (info: any) => {
          if (info.status === 'progress' && typeof info.progress === 'number') {
            this.loadProgress = Math.round(info.progress);
            if (onProgress) onProgress(this.loadProgress);
          }
        };

        // Modelo Whisper-tiny multilingüe cuantizado (~39MB):
        // Soporta español, inglés, francés, alemán y portugués con latencia baja en CPU
        pipelineInstance = await pipeline(
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

        this.status = 'ready';
        this.loadProgress = 100;
        return true;
      } catch (err) {
        this.status = 'error';
        this.error = err instanceof Error ? err.message : String(err);
        return false;
      } finally {
        this.loadPromise = null;
      }
    })();

    return this.loadPromise;
  }

  async transcribe(audioSamples: Float32Array, language = 'es'): Promise<string> {
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

    const output = await pipelineInstance(audioSamples, {
      language: targetLang,
      task: 'transcribe',
    });

    const text = typeof output === 'string' ? output : output?.text || '';
    return text.trim();
  }

  async startRecording(): Promise<AudioRecorderHandle> {
    if (typeof navigator === 'undefined' || !navigator.mediaDevices?.getUserMedia) {
      throw new Error('media-devices-not-supported');
    }

    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const AudioCtx = window.AudioContext || (window as any).webkitAudioContext;
    const audioContext = new AudioCtx({ sampleRate: 16000 });
    const source = audioContext.createMediaStreamSource(stream);

    // Búfer mono para captura de voz
    const processor = audioContext.createScriptProcessor(4096, 1, 1);
    const chunks: Float32Array[] = [];

    processor.onaudioprocess = (e) => {
      const channel = e.inputBuffer.getChannelData(0);
      chunks.push(new Float32Array(channel));
    };

    source.connect(processor);
    processor.connect(audioContext.destination);

    let stopped = false;

    const cleanup = async () => {
      if (stopped) return;
      stopped = true;
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
