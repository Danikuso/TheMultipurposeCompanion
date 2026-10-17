# TMC F0 «Cimientos» — Plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construir la base de TMC: leer el diario y `Status.json` en tiempo real, mantener el estado del juego, guardarlo en SQLite (incluido todo el historial), servirlo por WebSocket a una UI Svelte con colores del HUD e i18n, empaquetado en una app Tauri 2 con instalador NSIS, y un panel general funcional.

**Architecture:** Workspace de Cargo con crates pequeños (`tmc-model` → tipos y estado puro; `tmc-journal` → lectura de archivos; `tmc-store` → SQLite; `tmc-core` → actor de estado, módulos y topics; `tmc-server` → axum HTTP/WS; `tmc-app` → composición). Las apps (`apps/desktop` Tauri, `apps/headless`) solo componen. La UI (`ui/`) se sirve desde el servidor local tanto a la ventana Tauri como a navegadores en la LAN.

**Tech Stack:** Rust estable (edición 2024), tokio, serde/serde_json, chrono, bitflags, rusqlite (bundled), axum, rust-embed, fluent-bundle, notify, sysinfo, arboard, qrcode, directories, tracing; Tauri 2; Svelte 5 + TypeScript + Vite, @fluent/bundle, Vitest, Playwright.

**Spec:** `docs/specs/2026-10-17-tmc-arquitectura-y-f0-design.md` (léela antes de empezar). Datos de referencia: `docs/research/notes/local_game_data.md`.

## Global Constraints

- Idioma de código/identificadores: inglés. Idioma de docs, commits y textos de UI de referencia: español.
- Rust edición 2024, toolchain estable fijado en `rust-toolchain.toml`; `cargo clippy --workspace --all-targets -- -D warnings` limpio.
- Licencia: PolyForm Noncommercial 1.0.0. **Prohibido** copiar código o datos GPL/AGPL al repo.
- Puerto por defecto del servidor: **47320** (si está ocupado, el siguiente libre). Escucha en `127.0.0.1` salvo modo LAN.
- Sondeo del diario: **250 ms** con juego activo, **1000 ms** sin juego. Status.json: **250 ms**. Comprobación de diario nuevo: **1 s**. Comprobación del proceso del juego: **10 s**.
- Reintentos de archivos compañeros/Status: hasta **10 × 50 ms**.
- Difusión a la UI: como máximo **10 Hz** (cada 100 ms), solo topics cambiados, valor completo.
- SQLite: WAL, `synchronous=NORMAL`; escritor en lotes (commit cada **500 ms o 500 filas**); importación en transacciones de **5 000 filas**.
- Presupuesto: < **150 MB** RAM (árbol de procesos) y < **1 %** CPU en reposo; p95 evento→UI < **500 ms**.
- UI: sin animaciones infinitas ni `requestAnimationFrame` continuo; transiciones < 200 ms solo ante cambios.
- Idiomas: `es` (referencia), `en`, `fr`, `de`, `pt`. Todas las cadenas visibles salen de `locales/<lang>/*.ftl`.
- Nunca enviar ni subir nada a servicios online en F0. Nunca escribir secretos en el repo (`.env` ignorado).
- Commits pequeños, mensaje en español con prefijo convencional (`feat:`, `test:`, `chore:`…) y la línea `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Cambio de archivo de diario a mitad de sesión** (el juego crea `Journal.*.02.log` o una sesión nueva sin cerrar TMC): no se pierde ni se duplica ninguna línea → test `watcher_switches_to_new_journal_without_losing_lines` (Task 7).
2. **Línea a medio escribir al final del archivo** (el juego aún no escribió el `\n`): nunca se parsea media línea; se completa en la siguiente lectura → test `tailer_holds_partial_line_until_newline` (Task 6).
3. **`Status.json` leído vacío o truncado** mientras el juego lo reescribe: el estado anterior se conserva, no se borra ni se emite un Status vacío → test `status_poller_ignores_empty_and_truncated_reads` (Task 7).
4. **TMC arranca con la partida ya empezada**: el estado se reconstruye desde el diario actual **sin** disparar alertas de eventos pasados → test `replayed_events_do_not_raise_alerts` (Task 11).
5. **Rutas con espacios, acentos o carpetas redirigidas** (p. ej. OneDrive, «Partidas guardadas»): la localización y lectura funcionan → test `reads_journal_from_path_with_spaces_and_accents` (Task 5).

---

## Mapa de archivos

```
Cargo.toml                         workspace + dependencias compartidas
rust-toolchain.toml  rustfmt.toml  clippy.toml
LICENSE  THIRD_PARTY_NOTICES.md  README.md
.github/workflows/ci.yml
crates/tmc-model/src/{lib.rs, status.rs, events.rs, companion.rs, state.rs, source.rs, theme.rs}
crates/tmc-journal/src/{lib.rs, locate.rs, files.rs, tail.rs, status_poller.rs, companion_reader.rs, watcher.rs, process.rs, hud.rs}
crates/tmc-store/src/{lib.rs, migrations.rs, writer.rs, import.rs, query.rs}
crates/tmc-core/src/{lib.rs, module.rs, actor.rs, topics.rs, alert.rs}
crates/tmc-modules/src/{lib.rs, registry.rs, dashboard/mod.rs, dashboard/rules.rs}
crates/tmc-i18n/src/lib.rs
crates/tmc-server/src/{lib.rs, ws.rs, rest.rs, security.rs, assets.rs, pairing.rs}
crates/tmc-app/src/{lib.rs, config.rs, logging.rs, wiring.rs, clipboard.rs}
crates/tmc-replay/src/{main.rs, replay.rs, anonymize.rs}
apps/headless/src/main.rs
apps/desktop/{Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json, icons/, src/main.rs, src/tray.rs, src/window.rs}
locales/{es,en,fr,de,pt}/main.ftl
ui/{package.json, vite.config.ts, tsconfig.json, index.html, src/...}
tests/fixtures/journal/*.log  tests/fixtures/status/*.json
scripts/measure-footprint.ps1
```

---

### Task 0: Entorno de desarrollo

**Files:** ninguno (solo instalación local).

**Interfaces:** — Produces: `cargo`, `rustc` estables con target MSVC, `cargo tauri` CLI, Node 24 / npm 11 (ya instalados).

- [ ] **Step 1: Instalar Visual Studio Build Tools 2022 con C++** (necesario para el target MSVC de Rust y WebView2)

Run: `winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
Expected: termina con código 0 (puede pedir UAC al usuario).

- [ ] **Step 2: Instalar rustup y toolchain estable**

Run: `winget install --id Rustlang.Rustup` y en una terminal nueva `rustup default stable && rustup component add clippy rustfmt`
Expected: `rustc --version` muestra `rustc 1.x.y` y `cargo clippy --version` responde.

- [ ] **Step 3: Instalar Tauri CLI**

Run: `cargo install tauri-cli --version "^2" --locked`
Expected: `cargo tauri --version` muestra `tauri-cli 2.x`.

---

### Task 1: Workspace, licencia y CI

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `LICENSE`, `THIRD_PARTY_NOTICES.md`, `.github/workflows/ci.yml`, `crates/tmc-model/{Cargo.toml,src/lib.rs}`
- Modify: `README.md`

**Interfaces:**
- Produces: workspace con `members = ["crates/*", "apps/headless", "apps/desktop"]` (los dos `apps/*` se añaden a `members` en sus tasks); `[workspace.dependencies]` con versiones fijadas para todas las crates del Tech Stack; `[workspace.package] edition = "2024", license-file = "LICENSE"`.

- [ ] **Step 1: Crear el workspace** con `crates/tmc-model` como único miembro (lib vacía con `#![forbid(unsafe_code)]`), `rust-toolchain.toml` (`channel = "stable"`, componentes clippy y rustfmt), `rustfmt.toml` (`edition = "2024"`, `max_width = 100`).
- [ ] **Step 2: Licencia y avisos.** `LICENSE` = texto íntegro de PolyForm Noncommercial 1.0.0 con `Required Notice: Copyright (c) 2026 Danikuso (https://github.com/Danikuso/TheMultipurposeCompanion)`. `THIRD_PARTY_NOTICES.md` con sección por dependencia directa (se rellena al añadirlas). Añadir al `README.md` el aviso: «Elite Dangerous es una marca de Frontier Developments plc. TMC no está afiliado ni respaldado por Frontier.»
- [ ] **Step 3: CI.** `.github/workflows/ci.yml`: jobs en `windows-latest` y `ubuntu-latest` con `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; en Ubuntu instalar antes `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev`. Job `ui` (ubuntu): `npm ci && npm run check && npm test && npm run build` en `ui/` (se activa cuando exista `ui/package.json`: condición `hashFiles('ui/package.json') != ''`).
- [ ] **Step 4: Verificar**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: todo OK, 0 tests.

- [ ] **Step 5: Commit** `chore: workspace de Cargo, licencia PolyForm NC y CI`

---

### Task 2: `tmc-model` — Status.json

**Files:**
- Create: `crates/tmc-model/src/status.rs`, `tests/fixtures/status/{docked.json,menu.json,on_foot.json}`
- Modify: `crates/tmc-model/src/lib.rs`, `crates/tmc-model/Cargo.toml`

**Interfaces:**
- Produces:
  - `bitflags! pub struct Flags: u32` con constantes (bit): `DOCKED`0, `LANDED`1, `LANDING_GEAR_DOWN`2, `SHIELDS_UP`3, `SUPERCRUISE`4, `FLIGHT_ASSIST_OFF`5, `HARDPOINTS_DEPLOYED`6, `IN_WING`7, `LIGHTS_ON`8, `CARGO_SCOOP_DEPLOYED`9, `SILENT_RUNNING`10, `SCOOPING_FUEL`11, `SRV_HANDBRAKE`12, `SRV_TURRET_VIEW`13, `SRV_TURRET_RETRACTED`14, `SRV_DRIVE_ASSIST`15, `FSD_MASS_LOCKED`16, `FSD_CHARGING`17, `FSD_COOLDOWN`18, `LOW_FUEL`19, `OVERHEATING`20, `HAS_LAT_LONG`21, `IS_IN_DANGER`22, `BEING_INTERDICTED`23, `IN_MAIN_SHIP`24, `IN_FIGHTER`25, `IN_SRV`26, `HUD_ANALYSIS_MODE`27, `NIGHT_VISION`28, `ALTITUDE_FROM_AVERAGE_RADIUS`29, `FSD_JUMP`30, `SRV_HIGH_BEAM`31.
  - `bitflags! pub struct Flags2: u32`: `ON_FOOT`0, `IN_TAXI`1, `IN_MULTICREW`2, `ON_FOOT_IN_STATION`3, `ON_FOOT_ON_PLANET`4, `AIM_DOWN_SIGHT`5, `LOW_OXYGEN`6, `LOW_HEALTH`7, `COLD`8, `HOT`9, `VERY_COLD`10, `VERY_HOT`11, `GLIDE_MODE`12, `ON_FOOT_IN_HANGAR`13, `ON_FOOT_SOCIAL_SPACE`14, `ON_FOOT_EXTERIOR`15, `BREATHABLE_ATMOSPHERE`16, `TELEPRESENCE_MULTICREW`17, `PHYSICAL_MULTICREW`18, `FSD_HYPERDRIVE_CHARGING`19, `SCO_ACTIVE`20, `SUPERCRUISE_ASSIST`21, `NPC_CREW_ACTIVE`22. Bits desconocidos se conservan (`from_bits_retain`).
  - `pub enum GuiFocus { NoFocus, InternalPanel, ExternalPanel, CommsPanel, RolePanel, StationServices, GalaxyMap, SystemMap, Orrery, Fss, Saa, Codex, Unknown(u8) }` (valores 0–11).
  - `pub struct Status { timestamp: DateTime<Utc>, flags: Flags, flags2: Flags2, pips: Option<[u8;3]>, fire_group: Option<u8>, gui_focus: GuiFocus, fuel: Option<Fuel>, cargo: Option<f64>, legal_state: Option<String>, latitude/longitude/altitude/heading: Option<f64>, body_name: Option<String>, planet_radius: Option<f64>, balance: Option<i64>, destination: Option<Destination>, oxygen/health/temperature/gravity: Option<f64>, selected_weapon: Option<String> }`; `Fuel { main: f64, reservoir: f64 }`; `Destination { system: u64, body: u32, name: String, name_localised: Option<String> }`. Todos `Serialize + Deserialize + Clone + PartialEq + Debug`.
  - `pub fn Status::parse(text: &str) -> Result<Status, ParseError>`; `pub enum ParseError { Empty, Json(String), MissingField(&'static str) }` (en `lib.rs`, compartido).

- [ ] **Step 1: Tests** en `status.rs` (`#[cfg(test)]`):

```rust
#[test] fn parses_docked_flags() {
    let s = Status::parse(r#"{"timestamp":"2026-10-16T20:23:04Z","event":"Status","Flags":16842765}"#).unwrap();
    assert!(s.flags.contains(Flags::DOCKED | Flags::LANDING_GEAR_DOWN | Flags::SHIELDS_UP | Flags::FSD_MASS_LOCKED | Flags::IN_MAIN_SHIP));
    assert!(s.flags2.is_empty());
    assert_eq!(s.gui_focus, GuiFocus::NoFocus);
}
#[test] fn bit31_does_not_overflow() { /* Flags = 2147483648 → contains(SRV_HIGH_BEAM) */ }
#[test] fn parses_sco_and_on_foot_flags2() { /* Flags2 = (1<<20)|(1<<0) → SCO_ACTIVE y ON_FOOT */ }
#[test] fn unknown_gui_focus_is_preserved() { /* GuiFocus 42 → Unknown(42) */ }
#[test] fn empty_text_is_empty_error() { assert_eq!(Status::parse("  "), Err(ParseError::Empty)); }
#[test] fn truncated_json_is_json_error() { assert!(matches!(Status::parse(r#"{"timestamp":"2026"#), Err(ParseError::Json(_)))); }
#[test] fn parses_fixtures() { /* los 3 archivos de tests/fixtures/status parsean y on_foot.json tiene oxygen Some */ }
```

- [ ] **Step 2: Ejecutar** `cargo test -p tmc-model` → FAIL (no compila).
- [ ] **Step 3: Implementar** con serde (`#[serde(rename_all = "PascalCase")]`, `Fuel` desde `FuelMain`/`FuelReservoir`, `Name_Localised`); `Flags`/`Flags2` se deserializan desde `u64`/`u32` sin signo.
- [ ] **Step 4: Ejecutar** `cargo test -p tmc-model` → PASS.
- [ ] **Step 5: Commit** `feat(model): Status.json con flags, Flags2 y GuiFocus`

---

### Task 3: `tmc-model` — eventos del diario y archivos compañeros

**Files:**
- Create: `crates/tmc-model/src/events.rs`, `crates/tmc-model/src/companion.rs`, `tests/fixtures/journal/basic_session.log`
- Modify: `crates/tmc-model/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct JournalEvent { pub timestamp: DateTime<Utc>, pub name: String, pub kind: EventKind, pub raw: Box<RawValue> }`
  - `pub fn parse_line(line: &str) -> Result<JournalEvent, ParseError>` (acepta `\r` final; línea vacía → `Empty`; sin `timestamp`/`event` → `MissingField`).
  - `pub enum EventKind` con variantes tipadas (struct homónimo en `events.rs`, campos `Option` salvo los imprescindibles, `#[serde(default)]`): `Fileheader{part, language, gameversion, build}`, `Commander{fid, name}`, `LoadGame{fid, commander, odyssey, ship, ship_localised, ship_id, ship_name, ship_ident, fuel_level, fuel_capacity, game_mode, credits, loan, gameversion, build}`, `Rank{combat, trade, explore, soldier, exobiologist, empire, federation, cqc}`, `Progress{…mismos campos en %}`, `Location(LocationInfo)`, `FsdJump(LocationInfo)`, `CarrierJump(LocationInfo)`, `Docked{station_name, station_type, market_id, star_system, system_address}`, `Undocked{station_name}`, `SupercruiseEntry{star_system}`, `SupercruiseExit{star_system, body, body_type}`, `ApproachBody{body}`, `LeaveBody{body}`, `Touchdown`, `Liftoff`, `StartJump{jump_type, star_system}`, `FsdTarget{name, system_address, remaining_jumps_in_route}`, `NavRoute`, `NavRouteClear`, `Loadout{ship, ship_id, ship_name, ship_ident, hull_health, unladen_mass, cargo_capacity, max_jump_range, fuel_capacity: Option<FuelCapacity{main, reserve}>, rebuy, modules: Vec<LoadoutModule{slot, item, health}>}`, `Cargo{vessel, count}`, `HullDamage{health, player_pilot, fighter}`, `UnderAttack{target}`, `Interdicted{submitted, interdictor}`, `FuelScoop{scooped, total}`, `RefuelAll{amount}`, `RefuelPartial{amount}`, `ShipyardSwap{ship_type, ship_id}`, `Died`, `Resurrect`, `Shutdown`, `Market`, `Outfitting`, `Shipyard`, `ModuleInfo`, `Backpack`, `ShipLocker`, `FcMaterials`, y `Other`.
  - `pub struct LocationInfo { star_system, system_address: u64, star_pos: [f64;3], body: Option<String>, body_type: Option<String>, docked: bool, station_name: Option<String>, station_type: Option<String>, market_id: Option<u64>, jump_dist: Option<f64>, fuel_used: Option<f64>, fuel_level: Option<f64> }`.
  - `pub enum CompanionKind { Cargo, Market, Outfitting, Shipyard, ModulesInfo, NavRoute, Backpack, ShipLocker, FcMaterials }` con `fn file_name(self) -> &'static str` (`"Cargo.json"`, …, `"FCMaterials.json"`) y `fn for_event(name: &str) -> Option<CompanionKind>` (`Cargo`→Cargo, `Market`→Market, `Outfitting`, `Shipyard`, `ModuleInfo`→ModulesInfo, `NavRoute`, `Backpack`, `ShipLocker`, `FCMaterials`→FcMaterials).
  - `pub enum CompanionFile { Cargo(CargoFile{ vessel, count, inventory: Vec<CargoItem{name, name_localised, count, stolen, mission_id}> }), NavRoute(NavRouteFile{ route: Vec<RouteHop{star_system, system_address, star_pos, star_class}> }), Raw(CompanionKind, serde_json::Value) }` y `fn parse(kind: CompanionKind, text: &str) -> Result<CompanionFile, ParseError>` (solo Cargo y NavRoute tipados; el resto `Raw`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn parses_fsdjump_into_location_info() { /* línea FSDJump real → kind FsdJump, star_system, jump_dist Some */ }
#[test] fn unknown_event_is_other_not_error() { /* {"timestamp":…,"event":"FutureThing","X":1} → EventKind::Other, name "FutureThing" */ }
#[test] fn crlf_line_is_accepted() {}
#[test] fn missing_event_field_is_error() { /* → Err(MissingField("event")) */ }
#[test] fn raw_json_is_preserved_verbatim() { /* ev.raw.get() == línea sin \r */ }
#[test] fn companion_kind_for_event_maps_moduleinfo() { assert_eq!(CompanionKind::for_event("ModuleInfo"), Some(CompanionKind::ModulesInfo)); assert_eq!(CompanionKind::for_event("FSDJump"), None); }
#[test] fn parses_cargo_and_navroute_files() {}
#[test] fn every_line_of_basic_session_fixture_parses() { /* 0 errores */ }
```

`basic_session.log`: escrito a mano, ~25 líneas, nombre ficticio `CMDR TESTER`: Fileheader → Commander → LoadGame → Rank → Progress → Location (docked) → Loadout → Cargo → Undocked → StartJump → FSDJump ×2 → FSDTarget → SupercruiseExit → Docked → HullDamage → UnderAttack → Shutdown.

- [ ] **Step 2:** `cargo test -p tmc-model` → FAIL.
- [ ] **Step 3: Implementar.** `parse_line` deserializa primero a `Envelope { timestamp, event }` + `RawValue`, después hace `match` por nombre para deserializar la variante; si la variante tipada falla al deserializar, registra `tracing::debug!` y cae en `Other` (nunca error).
- [ ] **Step 4:** `cargo test -p tmc-model` → PASS.
- [ ] **Step 5: Commit** `feat(model): eventos del diario tipados y archivos compañeros`

---

### Task 4: `tmc-model` — GameState y SourceEvent

**Files:**
- Create: `crates/tmc-model/src/state.rs`, `crates/tmc-model/src/source.rs`, `crates/tmc-model/src/theme.rs`
- Modify: `crates/tmc-model/src/lib.rs`

**Interfaces:**
- Consumes: `JournalEvent`, `EventKind`, `Status`, `CompanionFile` (Tasks 2–3).
- Produces:
  - `pub enum SourceEvent { Journal { ev: JournalEvent, file: String, line: u64, replaying: bool }, Status(Status), Companion(CompanionFile), GameProcess { running: bool }, Theme(HudTheme), ImportProgress { done: u64, total: u64 } }`
  - `pub struct HudTheme { pub primary: Rgb, pub secondary: Rgb, pub background: Rgb, pub danger: Rgb, pub is_default: bool }`, `pub struct Rgb(pub u8, pub u8, pub u8)` con `fn to_css(&self) -> String` (`"#rrggbb"`).
  - `bitflags! pub struct Changes: u8 { CMDR, SHIP, LOCATION, STATUS, ROUTE, SESSION }`
  - `pub struct GameState { pub cmdr: Option<CommanderState>, pub ship: Option<ShipState>, pub location: LocationState, pub status: Option<Status>, pub route: RouteState, pub session: SessionState, pub galaxy: Galaxy, pub game_running: bool }` (`Default + Clone + Serialize`).
    - `CommanderState { name, fid, credits: Option<i64>, loan: Option<i64>, ranks: Ranks, progress: Ranks }` (`Ranks` = 8 campos `Option<u8>`).
    - `ShipState { ship_type, ship_type_localised, ship_id, name, ident, hull_health: Option<f64>, fuel_main: Option<f64>, fuel_reservoir: Option<f64>, fuel_capacity_main: Option<f64>, fuel_capacity_reserve: Option<f64>, cargo_count: Option<u32>, cargo_capacity: Option<u32>, rebuy: Option<i64>, max_jump_range: Option<f64>, damaged_modules: Vec<String> (health < 1.0), cargo: Vec<CargoItem> }`.
    - `LocationState { system, system_address, star_pos, body, body_type, station, station_type, market_id, mode: FlightMode }`; `enum FlightMode { Unknown, Docked, Landed, NormalSpace, Supercruise, Hyperspace, OnFoot, InSrv, InFighter, InTaxi, Multicrew }`.
    - `RouteState { hops: Vec<RouteHop>, next_system: Option<String>, remaining_jumps: Option<u32> }`.
    - `SessionState { started_at: Option<DateTime<Utc>>, jumps: u32, distance_ly: f64, credits_at_start: Option<i64>, credits_now: Option<i64> }` con `fn credits_delta(&self) -> Option<i64>`.
    - `enum Galaxy { Unknown, Live, Legacy }` desde `gameversion` (≥ 4.0 → Live).
  - `impl GameState { pub fn apply_event(&mut self, ev: &JournalEvent) -> Changes; pub fn apply_status(&mut self, s: &Status) -> Changes; pub fn apply_companion(&mut self, c: &CompanionFile) -> Changes }`

Reglas que los tests fijan: `LoadGame` reinicia `SessionState` (started_at = timestamp, credits_at_start = Credits); `FSDJump` suma `jumps += 1`, `distance_ly += JumpDist`, actualiza ubicación y `fuel_main`; `next_system` = primer salto de `route.hops` posterior al sistema actual, o `FSDTarget.Name` si no hay ruta; `NavRouteClear` vacía la ruta; `FlightMode` se deriva de Status (prioridad: OnFoot > InTaxi > Multicrew > InSrv > InFighter > Docked > Landed > FSD_JUMP→Hyperspace > Supercruise > NormalSpace); `Status.balance` actualiza `credits_now` y `cmdr.credits`; `Status.fuel` actualiza combustible; `LoadGame` → `game_running = true`, `Shutdown` → `game_running = false`, `GameProcess{running}` lo sobrescribe (lo aplica el actor). `Rgb` se serializa como cadena `"#rrggbb"` (Serialize propio), de modo que `core.theme` llega a la UI listo para CSS.

- [x] **Step 1: Tests:**

```rust
#[test] fn loadgame_starts_session_and_sets_cmdr() {}
#[test] fn fsdjump_counts_jump_distance_and_moves_location() {}
#[test] fn next_system_follows_navroute_after_current_system() {}
#[test] fn navroute_clear_empties_route() {}
#[test] fn flight_mode_derived_from_status_priority() { /* Flags DOCKED|IN_MAIN_SHIP → Docked; Flags2 ON_FOOT gana a todo */ }
#[test] fn status_balance_updates_credits_delta() {}
#[test] fn galaxy_live_vs_legacy_from_gameversion() { /* "4.4.1.1" → Live; "3.8.0.407" → Legacy */ }
#[test] fn loadout_marks_damaged_modules() {}
#[test] fn basic_session_fixture_end_state() { /* aplicar todo basic_session.log; assert sistema final, jumps == 2, mode Docked */ }
#[test] fn apply_returns_only_changed_areas() { /* UnderAttack → Changes::empty() */ }
```

- [ ] **Step 2:** `cargo test -p tmc-model` → FAIL. Ciclo inicial no observado en esta sesión; sí se observó RED de las cuatro regresiones de revisión (véase verificación incremental).
- [x] **Step 3: Implementar** los reductores en `state.rs` (sin E/S, sin tiempo real; puro).
- [x] **Step 4:** `cargo test -p tmc-model` → PASS.
- [x] **Step 5: Commit** `feat(model): GameState y reductores de eventos`

---

### Task 5: `tmc-journal` — localizar carpeta y listar diarios

**Files:**
- Create: `crates/tmc-journal/{Cargo.toml, src/lib.rs, src/locate.rs, src/files.rs}`

**Interfaces:**
- Produces:
  - `pub fn default_journal_dir() -> Option<PathBuf>` — Windows: `SHGetKnownFolderPath(FOLDERID_SavedGames)` (crate `windows` o `known-folders`) + `Frontier Developments\Elite Dangerous`; Linux: primer candidato existente entre `~/.steam/steam/steamapps/compatdata/359320/pfx/drive_c/users/steamuser/Saved Games/Frontier Developments/Elite Dangerous`, `~/.local/share/Steam/steamapps/compatdata/359320/…` y `~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/359320/…`.
  - `pub fn resolve_journal_dir(configured: Option<&Path>) -> Result<PathBuf, LocateError>`; `pub enum LocateError { NotFound, NotADirectory(PathBuf) }`.
  - `pub fn is_journal_name(name: &str) -> bool` (`Journal.` + `YYYY-MM-DDTHHMMSS.NN` + `.log`; acepta también el formato antiguo `Journal.YYMMDDHHMMSS.NN.log`).
  - `pub fn list_journals(dir: &Path) -> io::Result<Vec<PathBuf>>` ordenados por nombre (el formato antiguo va antes que el nuevo).
  - `pub fn graphics_override_path() -> Option<PathBuf>` (Windows `%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options\Graphics\GraphicsConfigurationOverride.xml`; Linux dentro del prefijo Proton: `…/pfx/drive_c/users/steamuser/AppData/Local/Frontier Developments/Elite Dangerous/Options/Graphics/…`).

- [x] **Step 1: Tests** (con `tempfile`):

```rust
#[test] fn recognises_new_and_old_journal_names() {}
#[test] fn ignores_non_journal_files() { /* Status.json, Journal.txt, JournalBeta… */ }
#[test] fn lists_journals_sorted_old_format_first() {}
#[test] fn configured_dir_wins_and_must_exist() {}
#[test] fn reads_journal_from_path_with_spaces_and_accents() { /* tempdir/"Partidas guardadas ñ é"/… → list_journals encuentra 2 archivos */ }
```

- [x] **Step 2:** `cargo test -p tmc-journal` → FAIL. **Step 3:** implementar. **Step 4:** PASS.
- [x] **Step 5: Commit** `feat(journal): localización de carpeta y listado de diarios`

---

### Task 6: `tmc-journal` — lector incremental de líneas

**Files:**
- Create: `crates/tmc-journal/src/tail.rs`

**Interfaces:**
- Produces: `pub struct LineTailer`; `pub fn LineTailer::open(path: &Path, offset: u64) -> io::Result<LineTailer>`; `pub fn read_new_lines(&mut self) -> io::Result<Vec<(u64 /*nº de línea, 1-based*/, String)>>`; `pub fn offset(&self) -> u64`; `pub fn line_no(&self) -> u64`; `pub fn path(&self) -> &Path`. Al abrir con `offset > 0`, `line_no` se reconstruye contando `\n` hasta el offset.

Comportamiento: abre con permisos de compartición lectura+escritura+borrado (Windows: `OpenOptionsExt::share_mode(FILE_SHARE_READ|FILE_SHARE_WRITE|FILE_SHARE_DELETE)`); lee desde `offset` hasta EOF; solo devuelve líneas terminadas en `\n` (quita `\r`); guarda el resto en buffer interno; ignora bytes NUL iniciales; si el archivo encoge (truncado), reinicia a 0.

- [x] **Step 1: Tests:**

```rust
#[test] fn reads_complete_lines_and_advances_offset() {}
#[test] fn tailer_holds_partial_line_until_newline() { /* escribe `{"a":1` → []; añade `}\n` → [(1, "{\"a\":1}")] */ }
#[test] fn strips_crlf() {}
#[test] fn resumes_from_offset_with_correct_line_numbers() {}
#[test] fn handles_40kb_line() {}
#[test] fn reads_while_writer_keeps_file_open() { /* writer abierto con OpenOptions append sin cerrar; tailer lee lo añadido */ }
#[test] fn skips_leading_nul_bytes() {}
```

- [x] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-journal` PASS.
- [x] **Step 5: Commit** `feat(journal): lector incremental de líneas con offset`

---

### Task 7: `tmc-journal` — sondeo de Status, compañeros, proceso, HUD y watcher

**Files:**
- Create: `crates/tmc-journal/src/{status_poller.rs, companion_reader.rs, process.rs, hud.rs, watcher.rs}`

**Interfaces:**
- Consumes: `LineTailer`, `list_journals`, `parse_line`, `Status::parse`, `CompanionKind`, `CompanionFile`, `SourceEvent`, `HudTheme`.
- Produces:
  - `pub struct StatusPoller` — `new(path: PathBuf)`, `poll(&mut self) -> Option<Status>`: compara (tamaño, mtime); si cambian, lee con reintento (10 × 50 ms) y devuelve `Some` solo si el texto es no vacío, parsea, y difiere del último texto aceptado.
  - `pub fn read_companion(dir: &Path, kind: CompanionKind) -> Option<CompanionFile>` (reintento 10 × 50 ms; `None` si nunca parsea).
  - `pub fn is_game_running() -> bool` (crate `sysinfo`, procesos `EliteDangerous64.exe`/`EliteDangerous64`, refresco solo de procesos).
  - `pub fn read_hud_theme(path: Option<&Path>) -> HudTheme`: lee `<MatrixRed>`, `<MatrixGreen>`, `<MatrixBlue>` de `GraphicsConfigurationOverride.xml` (vectores de 3 floats separados por comas). Color de salida = `r·MatrixRed + g·MatrixGreen + b·MatrixBlue` aplicado a la paleta base `primary #FF7100`, `secondary #FFB43C`, `danger #FF3B30` (sin transformar), `background #0B0B0E`, recortado a 0–255. Sin archivo, vacío (`<GraphicsConfig />`) o inválido → paleta base con `is_default = true`.
  - `pub struct WatchOptions { pub journal_dir: PathBuf, pub poll_active: Duration /*250ms*/, pub poll_idle: Duration /*1000ms*/, pub new_file_check: Duration /*1s*/, pub process_check: Duration /*10s*/ }` con `Default` salvo `journal_dir`.
  - `pub fn spawn_watcher(opts: WatchOptions, tx: tokio::sync::mpsc::Sender<SourceEvent>) -> tokio::task::JoinHandle<()>`.

Algoritmo del watcher (bucle en una tarea tokio; E/S con `spawn_blocking` o síncrona corta):
1. Al arrancar: `list_journals`; abre el más reciente con offset 0; envía todas sus líneas con `replaying: true` hasta EOF; después envía `Theme(read_hud_theme(...))`, el `Status` actual si existe, y pasa a `replaying: false`.
2. Cada tick (`poll_active` si `game_running`, si no `poll_idle`): `read_new_lines` → `parse_line` → `SourceEvent::Journal`; las líneas inválidas se registran con `tracing::warn!(file, line)` y se descartan. Tras cada evento cuyo nombre tenga `CompanionKind::for_event`, envía `SourceEvent::Companion(read_companion(...))` si es `Some`.
3. Cada `new_file_check` (y al recibir evento `Create` de `notify` en la carpeta): si hay un diario más nuevo que el actual, **primero vacía el actual** (`read_new_lines` final) y luego cambia al nuevo con offset 0 (`replaying: false`).
4. `StatusPoller::poll` cada 250 ms → `SourceEvent::Status`.
5. Cada `process_check`: `is_game_running()`; si cambia, envía `GameProcess { running }`.
6. Si el canal se cierra, termina.

- [x] **Step 1: Tests** (tokio test, tempdir; ticks acortados a 20 ms en tests):

```rust
#[test] fn status_poller_ignores_empty_and_truncated_reads() { /* válido → Some; vacío → None; truncado → None; mismo texto → None; nuevo válido → Some */ }
#[test] fn read_companion_returns_none_after_retries_on_garbage() {}
#[test] fn hud_theme_default_when_empty_override() { /* "<GraphicsConfig />" → is_default */ }
#[test] fn hud_theme_applies_matrix() { /* MatrixRed 0,0,1 ; MatrixGreen 0,1,0 ; MatrixBlue 1,0,0 → primary #0071FF */ }
#[tokio::test] async fn watcher_replays_current_journal_then_goes_live() { /* eventos iniciales replaying=true, el añadido después replaying=false */ }
#[tokio::test] async fn watcher_switches_to_new_journal_without_losing_lines() { /* escribe 3 líneas en A, crea B con 2 líneas y añade 1 más a A antes → se reciben las 6, en orden A antes que B, sin duplicados */ }
#[tokio::test] async fn watcher_emits_companion_after_trigger_event() { /* línea "Cargo" + Cargo.json → Companion(Cargo) */ }
#[tokio::test] async fn invalid_line_is_skipped_not_fatal() {}
```

- [x] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-journal` PASS.
- [x] **Step 5: Commit** `feat(journal): watcher del diario, Status, compañeros, proceso y tema HUD`

---

### Task 8: `tmc-store` — SQLite, escritor por lotes e importación del historial

**Files:**
- Create: `crates/tmc-store/{Cargo.toml, src/lib.rs, src/migrations.rs, src/writer.rs, src/import.rs, src/query.rs}`

**Interfaces:**
- Consumes: `parse_line`, `list_journals`, `LineTailer`.
- Produces:
  - `pub struct Store` (clonable; internamente ruta + conexión para lecturas); `Store::open(path: &Path) -> Result<Store, StoreError>`, `Store::open_in_memory() -> Result<Store, StoreError>`; aplica el esquema F0 de la especificación §6 tal cual + `PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;`.
  - `pub struct Migration { pub id: &'static str, pub sql: &'static str }`; `Store::apply_migrations(&self, migrations: &[Migration]) -> Result<(), StoreError>` (tabla `schema_version(id TEXT PRIMARY KEY, applied_at TEXT)`; idempotente).
  - `pub struct StoredEvent { pub file_name: String, pub line: u64, pub ts: DateTime<Utc>, pub name: String, pub cmdr: Option<String>, pub json: String }`
  - `pub struct EventWriter` — `EventWriter::spawn(store: &Store) -> EventWriter`, `fn send(&self, ev: StoredEvent)` (no bloquea; canal `std::sync::mpsc`), hilo propio con commit cada 500 ms o 500 filas, `INSERT OR IGNORE` (idempotente por `UNIQUE(file_id,line)`), no modifica `journal_file.offset` (R1: propiedad exclusiva de `import_history`); `fn flush_blocking(&self) -> Result<(), StoreError>` para tests y cierre (desviación documentada abajo).
  - `pub struct ImportOptions { pub throttle: Arc<AtomicBool> /*true = juego activo → limitar*/ }`; `pub struct ImportReport { pub files: u64, pub events: u64, pub skipped_lines: u64 }`.
  - `pub fn import_history(store: &Store, dir: &Path, opts: &ImportOptions, progress: &mut dyn FnMut(u64, u64)) -> Result<ImportReport, StoreError>`: recorre `list_journals`, salta los `complete = 1`, reanuda desde `journal_file.offset`, transacciones de 5 000 filas, `cmdr` = último `Commander.Name`/`LoadGame.Commander` visto en ese archivo, marca `complete = 1` en archivos que no son el más reciente. Con `throttle = true`, duerme 50 ms cada 5 000 filas.
  - `pub struct EventQuery { pub name: Option<String>, pub from: Option<DateTime<Utc>>, pub to: Option<DateTime<Utc>>, pub cmdr: Option<String>, pub limit: u32 /*≤1000*/, pub before_id: Option<i64> }`; `Store::query_events(&self, q: &EventQuery) -> Result<Vec<EventRow>, StoreError>`; `EventRow { id: i64, ts, name, cmdr, json }` ordenado por `id DESC`.
  - `Store::kv_get(&self, key) -> Option<String>`, `Store::kv_set(&self, key, value)`.

- [x] **Step 1: Tests:**

```rust
#[test] fn schema_is_created_and_wal_enabled() {}
#[test] fn migrations_are_idempotent() {}
#[test] fn writer_batches_and_is_idempotent() { /* mismo (file,line) dos veces → 1 fila */ }
#[test] fn import_is_resumable() { /* importar, añadir líneas al último archivo, reimportar → solo las nuevas; total correcto */ }
#[test] fn import_tags_cmdr_per_file() { /* dos archivos con comandantes distintos */ }
#[test] fn import_reports_progress_monotonic() {}
#[test] fn import_skips_invalid_lines_and_counts_them() {}
#[test] fn query_events_filters_and_paginates() {}
```

- [x] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-store` PASS.
- [x] **Step 5: Commit** `feat(store): SQLite, escritor por lotes e importación reanudable`

---

### Task 9: `tmc-core` — actor de estado, módulos, topics y alertas

**Files:**
- Create: `crates/tmc-core/{Cargo.toml, src/lib.rs, src/module.rs, src/actor.rs, src/topics.rs, src/alert.rs}`

**Interfaces:**
- Consumes: `GameState`, `Changes`, `SourceEvent`, `HudTheme`, `EventWriter`, `StoredEvent`.
- Produces:
  - `pub type TopicSet = BTreeSet<String>`
  - `pub trait Module: Send { fn id(&self) -> &'static str; fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet; fn topic_value(&self, topic: &str) -> Option<serde_json::Value>; fn topics(&self) -> Vec<String>; fn relevance(&self, _state: &GameState) -> Relevance { Relevance::Normal } }` — (`migrations()` del spec se añade cuando un módulo lo necesite; F0 no).
  - `pub enum Relevance { Hidden, Normal, Active, Urgent }` (Serialize, lowercase).
  - `pub struct ModuleCtx { pub replaying: bool, alerts: Vec<Alert> }` con `fn alert(&mut self, a: Alert)` (ignorado si `replaying`).
  - `pub struct Alert { pub id: String /*uuid v4*/, pub severity: Severity, pub key: String /*clave Fluent*/, pub args: BTreeMap<String, serde_json::Value> }`; `pub enum Severity { Info, Warn, Critical }` (Serialize lowercase); `Alert::new(severity, key) -> Alert` y `.arg(name, value)`.
  - Topics del núcleo (publicados por el actor desde `GameState` según `Changes`): `core.cmdr`, `core.ship`, `core.location`, `core.status`, `core.route`, `core.session`, `core.game` (`{ running, galaxy }`), `core.theme`, `core.modules` (`[{id, relevance}]`).
  - `pub enum CoreMessage { Update { seq: u64, topics: BTreeMap<String, serde_json::Value> }, Alert(Alert), ImportProgress { done: u64, total: u64 } }` (Serialize con `#[serde(tag = "type", rename_all = "kebab-case")]` → `update` / `alert` / `import-progress`).
  - `pub struct CoreHandle { pub events: mpsc::Sender<SourceEvent>, updates: broadcast::Sender<CoreMessage> }` con `fn subscribe(&self) -> broadcast::Receiver<CoreMessage>` y `async fn snapshot(&self) -> BTreeMap<String, serde_json::Value>` (pide al actor por `oneshot` el valor actual de todos los topics).
  - `pub fn spawn_core(modules: Vec<Box<dyn Module>>, writer: Option<EventWriter>, flush_every: Duration /*100ms*/) -> CoreHandle`

Reglas: el actor es el único dueño de `GameState` y módulos; por cada `SourceEvent::Journal` envía `StoredEvent` al `writer` (cmdr = `GameState.cmdr.name` vigente); llama a cada módulo dentro de `std::panic::catch_unwind(AssertUnwindSafe(..))` — si un módulo hace panic se elimina de la lista y se emite `Alert::new(Critical, "alert-module-crashed").arg("module", id)`; acumula topics sucios y, cada `flush_every`, emite un único `CoreMessage::Update` con `seq` creciente (nada si no hay sucios); alertas e `ImportProgress` se emiten inmediatamente.

- [x] **Step 1: Tests** (tokio, módulos falsos en el propio test):

```rust
#[tokio::test] async fn journal_event_updates_core_topics() { /* FSDJump → Update contiene core.location y core.session */ }
#[tokio::test] async fn updates_are_coalesced_to_flush_interval() { /* 50 eventos Status en 10 ms → 1 Update */ }
#[tokio::test] async fn snapshot_returns_all_topics() {}
#[tokio::test] async fn panicking_module_is_disabled_and_reported() { /* módulo que hace panic → Alert key "alert-module-crashed"; el siguiente evento sigue actualizando core.* */ }
#[tokio::test] async fn alerts_suppressed_while_replaying() {}
#[tokio::test] async fn journal_events_are_sent_to_writer() { /* Store in-memory + EventWriter → fila guardada con cmdr */ }
#[tokio::test] async fn no_update_when_nothing_changed() {}
```

- [x] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-core` PASS.
- [x] **Step 5: Commit** `feat(core): actor de estado, contrato de módulos, topics y alertas`

---

### Task 10: `tmc-i18n` y traducciones

**Files:**
- Create: `crates/tmc-i18n/{Cargo.toml, src/lib.rs}`, `locales/{es,en,fr,de,pt}/main.ftl`

**Interfaces:**
- Produces: `pub const LANGUAGES: [&str; 5] = ["es","en","fr","de","pt"]`; `pub struct I18n`; `I18n::new(lang: &str) -> I18n` (idioma desconocido → `es`; fallback de claves a `es`); `fn t(&self, key: &str, args: &[(&str, FluentValue)]) -> String` (clave inexistente → la propia clave); `pub fn keys(lang: &str) -> BTreeSet<String>`. Archivos embebidos con `rust-embed` (`folder = "../../locales"`).
- Claves mínimas de F0 (todas en los 5 idiomas, traducidas de verdad, no copias en español): `app-name`, `nav-dashboard`, `nav-settings`, `panel-cmdr`, `panel-ship`, `panel-location`, `panel-status`, `panel-route`, `panel-session`, `panel-alerts`, `field-credits`, `field-hull`, `field-fuel`, `field-cargo`, `field-rebuy`, `field-jump-range`, `field-system`, `field-body`, `field-station`, `field-next-system`, `field-remaining-jumps`, `field-session-time`, `field-jumps`, `field-distance`, `field-credits-earned`, `mode-docked`, `mode-landed`, `mode-normal-space`, `mode-supercruise`, `mode-hyperspace`, `mode-on-foot`, `mode-in-srv`, `mode-in-fighter`, `mode-in-taxi`, `mode-multicrew`, `mode-unknown`, `flag-shields`, `flag-hardpoints`, `flag-landing-gear`, `flag-mass-locked`, `flag-silent-running`, `flag-cargo-scoop`, `flag-sco`, `alert-low-fuel`, `alert-overheating`, `alert-interdiction`, `alert-under-attack`, `alert-hull-damage` (arg `$pct`), `alert-low-oxygen`, `alert-low-health`, `alert-module-crashed` (arg `$module`), `action-copy-next-system`, `toast-copied` (arg `$system`), `import-progress` (args `$done`, `$total`), `game-not-running`, `galaxy-legacy-warning`, `settings-language`, `settings-lan`, `settings-lan-help`, `settings-pairing-qr`, `settings-auto-copy`, `settings-theme`, `theme-hud`, `theme-classic`, `tray-open`, `tray-quit`, `empty-no-data`.

- [x] **Step 1: Tests:**

```rust
#[test] fn all_languages_have_same_keys_as_es() { for l in LANGUAGES { assert_eq!(keys(l), keys("es"), "{l}"); } }
#[test] fn formats_args() { /* es: alert-hull-damage con pct=25 contiene "25" */ }
#[test] fn unknown_language_falls_back_to_es() {}
#[test] fn missing_key_returns_key() {}
```

- [x] **Step 2:** FAIL. **Step 3:** implementar + escribir los `.ftl`. **Step 4:** `cargo test -p tmc-i18n` PASS.
- [x] **Step 5: Commit** `feat(i18n): Fluent compartido y traducciones es/en/fr/de/pt`

---

### Task 11: `tmc-modules` — registro y módulo `dashboard`

**Files:**
- Create: `crates/tmc-modules/{Cargo.toml, src/lib.rs, src/registry.rs, src/dashboard/mod.rs, src/dashboard/rules.rs}`

**Interfaces:**
- Consumes: `Module`, `ModuleCtx`, `Alert`, `Severity`, `GameState`, `SourceEvent`, `Flags`, `Flags2`.
- Produces: `pub fn all_modules() -> Vec<Box<dyn Module>>` (F0: `[Dashboard::new()]`); `pub struct Dashboard`; topic `dashboard.alerts` = lista de alertas activas `[{id, severity, key, args, since}]` (máx. 20, se retiran cuando la condición desaparece; las basadas en eventos caducan al procesar el siguiente Journal/Status con timestamp >= since + 60 s; véase desviación temporal).

Reglas de alerta (`rules.rs`), con antirrebote: la misma `key` no se repite en **30 s**:

| Condición | Severidad | Clave |
|---|---|---|
| `Flags::LOW_FUEL` pasa a activo | Warn | `alert-low-fuel` |
| `Flags::OVERHEATING` pasa a activo | Warn | `alert-overheating` |
| `Flags::BEING_INTERDICTED` pasa a activo | Critical | `alert-interdiction` |
| Evento `UnderAttack` | Warn | `alert-under-attack` |
| Evento `HullDamage` con `player_pilot != false`: `health ≤ 0.25` → Critical, si no Warn | — | `alert-hull-damage` (`pct` = round(health×100)) |
| `Flags2::LOW_OXYGEN` pasa a activo | Critical | `alert-low-oxygen` |
| `Flags2::LOW_HEALTH` pasa a activo | Critical | `alert-low-health` |

Relevance: `Urgent` si hay alguna alerta Critical activa, si no `Normal`.

- [x] **Step 1: Tests:**

```rust
#[test] fn low_fuel_alert_on_rising_edge_only() {}
#[test] fn same_alert_debounced_for_30s() { /* usa timestamps de los eventos, no reloj real */ }
#[test] fn hull_damage_severity_by_threshold() {}
#[test] fn replayed_events_do_not_raise_alerts() { /* SourceEvent::Journal{replaying:true, UnderAttack} → sin alertas, dashboard.alerts vacío */ }
#[test] fn alert_clears_when_flag_drops() {}
#[test] fn registry_contains_dashboard() {}
```

- [x] **Step 2:** FAIL. **Step 3:** implementar (el antirrebote usa `timestamp` del evento/Status para ser determinista). **Step 4:** `cargo test -p tmc-modules` PASS.
- [x] **Step 5: Commit** `feat(modules): registro de módulos y panel general con alertas`

---

### Task 12: `tmc-server` — HTTP, WebSocket, seguridad y emparejamiento

**Files:**
- Create: `crates/tmc-server/{Cargo.toml, src/lib.rs, src/ws.rs, src/rest.rs, src/security.rs, src/assets.rs, src/pairing.rs}`

**Interfaces:**
- Consumes: `CoreHandle`, `CoreMessage`, `Store`, `EventQuery`.
- Produces:
  - `pub trait ActionHost: Send + Sync + 'static { fn run_action(&self, name: &str, snapshot: &BTreeMap<String, serde_json::Value>) -> Result<serde_json::Value, ActionError>; }` (`ActionError { NotFound, Failed(String) }`).
  - `pub struct ServerConfig { pub port: u16 /*47320*/, pub lan: bool, pub token: String, pub ui_dir: Option<PathBuf> /*dev: servir carpeta; None: assets embebidos*/ }`
  - `pub struct ServerHandle { pub local_url: String /*http://127.0.0.1:{port}/*/, pub lan_url: Option<String /*http://{ip}:{port}/?token=…*/>, pub port: u16, shutdown: oneshot::Sender<()> }` con `fn shutdown(self)`.
  - `pub async fn serve(cfg: ServerConfig, core: CoreHandle, store: Store, host: Arc<dyn ActionHost>) -> Result<ServerHandle, ServerError>` — si el puerto está ocupado prueba los 20 siguientes.
  - Rutas: `GET /` y estáticos (UI; `rust-embed` sobre `ui/dist` con feature `embed-ui`), `GET /ws`, `GET /api/health` → `{ok:true, version}`, `GET /api/history/events` (query = `EventQuery`, `limit` por defecto 100), `POST /api/actions/{name}` → `ActionHost::run_action`, `GET /api/pairing` (solo loopback) → `{ url, qr_svg }` (crate `qrcode`, SVG).
  - Protocolo WS exactamente como spec §7.2: al conectar `hello` (`version: 1`, `locale` de config, lista de topics del snapshot), luego `snapshot`; después reenvía `CoreMessage` filtrados por las suscripciones (`subscribe` con patrones `prefijo.*` o nombre exacto; sin `subscribe` → todos). Cada `subscribe` recibido se responde con un `snapshot` nuevo de los topics que casan (así la UI puede bajar a `["core.game"]` al ocultarse y recuperar todo al volver). Si el cliente va lento (`broadcast` con `Lagged`), se le reenvía un `snapshot` completo.
  - Seguridad (`security.rs`, middleware): peticiones desde loopback → permitidas sin token pero `Host` debe ser `127.0.0.1:{port}` o `localhost:{port}`; peticiones desde otra IP → solo si `lan = true` **y** token válido (query `?token=` la primera vez → cookie `tmc_token` HttpOnly SameSite=Strict; después cookie) **y** `Host` = IP local:puerto; en `/ws` además `Origin` debe coincidir con el `Host`. Comparación de token en tiempo constante.

- [x] **Step 1: Tests** (`tower::ServiceExt::oneshot` para HTTP; `tokio-tungstenite` para WS):

```rust
#[tokio::test] async fn health_ok_from_loopback() {}
#[tokio::test] async fn rejects_foreign_host_header() { /* Host: evil.com → 403 (anti DNS-rebinding) */ }
#[tokio::test] async fn lan_request_without_token_is_rejected() {}
#[tokio::test] async fn lan_request_with_token_sets_cookie() {}
#[tokio::test] async fn ws_rejects_mismatched_origin() {}
#[tokio::test] async fn ws_sends_hello_then_snapshot_then_updates() {}
#[tokio::test] async fn ws_subscription_filters_topics() { /* subscribe ["core.status"] → snapshot solo con core.status; no recibe core.location */ }
#[tokio::test] async fn action_endpoint_calls_host() {}
#[tokio::test] async fn history_endpoint_paginates() {}
#[tokio::test] async fn falls_back_to_next_free_port() {}
```

- [x] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-server` PASS.
- [x] **Step 5: Commit** `feat(server): HTTP/WebSocket, seguridad LAN y emparejamiento por QR`

---

### Task 13: `tmc-app` + `apps/headless` — composición, configuración y portapapeles

**Files:**
- Create: `crates/tmc-app/{Cargo.toml, src/lib.rs, src/config.rs, src/logging.rs, src/wiring.rs, src/clipboard.rs}`, `apps/headless/{Cargo.toml, src/main.rs}`
- Modify: `Cargo.toml` (añadir `apps/headless` a `members`)

**Interfaces:**
- Consumes: todo lo anterior.
- Produces:
  - `pub struct Config { pub journal_dir: Option<PathBuf>, pub language: String /*"es"*/, pub server: ServerSection { port: u16 /*47320*/, lan: bool /*false*/, lan_token: Option<String> }, pub auto_copy_next_system: bool /*true*/, pub alerts_enabled: bool /*true*/ }` (`Serialize/Deserialize/Default`).
  - `pub fn config_path() -> PathBuf` (`directories::ProjectDirs::from("", "", "TMC")`: Windows `%APPDATA%\TMC\config\config.toml`; Linux `$XDG_CONFIG_HOME/tmc/config.toml`), `pub fn data_dir() -> PathBuf`; `Config::load_or_default() -> Config` (archivo inexistente → crea con valores por defecto; TOML inválido → se renombra a `config.toml.bak` y se usa el defecto); `Config::save(&self)`; `Config::ensure_lan_token(&mut self) -> String` (128 bits aleatorios, hex).
  - `pub fn init_logging(data_dir: &Path) -> WorkerGuard` (`tracing-appender` rotación diaria, conserva 7 archivos, nivel `info` salvo `RUST_LOG`).
  - `pub struct Clipboard` implementa la parte de portapapeles (`arboard`).
  - `pub struct AppActions` implementa `ActionHost`: acción `clipboard.copy-next-system` → copia `snapshot["core.route"]["next_system"]` y devuelve `{system}`; sin siguiente sistema → `ActionError::Failed("no-next-system")`.
  - `pub struct Running { pub server: ServerHandle, pub core: CoreHandle, pub store: Store }`; `pub async fn start(config: Config) -> anyhow::Result<Running>`: resuelve carpeta del diario, abre `Store` en `data_dir()/tmc.db`, crea `EventWriter`, `spawn_core(all_modules(), Some(writer), 100ms)`, `spawn_watcher`, lanza `import_history` en un hilo (`throttle` = juego activo; progreso → `SourceEvent::ImportProgress`), arranca `serve`; si `auto_copy_next_system`, tarea que escucha `Update` con `core.route` y copia `next_system` cuando cambia (no durante replay inicial).
  - `apps/headless/src/main.rs`: `tmc-headless [--config <ruta>] [--journal-dir <ruta>] [--ui-dir <ruta>]` → `start`, imprime `local_url` (y `lan_url`), espera Ctrl+C.

- [x] **Step 1: Tests** (`tmc-app`):

```rust
#[test] fn config_roundtrip_and_defaults() { /* port 47320, language "es", auto_copy true */ }
#[test] fn invalid_config_is_backed_up_and_defaulted() {}
#[test] fn lan_token_is_128_bit_hex_and_stable() {}
#[test] fn copy_next_system_action_without_route_fails() {}
#[test] fn corrupt_database_is_renamed_and_rebuilt() { /* tmc.db con basura → start renombra a tmc.db.corrupt-<timestamp>, crea una nueva y reimporta (spec §10) */ }
#[tokio::test] async fn start_end_to_end_with_temp_journal_dir() { /* tempdir con basic_session.log + Status.json → GET /api/health ok; WS snapshot contiene core.location.system esperado */ }
```

- [x] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-app` PASS.
- [x] **Step 5: Verificación manual con tus diarios reales**

Run: `cargo run -p tmc-headless` y abrir `http://127.0.0.1:47320/api/health`
Expected: `{"ok":true,...}`; en el log aparece la importación del historial terminada.

- [x] **Step 6: Commit** `feat(app): composición, configuración, portapapeles y binario headless`

---

### Task 14: `tmc-replay` — reproducción y anonimización de diarios

**Files:**
- Create: `crates/tmc-replay/{Cargo.toml, src/main.rs, src/replay.rs, src/anonymize.rs}`, `tests/fixtures/journal/real_*.log` (generados)

**Interfaces:**
- Produces (binario `tmc-replay`):
  - `tmc-replay play --from <archivo.log|carpeta> --to <carpeta> [--speed 10] [--status <carpeta con status_*.json>]`: escribe las líneas en `<to>/Journal.<fecha>.01.log` respetando los intervalos entre `timestamp` divididos por `speed` (máx. 2 s de espera real por línea); copia los archivos compañeros de la carpeta origen cuando aparece su evento disparador; si se pasa `--status`, reescribe `<to>/Status.json` con el snapshot más reciente cuyo timestamp ≤ al de la línea actual.
  - `tmc-replay anonymize --from <archivo.log> --to <archivo.log>`: sustituye nombre de comandante (`Commander`, `Name` en `Commander`/`LoadGame`), `FID`, `ShipName`, `ShipIdent`, nombres de amigos/alas/escuadrón y textos de chat (`ReceiveText`, `SendText`) por valores ficticios estables (`CMDR TESTER`, `F0000001`, …); elimina eventos `Friends`.
  - `pub fn anonymize_line(line: &str, map: &mut Pseudonyms) -> Option<String>` y `pub async fn play(opts: PlayOptions) -> anyhow::Result<()>` (públicas para tests).

- [x] **Step 1: Tests:**

```rust
#[test] fn anonymize_replaces_identity_fields_consistently() {}
#[test] fn anonymize_drops_friends_and_chat_text() {}
#[tokio::test] async fn play_writes_all_lines_in_order_at_speed() { /* speed 1000 → todas las líneas, orden intacto */ }
```

- [x] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-replay` PASS.
- [x] **Step 5: Generar fixtures reales anonimizados** a partir de los diarios del propietario (`%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous\Journal.*.log`) → `tests/fixtures/journal/real_01.log`…; revisar a mano con `grep -i` que no queda el nombre real del comandante.
- [x] **Step 6: Test de snapshot** en `tmc-model` (`insta`): `real_fixtures_end_state` aplica cada `real_*.log` y guarda el `GameState` final con `insta::assert_json_snapshot!`. Run: `cargo insta test -p tmc-model --accept` la primera vez; revisar el `.snap`.
- [x] **Step 7: Commit** `feat(replay): reproductor y anonimizador de diarios + fixtures reales`

---

### Task 15: UI — base (cliente WS, i18n, tema HUD, shell y rejilla)

**Files:**
- Create: `ui/package.json`, `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/svelte.config.js`, `ui/index.html`, `ui/src/main.ts`, `ui/src/App.svelte`, `ui/src/lib/ws.svelte.ts`, `ui/src/lib/i18n.ts`, `ui/src/lib/theme.ts`, `ui/src/lib/format.ts`, `ui/src/shell/{Sidebar.svelte, TopBar.svelte, Grid.svelte, Panel.svelte, Toasts.svelte, ImportProgress.svelte}`, `ui/src/styles/{tokens.css, base.css}`, `ui/tests/{ws.test.ts, i18n.test.ts, theme.test.ts}`

**Interfaces:**
- Consumes: protocolo WS (Task 12), topics `core.*`, `core.theme`, `core.modules`, mensajes `alert`, `import-progress`; `locales/*/main.ftl`.
- Produces:
  - `ws.svelte.ts`: `export const live: { connected: boolean; topics: Record<string, unknown>; seq: number }` (estado `$state`), `export function connect(url?: string): void` (por defecto `ws(s)://${location.host}/ws`; reconexión con espera 1 s → 2 s → 5 s → máx. 10 s), `export function topic<T>(name: string): T | undefined`, `export function onAlert(cb: (a: Alert) => void): () => void`, `export async function action(name: string): Promise<unknown>` (POST `/api/actions/{name}`). Con `document.hidden` envía `subscribe ["core.game"]`; al volver a visible, `subscribe ["*"]` (spec §8.3: sin actualizaciones de alta frecuencia en segundo plano).
  - `i18n.ts`: `export function setLocale(lang: string): void`, `export function t(key: string, args?: Record<string, string | number>): string` (`@fluent/bundle`; carga con `import.meta.glob('../../locales/*/main.ftl', { query: '?raw', eager: true })`; fallback a `es`; clave inexistente → la clave). Idioma inicial: `hello.locale`.
  - `theme.ts`: `export function applyTheme(t: { primary: string; secondary: string; background: string; danger: string }): void` → variables CSS `--hud-primary`, `--hud-secondary`, `--hud-bg`, `--hud-danger` + derivadas `--hud-primary-dim` (40 % alfa) y `--hud-border`.
  - `format.ts`: `credits(n)`, `ly(n)`, `pct(x)`, `duration(ms)` con `Intl.NumberFormat` del locale activo.
  - Shell: `Sidebar` (iconos de módulos de `core.modules`, punto de color según `relevance`), `TopBar` fija (sistema, modo de vuelo, combustible, casco, carga, créditos, indicador de conexión y de `core.game.running`; aviso `galaxy-legacy-warning` si `galaxy == "Legacy"`), `Grid` CSS grid: **4 columnas ≥ 2400 px**, 3 columnas ≥ 1600 px, 2 columnas ≥ 900 px, 1 columna por debajo; `Panel` (título traducido + contenido; borde y título con `--hud-*`); `Toasts` (alertas 8 s, Critical hasta cerrar); `ImportProgress` (barra mientras `done < total`).
  - Reglas de estilo: tipografía del sistema (`system-ui`) sin fuentes externas; contraste AA sobre `--hud-bg`; **sin** `@keyframes` infinitos; `transition` ≤ 200 ms.

- [ ] **Step 1: Scaffold** con Vite + Svelte 5 + TS (`npm create vite@latest ui -- --template svelte-ts`), añadir `@fluent/bundle`, `vitest`, `jsdom`, `@sveltejs/vite-plugin-svelte`, `svelte-check`. Scripts: `dev`, `build`, `check` (`svelte-check`), `test` (`vitest run`). `vite.config.ts`: proxy `/ws` y `/api` → `http://127.0.0.1:47320` en dev; `server.fs.allow` incluye `../locales`.
- [ ] **Step 2: Tests:**

```ts
test('ws applies snapshot then update', ...)        // mock WebSocket: hello → snapshot → update; topic('core.status') refleja el último valor
test('ws reconnects with backoff', ...)             // fake timers: 1000, 2000, 5000, 10000, 10000
test('ws narrows subscription when page hidden', ...) // visibilitychange hidden → envía subscribe ["core.game"]; visible → ["*"]
test('t falls back to es and formats args', ...)    // setLocale('xx'); t('alert-hull-damage', {pct: 25}) contiene '25'
test('all locales define the same keys', ...)       // parsea los 5 .ftl y compara el conjunto de ids
test('applyTheme sets css variables', ...)          // document.documentElement.style.getPropertyValue('--hud-primary') === '#ff7100'
```

- [ ] **Step 3:** `npm test` en `ui/` → FAIL. **Step 4:** implementar. **Step 5:** `npm test && npm run check && npm run build` → PASS.
- [ ] **Step 6: Commit** `feat(ui): cliente WebSocket, i18n Fluent, tema HUD y shell con rejilla`

---

### Task 16: UI — paneles del panel general y ajustes

**Files:**
- Create: `ui/src/modules/dashboard/{CmdrPanel.svelte, ShipPanel.svelte, LocationPanel.svelte, StatusPanel.svelte, RoutePanel.svelte, SessionPanel.svelte, AlertsPanel.svelte, index.ts}`, `ui/src/modules/registry.ts`, `ui/src/settings/{Settings.svelte, Pairing.svelte}`, `ui/tests/dashboard.test.ts`, `ui/tests/e2e/smoke.spec.ts`, `ui/playwright.config.ts`
- Modify: `crates/tmc-server` (feature `embed-ui` activa por defecto en las apps), `crates/tmc-app/src/config.rs` (endpoint de ajustes) si hace falta.

**Interfaces:**
- Consumes: `topic()`, `t()`, `action()`, formatos (Task 15); topics `core.cmdr`, `core.ship`, `core.location`, `core.status`, `core.route`, `core.session`, `dashboard.alerts`; `GET /api/pairing`.
- Produces:
  - `registry.ts`: `export const modules: { id: string; icon: string; titleKey: string; panels: Component[] }[]` con `dashboard` (añadir un módulo = una entrada).
  - Paneles (contenido según spec §8.5): Comandante (créditos, 8 rangos con barra de progreso), Nave (tipo localizado, nombre/ident, casco %, combustible principal + depósito en barra, carga usada/capacidad, rebuy, módulos dañados), Ubicación (sistema, cuerpo, estación, modo), Status en vivo (chips: escudos, hardpoints, tren, masa bloqueada, silencio, colector, SCO, pips como 3 barras), Ruta (siguiente sistema, saltos restantes, lista de saltos, botón «copiar» → `action('clipboard.copy-next-system')` + toast `toast-copied`), Sesión (duración que se recalcula **solo cada 30 s**, saltos, distancia, créditos ganados), Alertas (lista de `dashboard.alerts`).
  - Estado vacío en cada panel: `t('empty-no-data')`.
  - Ajustes: idioma (5), tema (HUD / clásico / alto contraste — este último fija `--hud-primary #FFFFFF`, `--hud-bg #000000`), auto-copia, modo LAN (on/off) y `Pairing.svelte` que muestra el QR SVG de `/api/pairing`. Persistencia: `GET/PUT /api/config` (añadir en `tmc-server` + `tmc-app`, valida y guarda `Config`; los cambios de LAN aplican tras reiniciar y la UI lo indica).

- [ ] **Step 1: Tests:**

```ts
test('ship panel shows fuel bar and hull percent', ...)        // topics simulados → texto "87 %" y barra con ancho proporcional
test('route panel copy button calls action', ...)              // mock fetch POST /api/actions/clipboard.copy-next-system
test('panels show empty state without data', ...)
test('session duration does not re-render faster than 30 s', ...) // fake timers
```

`smoke.spec.ts` (Playwright, `webServer` = `cargo run -p tmc-headless -- --journal-dir <tmp>` alimentado con `tmc-replay play --from tests/fixtures/journal/basic_session.log --speed 50`): la página carga, el TopBar muestra el sistema final del fixture y no hay errores en consola.

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `npm test && npm run check && npx playwright test` → PASS; `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** `feat(ui): panel general, ajustes y emparejamiento LAN`

---

### Task 17: `apps/desktop` — Tauri 2, bandeja e instalador

**Files:**
- Create: `apps/desktop/{Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json, icons/*, src/main.rs, src/tray.rs, src/window.rs}`
- Modify: `Cargo.toml` (añadir `apps/desktop` a `members`)

**Interfaces:**
- Consumes: `tmc_app::{Config, start, Running}`, `tmc_i18n::I18n`.
- Produces: binario `TMC.exe`.
  - Arranque: plugin `tauri-plugin-single-instance` (segunda instancia → muestra la ventana existente); `start(config)` en el runtime de Tauri; ventana principal `main` creada con `WebviewUrl::External(running.server.local_url)`, título «TMC — The Multipurpose Companion», tamaño inicial 1600×900, recuerda posición/tamaño (`tauri-plugin-window-state`).
  - **Cerrar ventana = ir a la bandeja destruyendo el webview** (`CloseRequested` → `prevent_close` no; se cierra la ventana y la app sigue viva con el núcleo y el servidor, liberando los procesos de WebView2). Menú de bandeja (textos con `I18n`): `tray-open` (recrea la ventana), `tray-quit` (apaga servidor y sale). Clic izquierdo en el icono = abrir.
  - **Minimizar**: en `WindowEvent::Resized` con ventana minimizada → `webview.with_webview` → `ICoreWebView2Controller::SetIsVisible(false)` y `ICoreWebView2_19::SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW)`; al restaurar, lo inverso (solo `#[cfg(windows)]`; crate `webview2-com`).
  - `tauri.conf.json`: `identifier = "io.github.danikuso.tmc"`, `productName = "TMC"`, bundle `targets = ["nsis"]`, `windows.webviewInstallMode = { type = "downloadBootstrapper" }`, `nsis.languages = ["Spanish","English","French","German","Portuguese"]`, sin `devUrl` (la UI la sirve el servidor), `beforeBuildCommand = "npm --prefix ../../ui run build"`.
  - `capabilities/default.json`: mínimas (ventana principal, sin `shell`/`fs` expuestos al JS — la UI habla solo con el servidor local).

- [ ] **Step 1: Verificación de build**

Run: `cargo tauri build` (desde `apps/desktop`)
Expected: genera `target/release/bundle/nsis/TMC_<versión>_x64-setup.exe`.

- [ ] **Step 2: Prueba manual** (anotar resultado en el PR/commit): instalar el `setup.exe`; arranca, muestra el panel con tus datos reales; cerrar → queda en bandeja y desaparecen los procesos `msedgewebview2.exe` hijos de TMC (comprobar en el Administrador de tareas); «Abrir» desde la bandeja la recrea; una segunda ejecución no abre otra instancia; «Salir» libera el puerto 47320.

- [ ] **Step 3: Commit** `feat(desktop): app Tauri con bandeja, ahorro de memoria e instalador NSIS`

---

### Task 18: Presupuesto de consumo y cierre de F0

**Files:**
- Create: `scripts/measure-footprint.ps1`
- Modify: `README.md` (cómo compilar, ejecutar, usar replay y modo LAN), `THIRD_PARTY_NOTICES.md` (dependencias finales)

**Interfaces:**
- Produces: `scripts/measure-footprint.ps1 -Exe <ruta a TMC.exe> -JournalDir <carpeta> -Seconds 120 [-MaxMB 150] [-MaxCpu 1.0]`: arranca TMC con `--journal-dir`, lanza en paralelo `tmc-replay play --speed 5` sobre `tests/fixtures/journal/real_01.log`, muestrea cada 2 s la **memoria privada (Private Bytes)** y la CPU del árbol de procesos (TMC + `msedgewebview2.exe` cuyo proceso padre pertenece al árbol), descarta los primeros 20 s (arranque e importación), e imprime `max_mb`, `avg_cpu_idle` (muestras de los últimos 30 s, ya sin replay) y `PASS`/`FAIL`; código de salida 1 si falla.

- [ ] **Step 1: Ejecutar** `pwsh scripts/measure-footprint.ps1 -Exe target/release/TMC.exe -JournalDir $env:TEMP\tmc-replay -Seconds 120`
Expected: `PASS` (< 150 MB, < 1 % CPU en reposo). Si falla, abrir incidencia con los números y no cerrar F0.
- [ ] **Step 2: Revisar los criterios de aceptación §15 de la especificación** uno a uno y anotar la evidencia (comando + salida) en `docs/plans/2026-10-17-tmc-f0-cimientos.md` bajo un apartado «Verificación F0».
- [ ] **Step 3: Commit** `chore: script de presupuesto de consumo y documentación de F0`

---

## Desviaciones conscientes respecto a la especificación (para F0)

- **Disposiciones guardadas por perfil y cambio automático de disposición** (spec §8.1, §5.3): F0 entrega la rejilla adaptable y la relevancia por módulo (`core.modules`), pero la edición/guardado de disposiciones y el salto automático llegan en F1, cuando haya más de un módulo que reordenar.
- **`Module::migrations()`** (spec §5.2) y escritura en tablas propias desde `ModuleCtx`: se añaden cuando el primer módulo lo necesite (F1). El esquema F0 de §6 sí se crea completo.
- **Secretos en `keyring`** (spec §9): F0 no maneja ningún secreto (no hay servicios online); se introduce en F6.


## Reconciliación y verificación incremental (2026-10-17)

- El propietario ratifica la aprobación registrada de spec y plan y autoriza revisar Task 4 existente antes de continuar con Task 5.
- Task 4 ya estaba implementada en 70e4999 y 177eac1; no se reconstruye su ciclo TDD original. Comprobación inicial: cargo fmt --all --check, cargo clippy --workspace --all-targets -- -D warnings y cargo test --workspace terminan con código 0 (31 tests).
- Revisión independiente de Task 4: corregir notificación de creación de estado ante Rank/Progress/Loadout parciales; activar game_running en Fileheader sin reiniciar contadores; alinear HudTheme por defecto con la paleta de Task 7. Se añaden regresiones con ciclo RED/GREEN observado.
- UI aún no existe (Task 15); checks npm no aplicables en este punto. El script de consumo se crea en Task 18; aún no hay aplicación ejecutable para medir.

- Cierre Task 4: regresiones RED (31 PASS/4 FAIL) y GREEN (35 PASS). Revisión de correcciones aprobada. Verificación global: cargo fmt --all --check, cargo clippy --workspace --all-targets -- -D warnings y cargo test --workspace, todos código 0. Avisos de caché incremental por hard links: Rust copia archivos como alternativa; sin fallos de compilación.

- Cierre Task 5: cargo test -p tmc-journal --offline con stubs termina con 9 FAIL (RED); tras implementación, 9 PASS (GREEN). Revisión independiente aprobada, sin hallazgos bloqueantes. cargo fmt --all --check, cargo clippy --workspace --all-targets --offline -- -D warnings y cargo test --workspace --offline: código 0, 44 tests PASS (9 journal + 35 model). git diff --check: código 0.
- Task 5 no cambia las interfaces previstas. Dependencias directas nuevas: windows 0.61.3 (solo Windows; MIT OR Apache-2.0), tempfile 3.27.0 (tests; MIT OR Apache-2.0), registradas en THIRD_PARTY_NOTICES.md. Se resolvió la restricción DNS mediante cargo fetch autorizado; sin downgrades de dependencias previas.
- Límite de verificación: ejecución en Windows; helpers Linux probados con carpetas temporales, pendientes ejecución nativa Linux y comprobación manual con Saved Games redirigida. No hay app ni script de consumo todavía; Task 5 no incorpora bucles ni tareas en segundo plano.


## Decisión aprobada del propietario: dependencia de importación (Task 8)

La spec §3.1 declara que tmc-store depende solo de tmc-model. Task 8 sitúa import_history en tmc-store y exige consumir list_journals y LineTailer de tmc-journal. El ledger anterior considera 5,6→8 correcto, pero R1–R5 no autorizan esa dependencia.

Cambio mínimo aprobado explícitamente por el propietario en esta conversación (2026-10-17): en spec §3.1, fila tmc-store, sustituir «tmc-model» por «tmc-model, tmc-journal». Mantiene las firmas del plan y el grafo acíclico store → journal → model. Tras implementarlo, verificar cargo metadata y tests de importación. Tasks 6–7 pueden continuar mientras se decide.

- Task 6: RED observado en 11 tests del lector incremental. offset() persiste hasta el último LF entregado; read_pos privado conserva la lectura física y la cola parcial, evitando pérdida al reabrir. Semántica concretada sin cambiar interfaces.

- Cierre Task 6: RED 11 FAIL → GREEN 11 nuevos tests PASS (20 journal). Revisión independiente aprobada. cargo fmt --all --check, cargo clippy --workspace --all-targets --offline -- -D warnings, cargo test --workspace --offline: código 0; 55 tests PASS (20 journal + 35 model). git diff --check: código 0.

- Cierre Task 7: RED 20 PASS/11 FAIL; GREEN inicial34 journal. Revisión detectó matrices XML comentadas/malformadas y Status inicial posterior a live. Regresiones RED observadas (HUD3PASS/2FAIL; watcher 'live journal preceded initial Status'); corregidas con roxmltree y barrera inicial de Status. Revisión final aprobada.
- Verificación Task 7: cargo fmt --all --check; cargo clippy --workspace --all-targets --offline -- -D warnings; cargo test --workspace --offline --quiet; git diff --check: código 0. 72 tests PASS (37 journal+35 model). Prueba Status corrupto durante live conserva entrega de journal en menos de 400 ms con tick de 20 ms. No equivale aún a medición p95 de la app final; consumo pendiente Task18.

- Comprobación previa Task 8: cargo metadata --no-deps --offline --format-version 1 confirma journal → model y store → journal + model, conforme a la tabla aprobada, sin ciclos. Store comparte conexión mediante Arc<Mutex<Connection>> para soportar clones y escritor en memoria; no afecta al modelo actor/GameState.


### Desviación Task 8: errores explícitos al confirmar escritura (2026-10-17)

La revisión encontró que flush_blocking sin retorno podía confirmar un lote fallido. Se cambia la firma a fn flush_blocking(&self) -> Result<(), StoreError>; las llamadas deben manejar el error. No cambia la spec: cumple su §10 (SQLite ocupado: reintento; fallos no tumban la app). El lote permanece en el escritor tras un fallo para permitir otro flush. En Drop se intenta drenar y se registra el fallo si SQLite impide hacerlo; no se espera indefinidamente. El diario original sigue siendo fuente de recuperación y el escritor nunca adelanta el offset del importador (R1). Las pruebas usan un bloqueo SQLite externo con timeout reducido para comprobar error y posterior recuperación. No afecta a firmas de Tasks 9–13 descritas; sus llamadas al método deberán manejar Result.

- Cierre Task 8: RED 15 FAIL → GREEN inicial 17; revisión de importación/migraciones aprobada y corrección de flush validada mediante bloqueo SQLite (RED falso éxito → GREEN 18 store). Revisión final aprobada. cargo fmt --all --check, cargo clippy --workspace --all-targets --offline -- -D warnings, cargo test --workspace --offline --quiet y git diff --check terminan con código 0: 90 tests PASS (37 journal + 35 model + 18 store).
- Riesgos para Task18: LineTailer materializa las líneas nuevas hasta EOF y Store serializa el acceso a una conexión; medir RAM y latencia de importaciones grandes antes de cerrar F0. No hay todavía aplicación final ni script de consumo para validar el presupuesto.

- Entorno para verificaciones posteriores: Node24.19.0 y npm11.17.0 disponibles; usar npm.cmd en PowerShell porque npm.ps1 está bloqueado por la política de scripts, sin cambiar esa política. Debian existe en WSL, pero no tiene cargo/rustc/cc/pkg-config en PATH; Docker Desktop está instalado pero su daemon Linux no está activo. Las pruebas nativas Linux siguen pendientes; no se han instalado herramientas ni alterado esos entornos durante esta comprobación.

- Cierre Task 9: RED 15 FAIL → GREEN 15 core; revisión detectó inanición de snapshots con eventos continuos. Regresión con productor sostenido: RED timeout de 150 ms → GREEN tras cuota de 32 eventos; test de las tres variantes CoreMessage incluido. Revisión final aprobada. Verificación global cargo fmt --all --check, cargo clippy --workspace --all-targets --offline -- -D warnings, cargo test --workspace --offline --quiet y git diff --check: código 0, 107 tests PASS (17 core + 37 journal + 35 model + 18 store).
- Task 9 concreta flush_every con mínimo de 100 ms para cumplir máximo de 10 Hz, y atiende snapshots tras como máximo 32 eventos consecutivos. Sin cambio de las firmas previstas; el cierre del writer maneja Result fuera del runtime.

- Cierre Task 10: RED 4 FAIL → GREEN 8 i18n. 67 claves y variables coherentes en los cinco idiomas, parseo/formato Fluent sin errores y bundle Send+Sync. Revisión aprobada. cargo fmt --all --check; cargo clippy --workspace --all-targets --offline -- -D warnings; cargo test --workspace --offline --quiet; git diff --check: código 0 y 115 tests PASS (17 core + 8 i18n + 37 journal + 35 model + 18 store).


### Desviación temporal Task 11 (2026-10-17)

La API on_event del plan no entrega ticks al módulo: el vencimiento de 60 s se evalúa con el siguiente evento Journal/Status que tenga timestamp, igual que el antirrebote de 30 s. Sin nuevos datos de juego no hay retirada autónoma exactamente al segundo 60. Esta limitación se mantiene explícita para conservar determinismo, contrato de módulo y ausencia de sondeos adicionales de rol en core. La spec no fija TTL de alertas; no cambia sus criterios de aceptación. Se verifica el borde exacto de 60 s y no se presenta esta comprobación como temporizador autónomo. Un futuro hook temporal genérico requeriría planificación explícita.

- Cierre Task 11: RED 11 FAIL → GREEN inicial 11 PASS; regresión de escalado de HullDamage dentro de ventana de antirrebote (RED 11 PASS/1 FAIL → GREEN 12 PASS). Revisión contra spec y plan aprobada. cargo fmt --all --check; cargo clippy --workspace --all-targets --offline -- -D warnings; cargo test --workspace --offline: código 0 y 127 tests PASS (17 core + 8 i18n + 37 journal + 35 model + 12 modules + 18 store).

- Cierre Task 12: RED 10 FAIL → GREEN 12 server (HTTP, WebSocket hello/snapshot/update/subscribe, validación anti DNS-rebinding de Host/Origin, autenticación LAN en tiempo constante con cookie HttpOnly SameSite=Strict, emparejamiento QR SVG solo loopback y estáticos con fallback SPA). Dependencias conformes a spec §3.1 (`tmc-server -> tmc-core, tmc-model`). cargo fmt --all --check; cargo clippy --workspace --all-targets --offline -- -D warnings; cargo test --workspace --offline: código 0 y 139 tests PASS (17 core + 8 i18n + 37 journal + 35 model + 12 modules + 12 server + 18 store).

- Cierre Task 13: RED 6 FAIL → GREEN 6 app (config TOML con respaldo `.bak` ante sintaxis inválida, token LAN hex de 128 bits, acción `clipboard.copy-next-system`, reconstrucción automática de `tmc.db` corrupta a `tmc.db.corrupt-<timestamp>` y arranque extremo a extremo con `apps/headless`). Paso 5 verificado con diarios reales (`GET /api/health` → `{"ok":true,"version":"0.1.0"}` y `history import completed files=6 events=2599 skipped_lines=0` en el log rotado). cargo fmt --all --check; cargo clippy --workspace --all-targets --offline -- -D warnings; cargo test --workspace --offline: código 0 y 145 tests PASS (6 app + 17 core + 8 i18n + 37 journal + 35 model + 12 modules + 12 server + 18 store).

- Cierre Task 14: RED 3 FAIL → GREEN 3 replay (`anonymize` y `play`). Generados fixtures reales anonimizados (`real_01.log` a `real_03.log`) y verificado con búsqueda insensible a mayúsculas que no permanece ningún nombre de comandante, FID ni chat real (0 coincidencias). Snapshot `insta` (`real_fixtures_end_state`) añadido y verificado en `tmc-model`. cargo fmt --all --check; cargo clippy --workspace --all-targets --offline -- -D warnings; cargo test --workspace --offline: código 0 y 149 tests PASS (6 app + 17 core + 8 i18n + 37 journal + 36 model + 12 modules + 3 replay + 12 server + 18 store).
