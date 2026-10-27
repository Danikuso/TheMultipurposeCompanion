#!/usr/bin/env python3
"""
Worker subproceso de síntesis neuronal KokoroTTS para TMC.
Optimizado para baja latencia con ONNX Runtime multinúcleo y reproducción fluida en sounddevice.
"""

import sys
import os
import json
import traceback

def get_model_paths():
    appdata_local = os.environ.get('LOCALAPPDATA', '')
    candidates = [
        os.path.join(appdata_local, 'TMC', 'models', 'kokoro'),
        os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'models', 'kokoro'),
        os.path.join(os.path.dirname(os.path.abspath(__file__)), 'models', 'kokoro'),
        os.path.join(os.getcwd(), 'models', 'kokoro'),
    ]
    for c in candidates:
        voices = os.path.join(c, 'voices-v1.0.bin')
        if not os.path.exists(voices):
            continue
        # Priorizar FP32 si existe (en x86-64 Zen 3 / Intel moderno ejecuta AVX2 FMA directo, 8x más rápido que int8)
        model_fp = os.path.join(c, 'kokoro-v1.0.onnx')
        if os.path.exists(model_fp):
            return model_fp, voices
        model_int8 = os.path.join(c, 'kokoro-v1.0.int8.onnx')
        if os.path.exists(model_int8):
            return model_int8, voices
    return None, None

def determine_lang(voice_name):
    lower = (voice_name or '').lower()
    if lower.startswith('ef_') or lower.startswith('em_') or 'dora' in lower or 'alex' in lower or 'santa' in lower:
        return 'es'
    if lower.startswith('ff_') or 'siwis' in lower:
        return 'fr-fr'
    if lower.startswith('pf_') or lower.startswith('pm_'):
        return 'pt-br'
    if lower.startswith('if_') or lower.startswith('im_'):
        return 'it'
    if lower.startswith('jf_') or lower.startswith('jm_'):
        return 'ja'
    if lower.startswith('zf_') or lower.startswith('zm_'):
        return 'zh'
    return 'en-us'

def main():
    try:
        import onnxruntime as rt
        import kokoro_onnx
        import sounddevice as sd
    except ImportError as e:
        sys.stdout.write(json.dumps({"error": f"Missing python dependency: {e}"}) + "\n")
        sys.stdout.flush()
        sys.exit(1)

    model_path, voices_path = get_model_paths()
    if not model_path:
        sys.stdout.write(json.dumps({"error": "Kokoro ONNX model files not found"}) + "\n")
        sys.stdout.flush()
        sys.exit(2)

    try:
        # Configurar hilos e instrucciones aceleradas de CPU en ONNX Runtime
        opts = rt.SessionOptions()
        cpu_count = os.cpu_count() or 4
        opts.intra_op_num_threads = min(8, max(2, cpu_count))
        opts.inter_op_num_threads = 2
        opts.execution_mode = rt.ExecutionMode.ORT_SEQUENTIAL
        opts.graph_optimization_level = rt.GraphOptimizationLevel.ORT_ENABLE_ALL

        sess = rt.InferenceSession(model_path, sess_options=opts, providers=['CPUExecutionProvider'])
        kokoro = kokoro_onnx.Kokoro.from_session(sess, voices_path)
    except Exception as e:
        sys.stdout.write(json.dumps({"error": f"Failed to initialize Kokoro: {e}"}) + "\n")
        sys.stdout.flush()
        sys.exit(3)

    # Anunciar inicialización exitosa
    sys.stdout.write(json.dumps({"ready": True, "voices": kokoro.get_voices()}) + "\n")
    sys.stdout.flush()

    # Bucle de escucha de comandos por stdin
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except Exception:
            continue

        action = req.get("action", "speak")

        if action == "stop":
            try:
                sd.stop()
            except Exception:
                pass
            sys.stdout.write(json.dumps({"status": "stopped"}) + "\n")
            sys.stdout.flush()

        elif action == "ping":
            sys.stdout.write(json.dumps({"status": "pong"}) + "\n")
            sys.stdout.flush()

        elif action == "quit":
            try:
                sd.stop()
            except Exception:
                pass
            break

        elif action == "speak":
            text = req.get("text", "")
            voice = req.get("voice", "ef_dora")
            volume = float(req.get("volume", 80))
            speed = float(req.get("rate", 1.0))

            if not text:
                sys.stdout.write(json.dumps({"status": "empty_text"}) + "\n")
                sys.stdout.flush()
                continue

            lang = determine_lang(voice)
            try:
                samples, sample_rate = kokoro.create(text, voice=voice, speed=speed, lang=lang)
                vol_factor = max(0.0, min(1.0, volume / 100.0))
                audio = samples * vol_factor
                # Detener audio previo y reproducir
                sd.stop()
                sd.play(audio, sample_rate)
                sys.stdout.write(json.dumps({"status": "ok"}) + "\n")
                sys.stdout.flush()
            except Exception as e:
                sys.stdout.write(json.dumps({"status": "error", "message": str(e)}) + "\n")
                sys.stdout.flush()

if __name__ == "__main__":
    main()
