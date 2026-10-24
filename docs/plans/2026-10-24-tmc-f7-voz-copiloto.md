# TMC · Plan de ejecución de F7 — Voz, Alertas de Cabina y Copiloto Auditivo

- **Fecha:** 2026-10-24
- **Spec de referencia:** [`docs/specs/2026-10-24-tmc-f7-voz-copiloto-design.md`](../specs/2026-10-24-tmc-f7-voz-copiloto-design.md)
- **Rama de trabajo:** `fase/f7-voz-copiloto`
- **Estado:** 🔄 en propuesta / pendiente de aprobación del propietario

---

## Bloques de trabajo y tareas

### Bloque 0: Modelos de datos y configuración (`tmc-model`)

#### Tarea 1: Modelos de configuración de voz, canales y avisos tácticos (`tmc-model`)
- **Archivos:** `crates/tmc-model/src/tts.rs`, `crates/tmc-model/src/settings.rs`, `crates/tmc-model/src/lib.rs`
- **Cambios:**
  - `TtsEngineKind`: `Native` (Windows OneCore/SAPI5), `Kokoro` (neuronal ONNX).
  - `TtsChannel`: `Combat`, `Exploration`, `Navigation`, `Mining`, `Carrier`.
  - `TtsSettings`:
    - `enabled: bool` (por defecto false para opt-in limpio).
    - `engine: TtsEngineKind`.
    - `voice: Option<String>`.
    - `volume: u8` (0..100, por defecto 80).
    - `rate: f32` (0.8..1.5, por defecto 1.0).
    - `channels: HashMap<TtsChannel, bool>`.
  - `VoiceCue`: `priority` (`Emergency`, `Tactical`, `Informational`), `text_key: String`, `args: BTreeMap<String, String>`, `created_at: Instant`, `channel: TtsChannel`.
  - Integración en `AppSettings` y serialización JSON.
- **Verificación:** `cargo test -p tmc-model tts`
- **Commit:** `feat(model): modelos de configuración y avisos de voz para copiloto auditivo F7`

---

### Bloque 1: Crate del motor de audio y sintetizador (`tmc-tts`)

#### Tarea 2: Cola de locución priorizada con descarte y deduplicación (`tmc-tts`)
- **Archivos:** `crates/tmc-tts/Cargo.toml`, `crates/tmc-tts/src/lib.rs`, `crates/tmc-tts/src/queue.rs`
- **Cambios:**
  - Nuevo crate en el workspace `crates/tmc-tts`.
  - Trait `TtsEngine`: `fn speak(&mut self, text: &str, voice: Option<&str>, volume: u8, rate: f32) -> Result<(), TtsError>`, `fn available_voices(&self) -> Vec<VoiceInfo>`, `fn stop(&mut self)`.
  - `TtsQueue`:
    - Encolado con prioridades: `Emergency` interrumpe locución actual; `Tactical` se sitúa al frente; `Informational` a la cola.
    - Descarte de avisos viejos: cualquier aviso con más de 4 segundos de retraso en cola se descarta automáticamente para evitar hablar a destiempo.
    - Cooldown y deduplicación: mapa de última emisión por clave para evitar repetir la misma alerta en bucle (ej. temperatura alta máximo 1 cada 15s).
- **Verificación:** `cargo test -p tmc-tts queue`
- **Commit:** `feat(tts): crate tmc-tts con cola priorizada, descarte de avisos obsoletos y cooldown`

#### Tarea 3: Motor nativo ultraligero de Windows (`tmc-tts`)
- **Archivos:** `crates/tmc-tts/src/native.rs`
- **Cambios:**
  - Implementación de `TtsEngine` para el sistema operativo mediante el sintetizador nativo de Windows (OneCore / SAPI5).
  - Emisión asíncrona no bloqueante sin dependencias externas pesadas ni descargas.
  - Enumeración de voces instaladas en Windows (español Helena/Laura/Pablo, inglés Zira/David, etc.).
- **Verificación:** `cargo test -p tmc-tts native`
- **Commit:** `feat(tts): motor nativo ligero para windows con enumeracion de voces locales`

#### Tarea 4: Motor neuronal KokoroTTS desacoplado (`tmc-tts`)
- **Archivos:** `crates/tmc-tts/src/kokoro.rs`
- **Cambios:**
  - Integración de arquitectura Kokoro para inferencia ONNX en subproceso/worker desacoplado.
  - Carga bajo demanda solo si el usuario selecciona el motor neuronal en ajustes.
  - Descarga de memoria tras inactividad prolongada (> 60s sin locuciones).
  - Manejo de idiomas soportados (`es`, `en`, `fr`, `pt`) y fallback a motor nativo si el idioma es `de` (alemán, conforme a R12).
- **Verificación:** `cargo test -p tmc-tts kokoro`
- **Commit:** `feat(tts): motor neuronal kokoro en subproceso con carga bajo demanda y suspension`

---

### Bloque 2: Generador de avisos de cabina (`tmc-core` / `tmc-modules`)

#### Tarea 5: Generador táctico `VoiceAdvisor` en el flujo de eventos (`tmc-core`)
- **Archivos:** `crates/tmc-core/src/voice_advisor.rs`, `crates/tmc-core/src/lib.rs`, `crates/tmc-core/src/actor.rs`
- **Cambios:**
  - Detección de disparadores contextuales del diario:
    - **Navegación:** `FSDJump` -> Nombre del sistema + aviso de estrella repostable (`K, G, B, F, O, A, M`) o no repostable.
    - **Exploración:** `FSSDiscoveryScan` / `FSSAllBodiesFound` -> Total de cuerpos con bio-señales valiosas.
    - **Exobiología:** `ScanOrganic` -> Progreso de muestra biológica (1/3, 2/3, 3/3 completada) y distancia de clonación.
    - **Combate:** `ShieldState` (escudos caídos/restaurados), sobrecalentamiento (> 80%, daño térmico), daño grave de casco (< 50%, < 25%), interdicción.
    - **Minería:** `ProspectedAsteroid` -> Aviso si contiene mineral de interés > umbral configurado; `Cargo` al 100% de capacidad.
    - **Carrier:** Cuenta atrás de salto a 5 min y 1 min, y salto completado.
  - Despacho de `VoiceCue` a `tmc-tts` pasando por la localización de `tmc-i18n`.
- **Verificación:** `cargo test -p tmc-core voice_advisor`
- **Commit:** `feat(core): generador de avisos contextuales de cabina voice_advisor`

---

### Bloque 3: Endpoints y persistencia (`tmc-server` / `tmc-app` / `tmc-store`)

#### Tarea 6: Persistencia SQLite de configuración y endpoints REST de prueba (`tmc-store`, `tmc-server`, `tmc-app`)
- **Archivos:** `crates/tmc-store/src/settings.rs`, `crates/tmc-server/src/rest.rs`, `crates/tmc-app/src/lib.rs`
- **Cambios:**
  - Persistencia de `tts_settings` en la tabla `settings` de SQLite.
  - Endpoint `GET /api/tts/voices`: devuelve las voces disponibles del motor configurado.
  - Endpoint `POST /api/tts/test`: sintetiza una locución de prueba inmediata con el motor, voz, volumen y velocidad solicitados.
  - Endpoint `PUT /api/settings`: actualización en vivo de ajustes de voz sin reiniciar la aplicación.
- **Verificación:** `cargo test -p tmc-server tts`, `cargo test -p tmc-store settings`
- **Commit:** `feat(server): endpoints de voces disponibles, prueba auditiva y persistencia de ajustes de voz`

---

### Bloque 4: Localización en 5 idiomas (`locales/`)

#### Tarea 7: Catálogo de locuciones de copiloto en los 5 idiomas (`locales/`)
- **Archivos:** `locales/{es,en,fr,de,pt}/tts.ftl`
- **Cambios:**
  - Claves de avisos concisos de cabina:
    - `tts-nav-jump-scoopable = Salto completado a { $system }. Estrella repostable.`
    - `tts-nav-jump-unscoopable = Salto completado a { $system }. Advertencia: estrella no repostable.`
    - `tts-nav-bio-signals = Detectadas señales biológicas en { $count } cuerpos.`
    - `tts-exo-sample-progress = Muestra biológica { $current } de { $total } registrada.`
    - `tts-exo-sample-complete = Análisis de especie completado.`
    - `tts-combat-shields-down = ¡Alerta! Escudos caídos.`
    - `tts-combat-shields-up = Escudos restaurados.`
    - `tts-combat-heat-warning = Advertencia: temperatura al { $pct } por ciento.`
    - `tts-combat-hull-warning = ¡Alerta de casco! Integridad estructural al { $pct } por ciento.`
    - `tts-mining-valuable-find = Asteroide con { $pct } por ciento de { $mineral }.`
    - `tts-carrier-jump-imminent = Salto de Fleet Carrier en un minuto.`
    - `tts-test-phrase = Copiloto de TMC activo y listo para la navegación.`
  - Claves para la interfaz de ajustes (`settings-tts-title`, `settings-tts-engine`, `settings-tts-voice`, `settings-tts-test`, etc.).
  - Sincronización idéntica en los 5 idiomas verificada por tests de `tmc-i18n`.
- **Verificación:** `cargo test -p tmc-i18n`
- **Commit:** `feat(i18n): catalogo de locuciones de copiloto de cabina en los 5 idiomas`

---

### Bloque 5: Interfaz de usuario (`ui/`)

#### Tarea 8: Sección de Copiloto Auditivo en `SettingsPanel.svelte` (`ui/`)
- **Archivos:** `ui/src/modules/settings/SettingsPanel.svelte`, `ui/src/modules/settings/TtsSettingsCard.svelte` (o tarjeta integrada)
- **Cambios:**
  - Tarjeta «TMC Glass HUD» para el Copiloto Auditivo:
    - Interruptor general: *Copiloto de voz (Avisos de cabina)*.
    - Selector desplegable de motor: *Nativo de Windows (Ultraligero)* vs *Kokoro Neuronal (Voz IA Espacial)*.
    - Selector de voz según el motor activo.
    - Deslizadores (*sliders*) para volumen (0-100%) y velocidad (0.8x a 1.5x).
    - Conmutadores (*checkboxes*) por categoría: Combate, Exploración, Navegación, Minería, Fleet Carrier.
    - Botón de acción: *🔊 Probar voz de copiloto*.
  - Tests unitarios en Vitest para interacción y persistencia reactiva.
- **Verificación:** `cmd /c "npm --prefix ui test -- --run"`, `cmd /c "npm --prefix ui run check"`
- **Commit:** `feat(ui): controles de copiloto de voz en ajustes con selector de motor y prueba de audio`

---

### Bloque 6: Verificación integral, footprint y compilación

#### Tarea 9: Verificación completa de suite, huella de memoria y compilación Release/Debug
- **Cambios:**
  - Ejecución de la suite completa de tests de Rust y Vitest.
  - Verificación de huella de memoria (< 150 MB de RAM y < 1% CPU en reposo).
  - Compilación de ejecutables interactivos `target/release/TMC.exe` y `target/debug/TMC.exe`.
  - Actualización de `status.md`.
- **Verificación:** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cmd /c "npm --prefix ui run check"`
- **Commit:** `docs: actualizacion de status.md con cierre de F7 y binarios de release`
