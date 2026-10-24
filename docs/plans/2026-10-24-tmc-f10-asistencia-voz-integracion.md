# Plan: TMC Fase 10 — Asistencia e Integración de Voz (Zero-Friction Voice Assistant)

- **Spec:** [`docs/specs/2026-10-24-tmc-f10-asistencia-voz-integracion-design.md`](file:///d:/Proyectos/EDPyCompanion/docs/specs/2026-10-24-tmc-f10-asistencia-voz-integracion-design.md)
- **Rama:** `fase/f10-asistencia-voz-integracion`
- **Total tareas:** 7 tareas organizadas en 6 bloques secuenciales.

---

### Bloque 1: Modelos de datos y tipos de intenciones (`tmc-model`)

#### Tarea 1: Modelos de intenciones de voz, petición/respuesta y estado de VoiceAttack
- **Archivos:** `crates/tmc-model/src/voice_intent.rs`, `crates/tmc-model/src/lib.rs`, `crates/tmc-model/src/diagnostics.rs`
- **Modelos:**
  - `VoiceIntent`: variantes `ShipStatus`, `NextJump`, `CopyNextSystem`, `NearestInterstellarFactor`, `NearestBlackMarket`, `CrimeStatus`, `BioSignals`, `MaterialsSummary`, `FuelStatus`, `Unknown(String)`.
  - `VoiceCommandRequest`: campo `query` y `locale`.
  - `VoiceCommandResponse`: `intent`, `action_executed`, `spoken_text`, `clipboard_copied`.
  - `VoiceAttackStatus`: `installed: bool`, `running: bool`, `profile_available: bool`.
- **Verificación:** `cargo test -p tmc-model`
- **Commit:** `feat(model): modelos de intenciones de voz, peticiones y puente voiceattack F10`

---

### Bloque 2: Intérprete semántico de lenguaje natural contra el estado de juego (`tmc-core`)

#### Tarea 2: `VoiceIntentResolver` semántico en Rust
- **Archivos:** `crates/tmc-core/src/voice_intent_resolver.rs`, `crates/tmc-core/src/lib.rs`
- **Lógica:**
  - Normalización y detección léxica de patrones para las intenciones en los 5 idiomas (`es`, `en`, `fr`, `de`, `pt`).
  - Resolución en tiempo real contra `GameState` (ubicación, nave, combustible, multas, notoriedad, siguiente salto, bio-señales).
  - Emisión de `VoiceCue` con la respuesta hablada generada para que el piloto escuche la respuesta inmediatamente.
- **Tests concretos:**
  - Consulta "¿Estado de la nave?" devuelve métricas de casco/escudos/fuel.
  - Consulta "¿Cuál es el siguiente salto?" devuelve el sistema en ruta.
  - Consulta "¿Dónde está el factor interestelar más cercano?" devuelve la estación y marca la copia al portapapeles.
  - Pruebas equivalentes en inglés y otros idiomas.
- **Verificación:** `cargo test -p tmc-core`
- **Commit:** `feat(core): interprete semantico de intenciones de voz contra el estado de juego`

---

### Bloque 3: Endpoints REST y puente VoiceAttack (`tmc-server`, `tmc-app`)

#### Tarea 3: Endpoints REST de voz y generador de perfil VoiceAttack
- **Archivos:** `crates/tmc-server/src/rest.rs`, `crates/tmc-app/src/lib.rs`, `crates/tmc-app/src/voiceattack.rs`
- **Lógica:**
  - `POST /api/voice/intent`: procesa la consulta de voz, ejecuta la acción (copiar al portapapeles si corresponde) y vocaliza la respuesta vía `TtsQueue`.
  - `GET /api/voice/voiceattack`: detecta en segundo plano la presencia de VoiceAttack y genera el perfil XML `.vap` oficial de TMC sincronizado con los comandos REST.
- **Verificación:** `cargo test -p tmc-server`, `cargo test -p tmc-app`
- **Commit:** `feat(server): endpoints rest de voz, accion de portapapeles y perfil voiceattack`

---

### Bloque 4: Catálogo i18n en los 5 idiomas (`locales/`)

#### Tarea 4: Catálogo de respuestas vocales y patrones léxicos
- **Archivos:** `locales/{es,en,fr,de,pt}/voice_assistant.ftl`
- **Cadenas:**
  - Respuestas vocales para estado de nave, siguiente salto, factores interestelares, mercados negros, bio-señales y multas.
  - Etiquetas de UI para el asistente de voz y monitor de VoiceAttack.
- **Verificación:** `cargo test -p tmc-i18n`
- **Commit:** `feat(i18n): catalogo de respuestas del asistente de voz y textos de cabina en los 5 idiomas`

---

### Bloque 5: Interfaz de usuario («TMC Glass HUD») y motor de escucha (`ui/`)

#### Tarea 5: Componente de voz en TopBar, gestor de escucha y sección de ajustes
- **Archivos:** `ui/src/lib/voiceAssistant.svelte.ts`, `ui/src/shell/TopBar.svelte`, `ui/src/settings/Settings.svelte`
- **Contenido:**
  - Gestor de escucha reactivo: conmutador maestro «Activar asistencia por voz».
  - Reconocimiento de voz continuo y modo PTT.
  - Píldora animada en el TopBar con estados HUD: Inactivo, Escuchando, Procesando, Hablando.
  - Detección automática y visualización del estado de VoiceAttack en Ajustes.
- **Verificación:** `npm --prefix ui run check`
- **Commit:** `feat(ui): asistente de voz en topbar, escucha continua y controles en ajustes`

---

### Bloque 6: Tests y Verificación integral

#### Tarea 6: Tests unitarios en Vitest para la asistencia de voz (`ui/`)
- **Archivos:** `ui/tests/voice_assistant.test.ts`
- **Tests:** Activación de voz, alternancia de estados HUD, llamada a `/api/voice/intent` y persistencia de preferencias.
- **Verificación:** `npm --prefix ui test -- tests/voice_assistant.test.ts --run`
- **Commit:** `test(ui): pruebas unitarias para el asistente de voz y estados de escucha`

#### Tarea 7: Verificación global, compilación de binarios Release/Debug y cierre
- **Pasos:**
  - Workspace Rust: `cargo test --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`.
  - Suite Vitest: `npm --prefix ui test -- --run` y `npm --prefix ui run build`.
  - Binarios interactivos: `cargo build -p tmc-desktop --release` y `cargo build -p tmc-desktop`.
  - Verificación de huella y saneamiento de disco.
- **Commit:** `docs: actualizacion de status.md con cierre de F10 y binarios interactivos`
