# TMC · Especificación de F7 — Voz, Alertas de Cabina y Copiloto Auditivo

- **Estado:** 🔄 en propuesta / pendiente de aprobación del propietario
- **Fecha:** 2026-10-24
- **Rama:** `fase/f7-voz-copiloto`
- **Depende de:**
  - [`2026-10-17-tmc-arquitectura-y-f0-design.md`](2026-10-17-tmc-arquitectura-y-f0-design.md) (Arquitectura base, rendimiento < 150 MB y R12).
  - F1 ([`2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md`](2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md): Eventos de salto, estrellas repostables, bio-señales).
  - F2 ([`2026-10-18-tmc-f2-combate-mineria-design.md`](2026-10-18-tmc-f2-combate-mineria-design.md): Alertas de combate, temperatura, prospectores).
  - F5 ([`2026-10-23-tmc-f5-carrier-colonizacion-powerplay-design.md`](2026-10-23-tmc-f5-carrier-colonizacion-powerplay-design.md): Carrier y saltos).
  - Guía normativa de diseño: [`docs/design-system-glass-hud.md`](../design-system-glass-hud.md) y filosofía Companion [`AGENTS.md` §0](../../AGENTS.md).

---

## 1. Intención y filosofía de Companion

En *Elite Dangerous*, los momentos más tensos o enriquecedores suceden cuando el piloto tiene toda su atención visual fija en la trayectoria de vuelo, el combate o la maniobra de aterrizaje planetario:
- Apartar la mirada para leer una pantalla o una alerta de texto mientras se esquiva fuego enemigo o se vuela a ras de suelo en gravedad alta puede significar la destrucción de la nave.
- El copiloto COVAS oficial de Frontier ofrece avisos muy genéricos sobre la nave, pero **ignora completamente el contexto táctico y de conveniencia** que TMC sí conoce: si la estrella del sistema actual es repostable antes de quedarse sin combustible, cuántas señales biológicas con valor millonario hay en el sistema, si un asteroide prospectado supera el umbral de pureza rentable, o si los escudos acaban de caer.

Fase 7 dota a TMC de una **voz propia de copiloto espacial**:
1. **Inteligente y conciso:** Nunca habla por hablar ni satura la cabina con textos largos. Usa frases directas de radio táctica («Sol. Estrella repostable. Tres señales biológicas», «Temperatura crítica: 85 por ciento», «Prospector: Platino 42%»).
2. **Priorización estricta de alertas:** Una alerta de casco o temperatura corta o prevalece sobre un aviso informativo de exploración. Si se acumulan mensajes antiguos, se descartan automáticamente para no hablar con retraso.
3. **Arquitectura Híbrida Adaptativa (Rendimiento y Flexibilidad):**
   - **Motor Nativo (Windows OneCore / SAPI5):** Cero megabytes de descarga, arranque instantáneo, < 2 MB de memoria, ideal para jugadores con PCs modestos o que buscan inmediatez sin configuración.
   - **Motor Neuronal (KokoroTTS ONNX):** Voces de inteligencia artificial cinematográficas y de altísima naturalidad espacial, aisladas en un subproceso desacoplado con carga bajo demanda para respetar la regla de < 150 MB de RAM y < 1% de CPU en reposo de TMC.
4. **Control total y personalización:** El comandante elige qué canales desea escuchar (Combate, Exploración, Navegación, Minería, Carrier), el volumen, la velocidad y la voz específica, con un botón de prueba audible en Ajustes.

---

## 2. Objetivos, no objetivos y antiobjetivos

### Objetivos

1. **Nuevo crate `tmc-tts`:**
   - Abstracción de motor `TtsEngine` con dos implementaciones:
     - `NativeEngine`: Implementación en Windows usando APIs de sintetizador de voz nativo (Windows Media Speech / SAPI5) sin dependencias externas ni binarios pesados.
     - `KokoroEngine`: Implementación neuronal basada en modelos ONNX de Kokoro en subproceso desacoplado (con soporte para español, inglés, francés y portugués).
   - Cola de locución con prioridades (`Emergency`, `Tactical`, `Informational`) y descarte por antigüedad (*drop stale cues* > 4 segundos).
   - Control de cooldown por clave de aviso para evitar spam repetitivo (p. ej., no repetir aviso de temperatura más de una vez cada 15 segundos salvo empeoramiento).
2. **Generador de avisos de cabina (`VoiceAdvisor` en `tmc-core` / `tmc-modules`):**
   - **Navegación y Exploración:**
     - Salto completado: nombre del sistema y si la estrella primaria es repostable (K, G, B, F, O, A, M) o no.
     - Detección de biológicas: número de cuerpos con señales al terminar el FSS o entrar al sistema.
     - Exobiología: clonación de muestra («Muestra 1 de 3 registrada. Nueva muestra requerida a más de 500 metros»).
   - **Combate y Supervivencia:**
     - Escudos caídos / Escudos restaurados.
     - Advertencia de temperatura (> 80% y > 100%).
     - Daño estructural crítico de casco (< 50% y < 25%).
     - Interceptación en curso.
   - **Minería:**
     - Asteroide prospectado con mineral de interés por encima del umbral configurado por el usuario.
     - Bodega de carga llena al 100%.
   - **Fleet Carrier:**
     - Cuenta atrás de salto a 5 min y 1 min.
     - Salto de carrier completado.
3. **Localización de frases i18n (`locales/{es,en,fr,de,pt}/tts.ftl`):**
   - Frases tácticas parametrizadas en los 5 idiomas.
   - Nota para alemán: en alemán se usa síntesis nativa del SO conforme a R12 (Kokoro no cuenta con modelo en alemán).
4. **Panel de configuración en la UI (`SettingsPanel.svelte`):**
   - Conmutador general de Voz del Copiloto (On/Off).
   - Selector de motor (Nativo del SO vs Kokoro Neuronal).
   - Selector de voz y prueba de audio en vivo.
   - Control de volumen (0% a 100%) y velocidad (0.8x a 1.4x).
   - Conmutadores independientes de categorías de avisos.

### No objetivos
- Diálogo conversacional abierto por LLM (TMC no es un bot conversacional ni requiere GPUs masivas).
- Reconocimiento de voz / entrada de micrófono (STT) — eso corresponde a la Fase 10 (VoiceAttack / atajos).
- Mezclador de audio complejo o efectos DSP de radio (en F7 nos centramos en inteligibilidad y rendimiento).

---

## 3. Arquitectura y Componentes

```
[ Journal Events / GameState ]
            │
            ▼
    [ VoiceAdvisor ] (tmc-core / tmc-modules)
            │ (filtra categorías activas + cooldowns)
            ▼
    [ VoiceCue { priority, text_key, args } ]
            │
            ▼
     [ tmc-i18n ] ──> Localiza texto al idioma seleccionado
            │
            ▼
       [ tmc-tts ] ─── Cola priorizada (Emergency > Tactical > Info)
            │
            ├───────────────┬───────────────┐
            ▼                               ▼
    [ NativeEngine ]                [ KokoroEngine ]
    (Windows Speech API)          (Worker ONNX / Proceso aislado)
            │                               │
            └───────────────┬───────────────┘
                            ▼
                    [ Audio Output ]
```

---

## 4. Criterios de Aceptación de F7

1. **Rendimiento:** Con el motor nativo activo, el consumo de RAM de TMC no aumenta más de 5 MB y el uso de CPU en reposo permanece en 0.0%. Con Kokoro activo, el subproceso se carga bajo demanda y se suspende tras 60s sin locuciones.
2. **Prioridad y descarte:** Un aviso de «Escudos caídos» interrumpe o precede de inmediato a un aviso informativo de «Sistema explorado». Avisos encolados con más de 4s de retraso son descartados silenciosamente.
3. **Categorías configurables:** Desactivar «Minería» silencia al 100% las alertas de prospectores sin afectar a los avisos de navegación o combate.
4. **Prueba en Ajustes:** El botón «Probar voz» reproduce instantáneamente la locución de prueba seleccionada en el idioma y volumen actuales.
5. **Calidad y Tests:** Cobertura de tests unitarios para la cola de prioridades, deduplicación de alertas, selección de motores y archivos FTL completos en los 5 idiomas.
