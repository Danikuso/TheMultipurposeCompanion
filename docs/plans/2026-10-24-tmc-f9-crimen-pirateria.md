# TMC · Plan de Ejecución de F9 — Crimen, Notoriedad y Piratería (Law & Disorder)

- **Spec activa:** [`docs/specs/2026-10-24-tmc-f9-crimen-pirateria-design.md`](../specs/2026-10-24-tmc-f9-crimen-pirateria-design.md)
- **Rama:** `fase/f9-crimen-pirateria`
- **Enfoque:** TDD estricto (test → implementación mínima → verde → clippy/fmt → commit).

---

## Tareas de ejecución

### Bloque 1: Modelos de datos y eventos (`tmc-model`)

#### Tarea 1: Modelos de crimen, eventos de diario y topic `CrimeStatusTopic` (`tmc-model`)
- **Archivos:** `crates/tmc-model/src/crime.rs`, `crates/tmc-model/src/events.rs`, `crates/tmc-model/src/lib.rs`
- **Interfaces:**
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "snake_case")]
  pub enum CrimeType {
      Assault,
      Murder,
      Piracy,
      RecklessWeaponsDischarge,
      IllegalCargo,
      DisobeyPolice,
      DockingMajorBlockingAvenue,
      Other(String),
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct CommitCrimeEvent {
      pub crime_type: CrimeType,
      pub faction: String,
      pub victim: Option<String>,
      pub fine: Option<u64>,
      pub bounty: Option<u64>,
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct PayFinesEvent {
      pub amount: u64,
      pub broker_percentage: Option<f64>,
      pub all_fines: bool,
      pub faction: Option<String>,
      pub ship_id: Option<u64>,
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct PayBountiesEvent {
      pub amount: u64,
      pub broker_percentage: Option<f64>,
      pub all_bounties: bool,
      pub faction: Option<String>,
      pub ship_id: Option<u64>,
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct FactionFineBounty {
      pub faction: String,
      pub system: Option<String>,
      pub fines: u64,
      pub bounties: u64,
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct NearbyFacility {
      pub station_name: String,
      pub system_name: String,
      pub distance_ly: f64,
      pub pad_size: String,
      pub is_planetary: bool,
  }

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  pub struct CrimeStatusTopic {
      pub legal_state: String, // "Clean", "IllegalCargo", "Wanted", "Hostile", etc.
      pub notoriety: u8,       // 0..10
      pub notoriety_decay_seconds: u32,
      pub total_fines: u64,
      pub total_bounties: u64,
      pub factions: Vec<FactionFineBounty>,
      pub stolen_items: Vec<StolenCargoItem>,
      pub nearest_interstellar_factor: Option<NearbyFacility>,
      pub nearest_black_market: Option<NearbyFacility>,
      pub system_security: String, // "High", "Medium", "Low", "Anarchy"
      pub police_response_estimate_seconds: Option<u32>,
  }
  ```
- **Tests concretos:**
  - Deserialización de `CommitCrime` con `Fine` o con `Bounty`.
  - Deserialización de `PayFines` y `PayBounties`.
  - Serialización y deserialización de `CrimeStatusTopic`.
- **Verificación:** `cargo test -p tmc-model`
- **Commit:** `feat(model): modelos de crimen, notoriedad, facciones y topic de estado legal F9`

---

### Bloque 2: Lógica de negocio del módulo `crime` (`tmc-modules`)

#### Tarea 2: Implementación de `CrimeModule` en Rust (`tmc-modules`)
- **Archivos:** `crates/tmc-modules/src/crime/mod.rs`, `crates/tmc-modules/src/lib.rs`
- **Lógica:**
  - Rastreo en tiempo real de `Status.json` (`legal_state`, `flags`: supercrucero/espacio vs atracado).
  - Temporizador de vuelo: acumula delta time en vuelo para decrementar notoriedad cada 7.200 s (2 horas). Al atracar (`Flags::DOCKED`), el temporizador se pausa.
  - Gestión de multas y recompensas ante eventos `CommitCrime`, `PayFines`, `PayBounties` y `Resurrect` (al morir, las recompensas activas pueden pasar a dormant/liquidarse según reglas del juego).
  - Inspección de `Cargo` para extraer mercancías con `stolen > 0`.
  - Resolución del Interstellar Factor más cercano (filtrando sistemas/estaciones con servicio `Facilitator` o `InterstellarFactorsContact`) y Mercado Negro más cercano (`BlackMarket`).
  - Publicación del topic `crime.status`.
- **Tests concretos:**
  - Decaimiento de notoriedad: 7.200 s de vuelo decrementan 1 punto; atracado no decrementa.
  - `CommitCrime` suma multas y añade la facción si no existía.
  - `PayFines` deduce o limpia multas.
  - Carga robada genera entrada en `stolen_items`.
- **Verificación:** `cargo test -p tmc-modules`
- **Commit:** `feat(modules): modulo crime con decaimiento de notoriedad, calculo de multas y servicios cercanos`

---

### Bloque 3: Integración de avisos de cabina (`tmc-core` / `tmc-tts`)

#### Tarea 3: Detección táctica en `VoiceAdvisor` (`tmc-core`)
- **Archivos:** `crates/tmc-core/src/voice_advisor.rs`, `crates/tmc-core/src/actor.rs`
- **Lógica:**
  - Emisión de `VoiceCue` con prioridad táctica ante:
    - Ganancia de notoriedad (`tts-crime-notoriety-gain`).
    - Notoriedad limpia (`tts-crime-notoriety-cleared`).
    - Salto a sistema de alta o media seguridad portando cargamento ilegal (`tts-crime-illegal-cargo`).
    - Interdicción policial (`tts-crime-police-interdiction`).
  - Cooldowns anti-spam específicos para no saturar al piloto en situaciones de combate.
- **Tests concretos:**
  - Salto con carga ilegal emite aviso de contrabando una sola vez.
  - Incremento de notoriedad emite aviso táctico de seguridad.
- **Verificación:** `cargo test -p tmc-core`
- **Commit:** `feat(core): avisos contextuales de seguridad y contrabando en voice_advisor`

---

### Bloque 4: Localización en 5 idiomas (`locales/`)

#### Tarea 4: Catálogo de textos y locuciones en los 5 idiomas (`locales/`)
- **Archivos:** `locales/{es,en,fr,de,pt}/crime.ftl`
- **Cadenas:**
  - Claves para UI: `crime-title`, `crime-notoriety`, `crime-legal-state`, `crime-clean`, `crime-wanted`, `crime-fines`, `crime-bounties`, `crime-interstellar-factor`, `crime-black-market`, `crime-notoriety-countdown`, `crime-guide-title`, `crime-guide-blocked`, `crime-guide-ready`, etc.
  - Claves de voz: `tts-crime-notoriety-gain`, `tts-crime-notoriety-cleared`, `tts-crime-illegal-cargo`, `tts-crime-police-interdiction`.
  - Sincronización exacta en los 5 idiomas verificada por `tmc-i18n`.
- **Verificación:** `cargo test -p tmc-i18n`
- **Commit:** `feat(i18n): catalogo de textos legales, guia de limpieza y voz en los 5 idiomas`

---

### Bloque 5: Interfaz de usuario («TMC Glass HUD») (`ui/`)

#### Tarea 5: Componente `CrimePanel.svelte` e integración en el Shell (`ui/`)
- **Archivos:** `ui/src/modules/crime/CrimePanel.svelte`, `ui/src/App.svelte`, `ui/src/shell/Sidebar.svelte`, `ui/src/lib/nav.ts`
- **Contenido:**
  - Navegación: nuevo botón en la barra lateral con icono `⚖️` y atajo rápido.
  - Tarjeta 1: *Estado Legal y Notoriedad* (semáforo de estado, medidor de 0 a 10 con colores HUD y contador de decaimiento en vuelo).
  - Tarjeta 2: *«Guía de Limpieza de Nombre del Ayudante»* (explicación pedagógica de notoriedad si > 0; tarjeta del Interstellar Factor más cercano con botón de 1 clic para copiar si notoriedad = 0).
  - Tarjeta 3: *Contrabando y Mercados Negros* (resumen de bodega robada, consejos tácticos de entrada y Mercado Negro más cercano con botón de copia).
  - Tarjeta 4: *Desglose de Delitos y Recompensas* (acordeón por facción y sistema).
- **Verificación:** `cmd /c "npm --prefix ui run check"`
- **Commit:** `feat(ui): panel de cabina glass hud para crimen, notoriedad, contrabando y limpieza de nombre`

#### Tarea 6: Tests unitarios en Vitest (`ui/`)
- **Archivos:** `ui/tests/crime.test.ts`
- **Pruebas:**
  - Estado limpio sin multas muestra badge de conformidad legal.
  - Notoriedad > 0 muestra bloqueo de Interstellar Factors y temporizador de decaimiento.
  - Notoriedad 0 con multas muestra el Interstellar Factor más cercano con botón de copia.
  - Carga robada activa la sección de contrabando y mercado negro.
- **Verificación:** `cmd /c "npm --prefix ui test -- --run"`
- **Commit:** `test(ui): pruebas unitarias para el panel de crimen y guia de limpieza de nombre`

---

### Bloque 6: Verificación integral, huella y compilación

#### Tarea 7: Verificación completa, footprint y compilación de binarios Release/Debug
- **Cambios:**
  - Comprobación de que toda la suite de Rust (`cargo test --workspace`) y Vitest pasa en verde.
  - Verificación de clippy (`cargo clippy --workspace --all-targets -- -D warnings`).
  - Svelte check y Vite build de producción.
  - Medición de huella (< 150 MB de RAM y < 1% CPU en reposo).
  - Compilación de `target/release/TMC.exe` y `target/debug/TMC.exe`.
  - Actualización de `status.md`.
- **Verificación:** Suite global y comprobación de binarios.
- **Commit:** `docs: actualizacion de status.md con cierre de F9 y binarios release`
