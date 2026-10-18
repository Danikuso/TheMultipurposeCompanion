# TMC F1 «Navegación, exploración y exobiología» — Plan de implementación

> **Para agentes:** SUB-SKILL OBLIGATORIA: usar superpowers:subagent-driven-development (recomendado) o superpowers:executing-plans para ejecutar este plan tarea a tarea. Los pasos usan casillas (`- [ ]`) para el seguimiento.

**Goal:** Convertir TMC en un compañero de exploración: tira de ruta con la siguiente estrella y las tres siguientes (ficha local + remota), alcance de salto real de la nave, planificador Spansh con copia automática, valor de los cuerpos del sistema con consejo de mapeo, y exobiología con predicción de especies y acompañamiento del muestreo.

**Architecture:** Se conserva la arquitectura de F0 (actor único dueño del estado, módulos sin E/S, difusión a 10 Hz). La red entra por un crate nuevo `tmc-net` (sin dependencias del núcleo) que la app compone como `NetService`; los módulos piden consultas con `ModuleCtx::request` y reciben `SourceEvent::Net`. Las lecturas/escrituras de SQLite de los módulos salen del actor como tareas (`ModuleCtx::store_task`) y vuelven como `SourceEvent::Stored`. Tres módulos nuevos (`navigation`, `exploration`, `exobiology`) con datos estáticos propios embebidos.

**Tech Stack:** el de F0 (Rust 2024, tokio, serde, rusqlite, axum, Fluent, Tauri 2, Svelte 5 + TS + Vite, Vitest, Playwright) + `reqwest` (sin features por defecto, TLS con `rustls`; nombre exacto de la feature en la versión fijada: verificar en código al ejecutar).

**Spec:** [`docs/specs/2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md`](../specs/2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md) (aprobada). Base: [`2026-10-17-tmc-arquitectura-y-f0-design.md`](../specs/2026-10-17-tmc-arquitectura-y-f0-design.md). Dominio: `docs/research/notes/player_roles_domain.md` §1–§2, `docs/research/notes/community_apis_core.md` (EDSM), `docs/research/notes/specialized_apis_static_data.md` §1 (Spansh).

## Restricciones globales

- Código e identificadores en inglés; documentación, commits y textos de referencia (`es`) en español.
- Presupuesto (requisito): **< 150 MB** RAM del árbol de procesos, **< 1 %** CPU en reposo, difusión a la UI **máx. 10 Hz**, latencia evento→UI **p95 < 500 ms**. Sin animaciones infinitas ni `requestAnimationFrame` continuo.
- Red: timeout **5 s** por petición; reintentos con retroceso **30 s → 2 min → 10 min**; **3 fallos seguidos = servicio caído 10 min** y se prueba la alternativa. Límites: Spansh **1 petición/s y una ruta a la vez**, Canonn **1 petición/s**, EDSM **10 peticiones/10 s**. `User-Agent: TMC/<versión>`. Caché `api_cache`: ficha de sistema 24 h, política 1 h, Canonn 30 días, rutas sin caché.
- Nunca se consulta nada con `Galaxy::Legacy`, ni con el ajuste «Consultar servicios online» desactivado. Nunca se envía nada del comandante (solo nombres/ids de sistemas y parámetros de nave).
- Las peticiones de red y las consultas a SQLite **nunca** se ejecutan dentro del actor.
- Licencias: PolyForm Noncommercial 1.0.0. **Prohibido** incorporar código o datos GPL/AGPL (p. ej. `bio-criteria` de SrvSurvey). Las respuestas de Canonn/Spansh/EDSM solo van a `api_cache`, nunca al repositorio (los fixtures grabados en `tests/fixtures/net/` contienen solo datos públicos de la galaxia y se autorizan en los spikes). Toda dependencia o fuente de datos nueva se anota en `THIRD_PARTY_NOTICES.md`. Datos CC-BY/CC-BY-SA **no** se incorporan al repo sin aprobación del propietario; las dependencias transitivas de `rustls`/`reqwest` se aceptan si su licencia es permisiva y se anotan en `THIRD_PARTY_NOTICES.md` (verificar en Task 10).
- i18n: ninguna cadena visible en el código; claves nuevas en `locales/{es,en,fr,de,pt}/{navigation,exploration,exobiology}.ftl` (y `main.ftl` para ajustes); los 5 idiomas con **las mismas claves** (test). Traducciones reales, no copias del español.
- **Checks** (antes de cada commit): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; si se toca `ui/`: `npm --prefix ui run check`, `npm --prefix ui test`, `npm --prefix ui run build`.
- **Todo test afirma un valor concreto; no hay tests vacíos** (ningún cuerpo `{}` ni solo comentario): cada test compara contra un valor esperado escrito en el propio test.
- Commits pequeños (uno por tarea como mínimo), en español con prefijo convencional y la línea final `Co-Authored-By: <modelo que ejecuta> <noreply@anthropic.com>` (AGENTS.md §7). **Sin push.** Actualizar `status.md` al empezar y terminar cada tarea (AGENTS.md §4).
- Funciones compartidas, una sola fuente: la clase de estrella (`scoopable`, `special`) vive en `data::star_flags` (Tasks 13/15/21); el libro de datos sin vender se actualiza con `UnsoldLedger::apply_event`, también al reconstruir y en `eval`; `auto_copy` tiene una sola fuente (`Config.auto_copy`, expuesto como `ModuleSettings.auto_copy_next_system`).

## Foco de revisión

Entradas que la spec implica, que ningún test de F0 cubre y que más pueden morder al usuario. Cada una tiene su test en la tarea dueña:

1. **Respuesta remota tardía tras otro salto** (la ficha de X llega cuando X ya salió de la tira, o llega una respuesta vieja después de una nueva): se guarda en caché pero no se aplica ni pisa datos más nuevos → `late_response_for_system_no_longer_in_strip_is_not_applied` y `older_response_does_not_overwrite_newer` (Task 15); agrupación de peticiones idénticas → `identical_inflight_requests_are_coalesced` (Task 11).
2. **Sistema con > 100 cuerpos** (topic grande, UI lenta): el topic sigue acotado y ordenado, y la tabla se virtualiza → `system_with_300_bodies_keeps_topic_under_64_kib` (Task 21) y `body_table_renders_only_visible_rows` (Task 23).
3. **Loadout sin FSD conocido o ingeniería rara** (FSD nuevo no tabulado, modificador desconocido, sin módulo FSD, sin `MaxJumpRange`): nunca bloquea el planificador; respaldo escalado o «sin datos» → `unknown_fsd_item_falls_back_to_scaled_max_jump_range`, `unknown_modifier_is_ignored_and_validation_decides`, `loadout_without_fsd_hides_fuel_after_jump` (Task 13).
4. **Muestreo interrumpido por reinicio de la app o por muerte**: tras reiniciar (incluso con diario nuevo) se recuperan las muestras y su posición desde `exo_sample`; `Died` descarta muestreo y valor sin vender → `sampling_resumes_after_restart_from_exo_sample`, `died_clears_sampling_tracker` (Task 26) y `died_zeroes_unsold_bio` (Task 27).
5. **Servicios online desactivados o caídos**: la tira muestra lo local y «sin datos», el planificador se desactiva con explicación, tres fallos pasan a EDSM → `disabled_online_services_never_reach_transport` (Task 11), `three_failures_mark_spansh_down_and_use_edsm` (Task 10), `strip_shows_local_card_and_no_data_when_services_disabled` (Task 15).

## Resumen de tareas

| # | Tarea | Bloque |
|---|---|---|
| 0 | Spike 1 — Spansh (ficha por id64, rutas, autocompletado) | 0 |
| 1 | Spike 2 — licencias de tabla FSD y datos de especies | 0 |
| 2 | Spike 3 — Canonn (API y licencia) | 0 |
| 3 | Aprobaciones previas del propietario | 0 |
| 4 | `tmc-model` — eventos de exploración y exobiología | 1 |
| 5 | `tmc-model` — Loadout con ingeniería, ubicación ampliada, Touchdown/Embark | 1 |
| 6 | `tmc-model` — `NetRequest`/`NetResponse`, `ModuleSettings`, nuevas `SourceEvent` | 1 |
| 7 | `tmc-store`/`tmc-core` — API pública de SQL y `api_cache`, peticiones de red, tareas de SQLite y migraciones de módulo | 1 |
| 8 | i18n — varios `.ftl` por idioma (Rust y UI) | 1 |
| 9 | `tmc-net` — limitador, salud del servicio y política de caché | 1 |
| 10 | `tmc-net` — adaptadores Spansh/EDSM/Canonn y `NetClient` | 1 |
| 11 | `tmc-app` — `NetService`, configuración nueva y ajustes en caliente | 1 |
| 12 | `tmc-server` — acciones con argumentos, despacho a módulos y autocompletado | 1 |
| 13 | `navigation` — alcance de salto y combustible | 2 |
| 14 | `navigation` — ruta activa y reglas de avance | 2 |
| 15 | `navigation` — tira de ruta (ficha local + remota) | 2 |
| 16 | `navigation` — planificador Spansh y persistencia `nav_route_plan` | 2 |
| 17 | `tmc-app` — copia automática y manual desde la ruta activa | 2 |
| 18 | UI — disposiciones, ancho completo, tipografía, tira de ruta y panel de alcance | 2 |
| 19 | UI — panel del planificador | 2 |
| 20 | `exploration` — valor de cuerpos y sondas | 3 |
| 21 | `exploration` — cuerpos del sistema, FSS y consejo | 3 |
| 22 | `exploration` — datos sin vender | 3 |
| 23 | UI — panel «Cuerpos del sistema» | 3 |
| 24 | `exobiology` — tabla de especies y geodesia (haversine/rumbo) | 4 |
| 25 | `exobiology` — predicción (reglas + historial + Canonn) | 4 |
| 26 | `exobiology` — muestreo y `exo_sample` | 4 |
| 27 | `exobiology` — módulo, bonus ×5, bio sin vender y relevancia | 4 |
| 28 | UI — panel de exobiología | 4 |
| 29 | `tmc-headless eval` (criterios 3, 4 y 6) | 5 |
| 30 | Fixtures anonimizados y pruebas de extremo a extremo (criterios 2 y 5) | 5 |
| 31 | Disposición «Exploración» por defecto y ajustes nuevos en la UI | 5 |
| 32 | Consumo, verificación de los 10 criterios y cierre de F1 | 5 |

## Mapa de archivos

```
crates/tmc-model/src/{events.rs*, source.rs*, net.rs+, settings.rs+, lib.rs*}
crates/tmc-store/src/lib.rs*                       (query_json, exec_json, api_cache_get/put)
crates/tmc-core/src/{module.rs*, actor.rs*, lib.rs*}   (lib.rs re-exporta Store, StoreError, Migration)
crates/tmc-net/{Cargo.toml+, src/lib.rs+, src/limiter.rs+, src/health.rs+, src/cache_policy.rs+,
                src/transport.rs+, src/spansh.rs+, src/edsm.rs+, src/canonn.rs+, src/client.rs+}
crates/tmc-modules/data/{fsd.json+, species.json+, bio_rules.json+, probes.json+}
crates/tmc-modules/src/{lib.rs*, registry.rs*, data.rs+, eval.rs+}
crates/tmc-modules/src/navigation/{mod.rs+, jump.rs+, route.rs+, strip.rs+, planner.rs+, persist.rs+}
crates/tmc-modules/src/exploration/{mod.rs+, value.rs+, system.rs+, unsold.rs+}
crates/tmc-modules/src/exobiology/{mod.rs+, species.rs+, geo.rs+, predict.rs+, sampling.rs+, persist.rs+}
crates/tmc-modules/src/dashboard/mod.rs*          (respeta settings.alerts_enabled)
crates/tmc-i18n/src/lib.rs*                        (carga todos los .ftl del idioma)
crates/tmc-app/src/{config.rs*, clipboard.rs*, wiring.rs*, net_service.rs+, lib.rs*}
crates/tmc-server/src/{lib.rs*, rest.rs*, ws.rs*}
apps/headless/{Cargo.toml*, src/main.rs*, src/eval.rs+}
locales/{es,en,fr,de,pt}/{main.ftl*, navigation.ftl+, exploration.ftl+, exobiology.ftl+}
ui/src/lib/{i18n.ts*, ws.svelte.ts*, format.ts*}
ui/src/shell/{Grid.svelte*, Panel.svelte*, Sidebar.svelte*, layouts.ts+, VirtualList.svelte+}
ui/src/App.svelte*  ui/src/styles/tokens.css*  ui/src/settings/Settings.svelte*
ui/src/modules/registry.ts*
ui/src/modules/navigation/{index.ts+, RouteStrip.svelte+, SystemCard.svelte+, JumpPanel.svelte+, PlannerPanel.svelte+}
ui/src/modules/exploration/{index.ts+, SystemBodiesPanel.svelte+}
ui/src/modules/exobiology/{index.ts+, ExobiologyPanel.svelte+}
ui/tests/{navigation.test.ts+, exploration.test.ts+, exobiology.test.ts+, layouts.test.ts+}
tests/fixtures/net/*.json+                         (respuestas grabadas en los spikes)
tests/fixtures/journal/{exploration_session.log+, exobiology_session.log+, real_explo_*.log+}
tests/fixtures/status/exo_*.json+
docs/research/notes/{spike_spansh_f1.md+, spike_licencias_f1.md+, spike_canonn_f1.md+}
THIRD_PARTY_NOTICES.md*  Cargo.toml* (workspace: reqwest)
```
(`+` crear, `*` modificar.)

---

## Bloque 0 — Spikes y aprobaciones

Las tareas de este bloque no escriben código de producción. Las peticiones reales se hacen a mano (curl o un script desechable en el scratchpad, nunca en el repo), a ritmo ≤ 1 petición/s.

### Task 0: Spike 1 — Spansh

**Files:** Create `docs/research/notes/spike_spansh_f1.md`, `tests/fixtures/net/spansh_system_ok.json`, `spansh_system_404.json`, `spansh_route_job.json`, `spansh_route_neutron_result.json`, `spansh_route_galaxy_result.json`, `spansh_autocomplete.json`.

**Interfaces:** Produces la tabla «campo de Spansh → campo interno» para `RemoteSystem` y `PlannedHop` (Task 6) y las URL/métodos que usará `tmc_net::spansh` (Task 10).

- [ ] **Step 1:** Verificar con peticiones reales: (a) ficha por id64 (`GET /api/system/{id64}` y/o `/api/dump/{id64}`): ¿trae descubierto, nº de cuerpos, estrella principal, allegiance, gobierno, facción controladora y su estado, economías, seguridad, población, Powerplay (potencia y estado)?; (b) ruta de neutrones (`/api/route` + sondeo `/api/results/{job}`): parámetros (`from`, `to`, `range`, `efficiency`, `supercharge_multiplier`), forma del resultado (saltos, distancia, neutrón, repostar); (c) ruta de galaxia (endpoint, parámetros de nave mínimos: masa, combustible, FSD, opciones repostar/inyecciones); (d) autocompletado de nombres (endpoint y forma); (e) tiempos de respuesta y cabeceras de límite.
- [ ] **Step 2:** Documentar en `spike_spansh_f1.md`: endpoint, método, parámetros, ejemplo de respuesta recortado, mapeo a campos internos, campos ausentes, latencia observada, errores (404, job fallido) y fecha de la prueba.
- [ ] **Step 3:** Grabar las respuestas (recortadas a lo necesario) en `tests/fixtures/net/`; comprobar que no contienen datos de comandantes.
- [ ] **Criterio de decisión:** si la ficha por id64 no trae política/Powerplay, esos campos se piden a EDSM (`api-v1/system` con `showInformation=1`) y se anota como desviación; si el planificador de galaxia exige datos de nave que el `Loadout` no da, F1 entrega «Galaxia» con los parámetros disponibles y lo anota. La interfaz interna (`NetQuery`/`NetOutcome`, Task 6) **no cambia**; solo el adaptador.
- [ ] **Step 4: Commit** `docs: spike de Spansh para F1 y respuestas grabadas`

### Task 1: Spike 2 — licencias de la tabla FSD y de los datos de especies

**Files:** Create `docs/research/notes/spike_licencias_f1.md`.

- [ ] **Step 1:** Para cada candidata (coriolis-data, EDSY, wiki de Fandom, tablas de Canonn, `codexRef` de Canonn/SrvSurvey): licencia exacta (enlace al archivo LICENSE), qué datos aportaría (FSD clase/rating → masa óptima, combustible máx. por salto, coeficiente, exponente, incl. SCO y Mk II; booster Guardian; valores y CCR por especie; nombres de géneros/especies) y compatibilidad con PolyForm NC.
- [ ] **Step 2:** Documentar la decisión por archivo de datos (`fsd.json`, `species.json`, `bio_rules.json`, `probes.json`): «se incorpora con aviso» (solo MIT/BSD/Apache/CC0) o «se construye desde hechos públicos y se verifica contra el historial» (cualquier otra, incluidas CC-BY, CC BY-SA y GPL/AGPL; incorporar datos CC-BY/CC-BY-SA exige aprobación expresa del propietario, que se registra en Task 3).
- [ ] **Criterio de decisión:** ante la duda, construir desde hechos y preguntar al propietario (AGENTS.md §6). SrvSurvey queda excluido sin excepción.
- [ ] **Step 3: Commit** `docs: spike de licencias de datos estáticos de F1`

### Task 2: Spike 3 — Canonn

**Files:** Create `docs/research/notes/spike_canonn_f1.md`, `tests/fixtures/net/canonn_body_ok.json`, `canonn_body_empty.json` (si el servicio existe).

- [ ] **Step 1:** Verificar con peticiones reales qué ofrece Canonn por cuerpo (id64 + BodyID o nombre) y/o por especie (estadística de cuerpos: tipo, atmósfera, gravedad, temperatura), endpoints, límites y **licencia de los datos**.
- [ ] **Step 2:** Documentar: endpoint(s), forma de la respuesta, cómo se traduce a `RemoteBodyBio { hints: Vec<RemoteSpeciesHint { genus, species, weight }> }` (Task 6), latencia, y licencia.
- [ ] **Criterio de decisión:** si no hay endpoint usable por cuerpo o por propiedades, o la licencia no permite ni la consulta, la capa 3 se desactiva (`NetQuery::BodyBio` devuelve `NetOutcome::Disabled` desde el adaptador) y se anota como desviación; F1 sigue con capas 1 y 2 sin bloqueo (spec §11).
- [ ] **Step 3: Commit** `docs: spike de Canonn para la predicción de exobiología`

### Task 3: Aprobaciones previas del propietario

**Files:** Modify `status.md` (sección «Bloqueos»).

- [ ] **Step 1:** Registrar en «Bloqueos» y pedir aprobación de:
  1. ~~Dependencias de `tmc-replay`~~ — **resuelto por el propietario (2026-10-17): `eval` vive en `tmc-headless eval`**; no se toca la arquitectura (§3.1).
  2. Resultados de los spikes que cambien alcance (p. ej. Canonn desactivado, política vía EDSM, ToS de Spansh).
- [ ] **Step 2:** Las tareas 4–28 pueden avanzar mientras tanto; el punto 1 ya está resuelto, así que la Task 29 no espera a nada.
- [ ] **Step 3: Commit** `docs(status): aprobaciones pendientes antes de F1`

---

## Bloque 1 — Eventos, red y contratos del núcleo

### Task 4: `tmc-model` — eventos de exploración y exobiología

**Files:** Modify `crates/tmc-model/src/events.rs`; Create `tests/fixtures/journal/exploration_session.log` (≈30 líneas escritas a mano, `CMDR TESTER`, sistemas ficticios).

**Interfaces:**
- Produces (structs con `#[serde(default, rename_all = "PascalCase")]`, campos `Option` salvo los imprescindibles, `Debug + Clone + PartialEq + Serialize + Deserialize + Default`):
  - `Scan { scan_type, body_name: String, body_id: Option<u32>, parents: Vec<BTreeMap<String, u32>>, star_system, system_address: Option<u64>, distance_from_arrival_ls: Option<f64>, star_type, subclass: Option<u32>, stellar_mass, radius, planet_class, terraform_state, atmosphere, atmosphere_type, atmosphere_composition: Vec<Composition { name, percent }>, volcanism, mass_em, surface_gravity, surface_temperature, surface_pressure, landable: Option<bool>, materials: Vec<Composition>, was_discovered: Option<bool>, was_mapped: Option<bool>, was_footfalled: Option<bool> }` (`DistanceFromArrivalLS` y `MassEM` con `rename` explícito).
  - `FssDiscoveryScan { progress, body_count: Option<u32>, non_body_count: Option<u32>, system_name, system_address }`, `FssAllBodiesFound { system_name, system_address, count }`.
  - `Signal { type_ (rename "Type"), type_localised (rename "Type_Localised"), count: u32 }`, `GenusRef { genus, genus_localised }`.
  - `FssBodySignals { body_name, body_id, system_address, signals: Vec<Signal> }`, `SaaScanComplete { body_name, body_id, system_address, probes_used: Option<u32>, efficiency_target: Option<u32> }`, `SaaSignalsFound { body_name, body_id, system_address, signals: Vec<Signal>, genuses: Vec<GenusRef> }`.
  - `pub enum OrganicScanType { Log, Sample, Analyse, Unknown }` (desconocido → `Unknown`, sin error); `ScanOrganic { scan_type: OrganicScanType, genus, genus_localised, species, species_localised, variant, variant_localised, system_address, body: Option<u32> }`.
  - `SellOrganicData { market_id, bio_data: Vec<BioSale { genus, species, variant, value: u64, bonus: u64 }> }`, `SellExplorationData { systems: Vec<String>, discovered: Vec<String>, base_value, bonus, total_earnings }`, `MultiSellExplorationData { discovered: Vec<SoldSystem { system_name, num_bodies }>, base_value, bonus, total_earnings }`.
  - `CodexEntry { entry_id, name, name_localised, category, sub_category, system_address, body_id, is_new_entry, voucher_amount, latitude, longitude }`, `JetConeBoost { boost_value: Option<f64> }`.
  - Variantes nuevas de `EventKind`: `Scan`, `FssDiscoveryScan`, `FssAllBodiesFound`, `FssBodySignals`, `SaaScanComplete`, `SaaSignalsFound`, `ScanOrganic`, `SellOrganicData`, `SellExplorationData`, `MultiSellExplorationData`, `CodexEntry`, `JetConeBoost` (nombres del diario: `FSSDiscoveryScan`, `FSSAllBodiesFound`, `FSSBodySignals`, `SAAScanComplete`, `SAASignalsFound`).
  - Método de ayuda: `impl Signal { pub fn is_biological(&self) -> bool }` (`$SAA_SignalType_Biological;`) e `is_geological`.

- [ ] **Step 1: Tests** (en `events.rs`):

```rust
#[test] fn parses_detailed_scan_of_landable_planet() { /* PlanetClass "High metal content body", MassEM 0.12, SurfaceGravity 2.1, Landable true, WasDiscovered false, WasMapped false, WasFootfalled false, BodyID 7 */ }
#[test] fn parses_star_scan() { /* StarType "N", StellarMass 1.4 → planet_class None */ }
#[test] fn fss_body_signals_detects_biological_count() { /* Signals [{Type:"$SAA_SignalType_Biological;",Count:3}] → is_biological, count 3 */ }
#[test] fn saa_signals_found_lists_genuses() { /* Genuses [{Genus:"$Codex_Ent_Bacterial_Genus_Name;"}] → 1 */ }
#[test] fn scan_organic_types_and_unknown() { /* "Log"→Log, "Analyse"→Analyse, "Foo"→Unknown */ }
#[test] fn sell_organic_data_values_and_bonus() { /* BioData[0].Value 1000000, Bonus 4000000 */ }
#[test] fn multi_sell_exploration_data_parses() { /* Discovered [{SystemName:"Sys A",NumBodies:7},{SystemName:"Sys B",NumBodies:3}], BaseValue 100000, Bonus 20000, TotalEarnings 120000 → discovered.len()==2, discovered[0].num_bodies==7, total_earnings==120000 */ }
#[test] fn every_line_of_exploration_session_fixture_is_typed() { /* 0 errores de parseo; el conjunto de nombres de evento que caen en Other es exactamente EXPECTED_OTHER, constante explícita del test (p. ej. ["Fileheader","Music"], los eventos no tipados que el fixture contiene a propósito) */ }
```

- [ ] **Step 2:** `cargo test -p tmc-model` → FAIL.
- [ ] **Step 3:** Implementar los structs y su clasificación en `classify`. Un struct que no deserialice sigue cayendo en `Other`.
- [ ] **Step 4:** `cargo test -p tmc-model` → PASS; Checks.
- [ ] **Step 5: Commit** `feat(model): eventos de exploración y exobiología tipados`

### Task 5: `tmc-model` — Loadout con ingeniería, ubicación ampliada, Touchdown/Embark

**Files:** Modify `crates/tmc-model/src/events.rs`, `crates/tmc-model/src/state.rs` (ajustar `match` de `Touchdown`/`Liftoff`/`ApproachBody`).

**Interfaces:**
- Consumes: `LoadoutModule`, `LocationInfo`, `BodyRef`, `EventKind::{Touchdown, Liftoff}` existentes.
- Produces:
  - `LoadoutModule` + `engineering: Option<Engineering>`; `Engineering { blueprint_name, level: Option<u32>, quality: Option<f64>, experimental_effect, modifiers: Vec<Modifier> }`; `Modifier { label: String, value: Option<f64>, original_value: Option<f64>, less_is_good: Option<u8> }`. (`Modifier.value` puede ser texto en algunos modificadores: si no es numérico → `None`, sin error; usar deserializador tolerante.)
  - `LocationInfo` + `body_id: Option<u32>`, `system_allegiance`, `system_government_localised`, `system_economy_localised`, `system_second_economy_localised`, `system_security_localised`, `population: Option<u64>`, `system_faction: Option<SystemFaction { name, faction_state }>`, `powers: Vec<String>`, `powerplay_state: Option<String>`, `boost_used: Option<f64>`.
  - `BodyRef` + `system_address: Option<u64>`, `body_id: Option<u32>`.
  - `EventKind::Touchdown(SurfaceEvent)` y `EventKind::Liftoff(SurfaceEvent)`; `SurfaceEvent { latitude, longitude, body, body_id, system_address, on_planet: Option<bool>, player_controlled: Option<bool> }`.
  - `EventKind::Embark(EmbarkEvent)`, `EventKind::Disembark(EmbarkEvent)`; `EmbarkEvent { srv: bool, taxi: bool, multicrew: bool, on_planet: bool, on_station: bool, body, body_id, system_address }`.
- `GameState` no cambia de forma (los módulos leen estos datos del propio evento).

- [ ] **Step 1: Tests:**

```rust
#[test] fn loadout_fsd_engineering_modifiers_are_parsed() { /* Item "int_hyperdrive_size5_class5", Engineering.Modifiers [{Label:"FSDOptimalMass",Value:1692.6,OriginalValue:1050.0}] */ }
#[test] fn non_numeric_modifier_value_does_not_break_loadout() { /* {Label:"WeaponMode",ValueStr:"..."} → value None, kind Loadout */ }
#[test] fn fsdjump_carries_politics_and_body_id() { /* SystemAllegiance "Federation", SystemFaction{Name,FactionState:"War"}, Powers ["Zachary Hudson"], PowerplayState "Fortified" */ }
#[test] fn touchdown_carries_position_and_body_id() { /* Latitude -12.5, Longitude 33.1, BodyID 7 */ }
#[test] fn disembark_on_planet_parsed() { /* {"event":"Disembark","SRV":true,"OnPlanet":true,"OnStation":false,"Body":"Planeta 1","BodyID":7} → srv==true, on_planet==true, on_station==false, body_id==Some(7) */ }
#[test] fn modifier_value_as_text_is_none() { /* {Label:"WeaponMode",Value:"Burst"} (Value como cadena) → value==None, kind Loadout, sin error */ }
#[test] fn existing_fixtures_still_end_in_same_state() { /* basic_session.log y real_*.log: snapshot insta sin cambios */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar; actualizar `state.rs` a las variantes con datos sin cambiar su comportamiento. **Step 4:** PASS; Checks (los snapshots `insta` de F0 no deben cambiar).
- [ ] **Step 5: Commit** `feat(model): ingeniería del Loadout, política del sistema y eventos de superficie`

### Task 6: `tmc-model` — `NetRequest`/`NetResponse`, `ModuleSettings` y nuevas `SourceEvent`

**Files:** Create `crates/tmc-model/src/net.rs`, `crates/tmc-model/src/settings.rs`; Modify `source.rs`, `lib.rs`.

**Interfaces:**
- Produces (todo `Debug + Clone + PartialEq + Serialize + Deserialize`):
  - `pub enum NetQuery { System { id64: u64, name: String }, NeutronRoute(NeutronRouteParams), GalaxyRoute(GalaxyRouteParams), Autocomplete { prefix: String }, BodyBio { system_address: u64, body_id: u32, body_name: String, profile: BodyProfile } }`.
  - `NeutronRouteParams { from: String, to: String, range_ly: f64, efficiency: u8, supercharge_multiplier: f64 }`; `GalaxyRouteParams { from, to, ship: ShipParams, refuel_scoopable: bool, use_injections: bool }`; `ShipParams { unladen_mass: f64, fuel_main_capacity: f64, fuel_reserve: f64, cargo: f64, optimal_mass: f64, max_fuel_per_jump: f64, fuel_mul: f64, fuel_power: f64, guardian_bonus_ly: f64, supercharge_multiplier: f64 }`.
  - `pub struct NetRequest { pub id: u64, pub module: String, pub key: String, pub query: NetQuery }` — `key` = clave de relevancia (p. ej. `"system:10477373803"`, `"route"`, `"bio:<addr>:<body>"`).
  - `pub struct NetResponse { pub id: u64, pub module: String, pub key: String, pub source: NetSource, pub fetched_at: DateTime<Utc>, pub outcome: NetOutcome }`; `pub enum NetSource { Spansh, Edsm, Canonn, Cache }`.
  - `pub enum NetOutcome { System(RemoteSystem), Route(PlannedRoute), Autocomplete(Vec<String>), BodyBio(RemoteBodyBio), NoData, Disabled, Failed(NetFailure) }`; `pub enum NetFailure { Timeout, Http(u16), Malformed, Down, RateLimited }`.
  - `RemoteSystem { id64, name, discovered: Option<bool>, body_count: Option<u32>, primary_star: Option<String>, allegiance, government, controlling_faction, faction_state, economy, second_economy, security, population: Option<u64>, powers: Vec<String>, powerplay_state: Option<String> }`.
  - `PlannedRoute { hops: Vec<PlannedHop>, total_ly: f64 }`; `PlannedHop { name: String, id64: Option<u64>, star_pos: Option<[f64; 3]>, distance_ly: f64, neutron: bool, refuel: bool, star_class: Option<String> }`.
  - `BodyProfile { planet_class, atmosphere_type, gravity_g: Option<f64>, temperature_k: Option<f64>, pressure_atm: Option<f64>, volcanism: Option<String>, star_class: Option<String> }`; `RemoteBodyBio { hints: Vec<RemoteSpeciesHint { genus: String, species: String, weight: f64 }> }`.
  - `settings.rs`: `pub struct ModuleSettings { pub alerts_enabled: bool, pub online_services: bool, pub auto_copy_next_system: bool, pub map_value_threshold: u64, pub default_neutron_efficiency: u8 }` con `Default` = `true, true, true, 500_000, 60`.
  - `SourceEvent` + `Net(NetResponse)`, `Action { name: String, args: serde_json::Value }`, `Settings(ModuleSettings)`, `Stored { module: String, tag: String, value: serde_json::Value }`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn net_response_roundtrips_json() { /* NetResponse{id:3, module:"navigation", key:"system:1", source:Spansh, outcome:System(RemoteSystem{id64:1,name:"Sol",..})} → from_str(to_string(x)) == x; idem con outcome Failed(NetFailure::Http(503)) */ }
#[test] fn module_settings_defaults() { let s = ModuleSettings::default(); assert_eq!(s.map_value_threshold, 500_000); assert_eq!(s.default_neutron_efficiency, 60); assert!(s.online_services && s.auto_copy_next_system && s.alerts_enabled); }
#[test] fn net_failure_http_serializes_stably() { /* to_string(&NetFailure::Http(503)) == el literal fijado en el test (p. ej. {"Http":503}) y deserializa a Http(503) */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar; añadir los brazos nuevos a los `match` exhaustivos existentes (`actor.rs` → `Changes::empty()`). **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(model): tipos de consulta de red, ajustes de módulos y nuevas fuentes de eventos`

### Task 7: `tmc-core` — peticiones de red, tareas de SQLite y migraciones de módulo

**Files:** Modify `crates/tmc-store/src/lib.rs`, `crates/tmc-core/src/module.rs`, `actor.rs`, `lib.rs`.

**Interfaces:**
- Consumes: `NetRequest`, `NetQuery`, `SourceEvent::{Net, Stored, Settings, Action}`, `Store`, `Migration`.
- Produces:
  - `tmc-store` (API pública; `lock()` sigue siendo `pub(crate)`): `Store::query_json(&self, sql: &str, params: &[serde_json::Value]) -> Result<Vec<serde_json::Value>, StoreError>` (cada fila como array JSON), `Store::exec_json(&self, sql: &str, params: &[serde_json::Value]) -> Result<usize, StoreError>`, `Store::api_cache_get(&self, key: &str) -> Result<Option<CachedEntry>, StoreError>` y `Store::api_cache_put(&self, key: &str, value: &str, fetched_at: DateTime<Utc>) -> Result<(), StoreError>` sobre la tabla `api_cache` de F0 (`CachedEntry { value: String, fetched_at: DateTime<Utc> }`). `tmc_core` re-exporta `Store`, `StoreError`, `CachedEntry` y `Migration`; así `tmc-modules` y `tmc-app` solo usan `tmc_core::*` (sin dependencia nueva de `tmc-modules` hacia `tmc-store`).
  - `Module` + `fn migrations(&self) -> &'static [Migration] { &[] }` (spec base §5.2).
  - `pub type StoreTask = Box<dyn FnOnce(&Store) -> Result<serde_json::Value, StoreError> + Send>`.
  - `ModuleCtx` + `pub fn request(&mut self, key: impl Into<String>, query: NetQuery) -> Option<u64>` (asigna `id` creciente y `module`; **devuelve `None` y descarta la petición si `replaying`**) y `pub fn store_task(&mut self, tag: impl Into<String>, task: StoreTask)` (se ejecuta también durante el replay). `ModuleCtx::new` pasa a recibir el id del módulo y el siguiente id de petición (crate-private).
  - `pub struct CoreOptions { pub writer: Option<EventWriter>, pub flush_every: Duration, pub store: Option<Store>, pub net: Option<mpsc::Sender<NetRequest>> }` y `pub fn spawn_core_with(modules: Vec<Box<dyn Module>>, opts: CoreOptions) -> CoreHandle`; `spawn_core` existente delega en ella (firma intacta).
- Reglas: el actor envía las `NetRequest` con `try_send` (cola llena → descarta con `warn!`, nunca bloquea); ejecuta cada `StoreTask` con `spawn_blocking` y reinyecta el resultado como `SourceEvent::Stored { module, tag, value }` por un **canal interno propio** (no un clon de `events`, para que el actor siga terminando cuando se cierran las entradas); un `Err` se registra y no se reinyecta; sin `store` las tareas se descartan con `debug!`. `Stored`/`Net` se entregan a todos los módulos (cada uno filtra por `module`).

- [ ] **Step 1: Tests** (en `actor.rs`, módulos falsos):

```rust
#[tokio::test] async fn module_request_reaches_net_channel_with_module_and_id() { /* request("system:1", System{id64:1,name:"Sol"}) → net_rx recibe NetRequest{id:1, module:"fake", key:"system:1"}; segunda petición id==2 */ }
#[tokio::test] async fn requests_during_replay_are_dropped() { /* Journal replaying:true → request devuelve None y net_rx.try_recv() es Err(Empty) */ }
#[tokio::test] async fn store_task_result_returns_as_stored_event() { /* tarea SELECT 42 → módulo recibe Stored{module:"fake", tag:"t", value:42} */ }
#[tokio::test] async fn full_net_channel_never_blocks_actor() { /* canal capacidad 1, 50 peticiones → el snapshot responde < 100 ms */ }
#[tokio::test] async fn actor_still_stops_when_inputs_close_with_store() { /* Store en memoria + StoreTask en vuelo; cerrar entradas → join del actor termina en < 1 s */ }
#[test] fn module_migrations_default_empty() { assert!(FakeModule.migrations().is_empty()); }
#[test] fn store_query_json_returns_rows() { /* tabla con (1,'a'),(2,'b') → [[1,"a"],[2,"b"]]; params [2] en WHERE id=?1 → [[2,"b"]] */ }
#[test] fn api_cache_put_then_get_roundtrips() { /* put("k","{\"a\":1}",t) → get("k")==Some(CachedEntry{value:"{\"a\":1}", fetched_at:t}); get("zz")==None */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-core` PASS (incluidos los tests de F0); Checks.
- [ ] **Step 5: Commit** `feat(core): API pública de SQL, peticiones de red y tareas de SQLite fuera del actor`

### Task 8: i18n — varios `.ftl` por idioma

**Files:** Modify `crates/tmc-i18n/src/lib.rs`, `ui/src/lib/i18n.ts`, `ui/tests/i18n.test.ts`; Create `locales/{es,en,fr,de,pt}/{navigation,exploration,exobiology}.ftl` (con una clave inicial `panel-route-strip`, `panel-system-bodies`, `panel-exobiology` respectivamente).

**Interfaces:**
- Produces: `I18n::new` y `keys(lang)` cargan **todos** los `*.ftl` de `locales/<lang>/` (orden alfabético; clave duplicada entre archivos → error en test). UI: `import.meta.glob('../../../locales/*/*.ftl', …)`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn keys_include_all_ftl_files() { assert!(keys("es").contains("panel-route-strip")); }
#[test] fn no_key_is_defined_in_two_files() { /* dir temporal con a.ftl y b.ftl que definen "foo = 1" → carga devuelve Err cuyo mensaje contiene "foo"; los locales reales cargan Ok */ }
```
```ts
test('loads keys from every ftl file', ...)   // t('panel-system-bodies') !== 'panel-system-bodies'
test('all locales define the same keys', ...) // ahora sobre *.ftl
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks (Rust + UI).
- [ ] **Step 5: Commit** `feat(i18n): varios archivos Fluent por idioma`

### Task 9: `tmc-net` — limitador, salud del servicio y política de caché

**Files:** Create `crates/tmc-net/{Cargo.toml, src/lib.rs, src/limiter.rs, src/health.rs, src/cache_policy.rs}`; Modify `Cargo.toml` (workspace).

**Interfaces:**
- Consumes: `NetQuery`.
- Produces (todo con reloj inyectado `Instant`/`DateTime<Utc>` para tests deterministas):
  - `pub enum Service { Spansh, Edsm, Canonn }`.
  - `pub struct RateLimiter` — `RateLimiter::per_window(max: u32, window: Duration)`; `fn try_acquire(&mut self, now: Instant) -> Result<(), Duration /*espera*/>`. Fábrica `RateLimiter::for_service(Service)` → Spansh 1/1 s, Canonn 1/1 s, EDSM 10/10 s.
  - `pub struct ServiceHealth` — `fn record_success(&mut self)`, `fn record_failure(&mut self, now: Instant)`, `fn retry_at(&self) -> Option<Instant>` (30 s, 2 min, 10 min tras 1.º/2.º/3.º fallo; **el consumidor es `NetService`, Task 11**, que reprograma la petición fallida), `fn is_down(&self, now: Instant) -> bool` (3 fallos seguidos → caído 10 min).
  - `pub struct TtlPolicy { pub fresh: Duration, pub max_stale: Duration }`; `pub enum CacheKind { SystemCard, SystemPolitics, BodyBio }`; `pub fn ttl_for(kind: CacheKind) -> TtlPolicy`: `SystemCard` (ficha) fresh 24 h / max_stale 24 h; `SystemPolitics` fresh **1 h** / max_stale 24 h (servida como caché con antigüedad y refrescada en segundo plano); `BodyBio` 30 días / 30 días (spec §6: ficha y política caducan por separado).
  - `pub fn cache_keys(q: &NetQuery) -> Vec<(CacheKind, String)>`: `System` → dos claves `"spansh:system:<id64>:card"` y `"spansh:system:<id64>:politics"`; `BodyBio` → `"canonn:body:<addr>:<id>"`; rutas y `Autocomplete` → vacío (sin caché).

- [ ] **Step 1: Tests:**

```rust
#[test] fn spansh_limiter_allows_one_per_second() { /* t0 Ok; t0+500 ms → Err(≈500 ms ±10); t0+1 s Ok */ }
#[test] fn edsm_limiter_allows_ten_per_ten_seconds() { /* 10 OK, el 11.º espera ≈ hasta el fin de la ventana */ }
#[test] fn backoff_sequence_is_30s_2m_10m() { /* record_failure(t0) → retry_at==t0+30 s; 2.º → t0+120 s; 3.º → t0+600 s */ }
#[test] fn three_consecutive_failures_mark_down_for_10_minutes() { /* is_down(t+9m59s) true, (t+10m01s) false */ }
#[test] fn success_resets_failure_count() { /* 2 fallos, record_success → retry_at None y is_down false; un fallo después → retry_at == t+30 s */ }
#[test] fn system_card_ttl_24h_and_politics_1h() { /* ttl_for(SystemCard).fresh==24 h; ttl_for(SystemPolitics).fresh==1 h y max_stale==24 h; BodyBio 30 días */ }
#[test] fn routes_and_autocomplete_have_no_cache_keys() { assert!(cache_keys(&NetQuery::NeutronRoute(..)).is_empty()); /* idem Autocomplete; System{id64:5} → [(SystemCard,"spansh:system:5:card"),(SystemPolitics,"spansh:system:5:politics")] */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar (sin dependencias de red todavía). **Step 4:** `cargo test -p tmc-net` PASS; Checks; `cargo metadata` confirma `tmc-net → tmc-model` solo.
- [ ] **Step 5: Commit** `feat(net): limitador por servicio, salud con retroceso y política de caché`

### Task 10: `tmc-net` — adaptadores Spansh/EDSM/Canonn y `NetClient`

**Files:** Create `crates/tmc-net/src/{transport.rs, spansh.rs, edsm.rs, canonn.rs, client.rs}`; Modify `Cargo.toml` (`reqwest` con rustls), `THIRD_PARTY_NOTICES.md`.

**Interfaces:**
- Consumes: Tasks 0, 2 (URL y formas grabadas), Task 9.
- Produces:
  - `pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>`; `pub struct HttpReply { pub status: u16, pub body: String }`; `pub enum TransportError { Timeout, Connect(String) }`; `pub trait Transport: Send + Sync { fn get(&self, url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>>; fn post_form(&self, url: &str, form: &[(String, String)]) -> BoxFut<'_, Result<HttpReply, TransportError>>; }`; `pub struct ReqwestTransport` (timeout 5 s, `User-Agent: TMC/<CARGO_PKG_VERSION>`).
  - Adaptadores puros (sin red; **la forma interna es fija y el adaptador se ajusta al resultado de los spikes**): `spansh::system_url(id64) -> String`, `spansh::parse_system(body: &str) -> Result<Option<RemoteSystem>, NetFailure>` (`Ok(None)` = `NoData`), `spansh::neutron_form(&NeutronRouteParams) -> Vec<(String, String)>`, `spansh::galaxy_form(&GalaxyRouteParams) -> Vec<(String, String)>`, `spansh::parse_job(body) -> Result<String, NetFailure>`, `spansh::parse_route_result(body) -> Result<RoutePoll, NetFailure>` (`enum RoutePoll { Pending, Done(PlannedRoute), Failed(String) }`), `spansh::parse_autocomplete(body) -> Result<Vec<String>, NetFailure>`; `edsm::system_url(name) -> String`, `edsm::parse_system(body) -> Result<Option<RemoteSystem>, NetFailure>` (`{}` y `[]` → `Ok(None)` = `NoData`; cadena vacía, JSON truncado o tipos cambiados → `Err(Malformed)`); `canonn::body_url(..)`, `canonn::parse_body(body) -> Result<RemoteBodyBio, NetFailure>`.
  - `pub struct NetClient` — `NetClient::new(transport: Arc<dyn Transport>) -> NetClient`; `pub async fn execute(&self, q: &NetQuery) -> (NetSource, NetOutcome)`: aplica limitador (espera asíncrona), salud por servicio, alternativa Spansh→EDSM para `System`, sondeo de rutas cada 2 s hasta 120 s (una ruta a la vez, `Mutex` asíncrono), 404 → `NoData`, 429 → `RateLimited` + retroceso, JSON inválido → `Failed(Malformed)`.

- [ ] **Step 1: Tests** (con `FakeTransport` que sirve `tests/fixtures/net/*.json`):

```rust
#[test] fn parses_recorded_spansh_system() { /* valores concretos del fixture del spike: nombre, allegiance, body_count */ }
#[test] fn parses_recorded_neutron_route() { /* nº de saltos y total_ly del fixture; al menos un hop con neutron=true */ }
#[test] fn malformed_or_truncated_bodies_fail_cleanly() { /* "", "{", "null", JSON con tipos cambiados → Err(Malformed) sin panic (Spansh y EDSM) */ }
#[test] fn edsm_empty_object_or_array_is_no_data() { assert_eq!(edsm::parse_system("{}"), Ok(None)); assert_eq!(edsm::parse_system("[]"), Ok(None)); assert_eq!(edsm::parse_system(""), Err(NetFailure::Malformed)); }
#[tokio::test] async fn spansh_404_is_no_data() { /* FakeTransport 404 → execute == (NetSource::Spansh, NetOutcome::NoData) y la salud de Spansh no cuenta fallo */ }
#[tokio::test] async fn three_failures_mark_spansh_down_and_use_edsm() { /* Spansh 503 ×3 → 4.ª consulta System va a EDSM, source Edsm */ }
#[tokio::test] async fn route_job_is_polled_until_done() { /* job → Pending ×2 → Done; el transporte recibe exactamente 3 sondeos */ }
#[tokio::test] async fn timeout_maps_to_failed_timeout() { /* FakeTransport devuelve Err(Timeout) → outcome == Failed(NetFailure::Timeout) */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks. **Licencias (C19):** listar el árbol (`cargo tree -p tmc-net -e normal` y `cargo deny check licenses` si está disponible, si no `cargo metadata` + revisión manual); las transitivas permisivas (MIT/Apache/BSD/ISC/Zlib/MPL con aviso) se anotan en `THIRD_PARTY_NOTICES.md`; cualquier GPL/AGPL/CC-BY(-SA) o licencia no reconocida **detiene la tarea** y se pregunta al propietario. Anotar `reqwest`, `rustls` y transitivas relevantes en `THIRD_PARTY_NOTICES.md`.
- [ ] **Step 5: Commit** `feat(net): clientes de Spansh, EDSM y Canonn con degradación`

### Task 11: `tmc-app` — `NetService`, configuración nueva y ajustes en caliente

**Files:** Create `crates/tmc-app/src/net_service.rs`; Modify `config.rs`, `clipboard.rs`, `wiring.rs`, `lib.rs`, `crates/tmc-modules/src/dashboard/mod.rs`, `crates/tmc-modules/src/registry.rs`.

**Interfaces:**
- Consumes: `NetClient`, `ttl_for`, `cache_keys`, `CacheKind`, `ServiceHealth::retry_at`, `Store::{api_cache_get, api_cache_put}` (Task 7), `CoreOptions`, `ModuleSettings`.
- Produces:
  - `Config` + `online_services: bool` (true), `map_value_threshold: u64` (500 000), `default_neutron_efficiency: u8` (60); `impl Config { pub fn module_settings(&self) -> ModuleSettings }` (`auto_copy_next_system` = el `Config.auto_copy` existente: no se añade un campo duplicado). Archivos `config.toml` de F0 sin estos campos cargan con los valores por defecto.
  - `#[derive(Clone)] pub struct NetServiceHandle { pub tx: mpsc::Sender<NetRequest>, client: Arc<NetClient>, online: Arc<AtomicBool> }` con `pub async fn autocomplete(&self, prefix: &str) -> Result<Vec<String>, NetFailure>` (para REST, sin pasar por el actor; mínimo 3 caracteres; con `online == false` devuelve `Ok(vec![])`); `CoreOptions.net` recibe `handle.tx.clone()`.
  - `pub fn spawn_net_service(client: Arc<NetClient>, store: Store, events: mpsc::Sender<SourceEvent>, online: Arc<AtomicBool>) -> NetServiceHandle`: con `online == false` responde `Disabled` sin tocar el transporte; consulta `api_cache` (`spawn_blocking`) por cada `CacheKind`: ficha fresca 24 h y política fresca 1 h → responde `source: Cache`; cualquiera caducada pero dentro de `max_stale` → responde caché y refresca en segundo plano (una sola vez); agrupa peticiones idénticas en curso por `cache_keys`/`key` (una sola llamada, N respuestas con sus `id`); guarda toda respuesta `System` correcta en `api_cache` partida en ficha (`discovered`, `body_count`, `primary_star`) y política (el resto) con su `fetched_at`; **reintentos (C4):** ante `Failed(..)` consulta `ServiceHealth::retry_at` y reprograma la misma petición (mismo `id`) con `tokio::time::sleep_until`, hasta que haya respuesta correcta o el servicio caiga; nunca bloquea al actor.  - `wiring::start`: `all_modules()` **siempre** (F0 desactivaba todos los módulos con `alerts_enabled = false`; ahora `Dashboard` lo respeta vía `SourceEvent::Settings`; se actualizan los tests de F0 que suponían «sin módulos»), aplica `migrations()` de cada módulo antes de `spawn_core_with`, envía `SourceEvent::Settings(config.module_settings())` al arrancar.
  - `AppActions::put_config` envía `SourceEvent::Settings` al núcleo y actualiza `online` (`AppActions` gana `with_runtime(config, events: mpsc::Sender<SourceEvent>, online: Arc<AtomicBool>, net: NetServiceHandle)`; `AppActions::autocomplete` delega en `net.autocomplete` salvo galaxia Legacy u `online == false` → `{items:[],page:0,more:false}`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn old_config_without_f1_fields_loads_defaults() { /* TOML de F0 con solo campos antiguos → online_services==true, map_value_threshold==500_000, default_neutron_efficiency==60; auto_copy conserva su valor y module_settings().auto_copy_next_system==auto_copy */ }
#[tokio::test] async fn disabled_online_services_never_reach_transport() { /* FakeTransport cuenta 0 llamadas; respuesta outcome==Disabled */ }
#[tokio::test] async fn fresh_cache_hit_skips_transport() { /* ficha y política de hace 10 min → 0 llamadas; source==Cache; outcome==System(valor cacheado) */ }
#[tokio::test] async fn stale_cache_is_served_and_refreshed_once() { /* ficha de hace 25 h → respuesta Cache inmediata + exactamente 1 llamada de refresco; segunda petición posterior → 0 llamadas */ }
#[tokio::test] async fn politics_expires_after_1h_while_card_lasts_24h() { /* ficha y política de hace 2 h → Cache + 1 refresco; ficha de hace 2 h y política de hace 10 min → 0 llamadas */ }
#[tokio::test] async fn failed_request_is_retried_after_backoff() { /* tokio::time::pause; transporte 503 y luego OK → primera NetResponse Failed(Http(503)); avanzar 30 s → 2.º intento; llega NetResponse System con el mismo id de petición; llamadas==2 */ }
#[tokio::test] async fn identical_inflight_requests_are_coalesced() { /* 3 NetRequest con mismo System id64 → 1 llamada, 3 NetResponse con ids 1,2,3 */ }
#[tokio::test] async fn put_config_propagates_settings_to_core() { /* online_services=false → módulo falso recibe Settings{online_services:false} y el AtomicBool queda false */ }
#[test] fn dashboard_emits_no_alerts_when_disabled_by_settings() { /* Settings{alerts_enabled:false} + evento que genera alerta de combustible bajo → 0 alertas; con true → 1 alerta */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-app -p tmc-modules` PASS; Checks.
- [ ] **Step 5: Commit** `feat(app): servicio de red con caché, opt-out y ajustes en caliente`

### Task 12: `tmc-server` — acciones con argumentos, despacho a módulos y autocompletado

**Files:** Modify `crates/tmc-server/src/{lib.rs, rest.rs, ws.rs}`, `crates/tmc-app/src/clipboard.rs`, `ui/src/lib/ws.svelte.ts`, `ui/tests/ws.test.ts`.

**Interfaces:**
- Produces:
  - `ActionHost` + `fn run_action_with(&self, name: &str, args: &Value, snapshot: &BTreeMap<String, Value>) -> Result<Value, ActionError> { self.run_action(name, snapshot) }` y `fn autocomplete(&self, _q: String, _snapshot: &BTreeMap<String, Value>) -> ActionFut` (por defecto `NotFound`); `pub type ActionFut = Pin<Box<dyn Future<Output = Result<Value, ActionError>> + Send + 'static>>` **definido en `tmc-server`** (no se usa `tmc_net::BoxFut`: `tmc-server` no depende de `tmc-net`).
  - `POST /api/actions/{name}` acepta cuerpo JSON opcional (`args`, vacío → `null`); WS `ClientMessage::Action { name, args: Option<Value> }`.
  - `GET /api/navigation/autocomplete?q=<texto>&page=<n>` → `{ items: string[], page, more: bool }` (20 por página; con galaxia Legacy u online desactivado el host devuelve `[]` → `{items:[],page:0,more:false}`).
  - `AppActions::run_action_with`: nombres con prefijo de un módulo registrado (`navigation.`, `exploration.`, `exobiology.`) → `SourceEvent::Action { name, args }` al núcleo con `try_send`, respuesta `{ "accepted": true }` (el resultado llega por topics); cola llena → `Failed("busy")`.
  - UI: `export async function action(name: string, args?: unknown): Promise<unknown>`.

- [ ] **Step 1: Tests:**

```rust
#[tokio::test] async fn action_body_args_reach_host() { /* POST {"to":"Colonia"} → host recibe args["to"]=="Colonia" */ }
#[tokio::test] async fn ws_action_with_args_reaches_host() { /* WS {"type":"action","name":"navigation.plan","args":{"to":"Colonia"}} → host falso registra name=="navigation.plan" y args["to"]=="Colonia" */ }
#[tokio::test] async fn autocomplete_route_paginates() { /* host falso con 45 nombres → page 0: 20 items, more true; page 2: 5 items, more false */ }
#[tokio::test] async fn autocomplete_returns_empty_for_legacy_or_offline() { /* host con online=false → {items:[],page:0,more:false}; idem galaxia Legacy en el snapshot; 0 llamadas al transporte */ }
#[test] fn module_action_is_forwarded_as_source_event() { /* run_action_with("navigation.plan", {"to":"X"}) → canal recibe SourceEvent::Action{name:"navigation.plan", args:{"to":"X"}} y respuesta {"accepted":true}; cola llena → Err(Failed("busy")) */ }
```
```ts
test('action posts args as json body', ...)   // fetch simulado recibe body '{"to":"Colonia"}' y content-type JSON
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar (los tests de F0 de acciones sin cuerpo siguen pasando). **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(server): acciones con argumentos hacia módulos y autocompletado paginado`

---

## Bloque 2 — Navegación

### Task 13: `navigation` — alcance de salto y combustible

**Files:** Create `crates/tmc-modules/data/fsd.json`, `crates/tmc-modules/src/data.rs`, `crates/tmc-modules/src/navigation/{mod.rs, jump.rs}` (`mod.rs` solo declara `pub mod jump;`; el struct `Navigation` lo define Task 15; `jump.rs` es la única lógica de esta tarea); Modify `crates/tmc-modules/src/lib.rs`, `THIRD_PARTY_NOTICES.md` (si Task 1 lo exige).

**Interfaces:**
- Consumes: `Loadout`, `LoadoutModule`, `Engineering`, `Modifier`, `Status.fuel`, `Cargo`, `JetConeBoost`.
- Produces:
  - `data.rs`: `pub struct StarFlags { pub scoopable: bool, pub special: Option<StarSpecial> }`, `pub enum StarSpecial { Neutron, WhiteDwarf }`, `pub fn star_flags(class: &str) -> StarFlags` (KGBFOAM → scoopable; `N` → Neutron; `D*` → WhiteDwarf; única fuente para Tasks 15 y 21).
  - `fsd.json`: cabecera `{ "source": "...", "license": "...", "version": "2026-10-17" }` + entradas `{ "item_prefix": "int_hyperdrive_size5_class5", "optimal_mass": .., "max_fuel_per_jump": .., "fuel_mul": .., "fuel_power": .., "kind": "standard|sco|sco_mk2" }` y booster `{ "item_prefix": "int_guardianfsdbooster_size5", "bonus_ly": 10.5 }`. Cargado una vez (`OnceLock`) por `data::fsd_table() -> &'static FsdTable`.
  - `pub struct FsdSpec { pub optimal_mass: f64, pub max_fuel_per_jump: f64, pub fuel_mul: f64, pub fuel_power: f64, pub guardian_bonus_ly: f64, pub kind: FsdKind }`; `pub enum FsdKind { Standard, Sco, ScoMk2 }`.
  - `pub struct ShipMass { pub unladen: f64, pub fuel_main: f64, pub fuel_reserve: f64, pub cargo: f64 }` con `fn total(&self) -> f64`.
  - `pub fn fsd_from_loadout(lo: &Loadout, table: &FsdTable) -> Option<FsdSpec>` (aplica `FSDOptimalMass`/`MaxFuelPerJump` de `Engineering.Modifiers`; modificadores desconocidos se ignoran).
  - `pub fn jump_range(fsd: &FsdSpec, mass: &ShipMass) -> f64` (= `jump_range_with(.., true)`), `pub fn jump_range_with(fsd: &FsdSpec, mass: &ShipMass, clamp_to_max_fuel: bool) -> f64` (sin recorte usa todo `fuel_main`; lo usa `eval`, Task 29), `pub fn fuel_for_jump(fsd: &FsdSpec, total_mass: f64, distance_ly: f64) -> Option<f64>` (`None` = no realizable).
  - `pub fn supercharge_multiplier(kind: FsdKind, star: SuperchargeStar) -> f64` (`Neutron`: 4 / Mk II 6; `WhiteDwarf`: 1,5 / Mk II 3).
  - `pub enum RangeSource { Formula, Scaled, Unknown }`; `pub struct JumpModel { pub source: RangeSource, pub fsd: Option<FsdSpec>, pub game_max_range: Option<f64>, pub laden_ref_mass: Option<f64> }`; `pub fn build_jump_model(lo: &Loadout, table: &FsdTable) -> JumpModel`: valida `jump_range` en «sin carga, depósito lleno» contra `Loadout.MaxJumpRange` (±0,05 al) → `Formula`; si no coincide o no hay tabla → `Scaled` (`warn!` una vez por `ShipID`); sin FSD ni `MaxJumpRange` → `Unknown`. `impl JumpModel { pub fn range_now(&self, mass: &ShipMass) -> Option<f64> }` (`Scaled`: `MaxJumpRange · masa_ref / masa_actual`).
- Algoritmo (no lo determina la firma):

```rust
pub fn jump_range(fsd: &FsdSpec, m: &ShipMass) -> f64 {
    let fuel = m.fuel_main.min(fsd.max_fuel_per_jump);
    if fuel <= 0.0 || m.total() <= 0.0 { return 0.0; }
    (fuel / fsd.fuel_mul).powf(1.0 / fsd.fuel_power) * fsd.optimal_mass / m.total() + fsd.guardian_bonus_ly
}
pub fn fuel_for_jump(fsd: &FsdSpec, total_mass: f64, d: f64) -> Option<f64> {
    let base = (d - fsd.guardian_bonus_ly).max(0.0); // verificar en eval (Task 29) si el booster entra así
    let f = fsd.fuel_mul * (base * total_mass / fsd.optimal_mass).powf(fsd.fuel_power);
    (f <= fsd.max_fuel_per_jump + 1e-9).then_some(f)
}
```

- [ ] **Step 1: Tests** (`FsdSpec` de prueba: masa óptima 1050 t, máx. 5 t, `fuel_mul` 0,012, `fuel_power` 2,45):

```rust
#[test] fn range_matches_reference_value() { /* total 500 t, combustible ≥ 5 → 24.63 ± 0.01 */ }
#[test] fn guardian_booster_adds_flat_bonus() { /* +10.5 → 35.13 ± 0.01 */ }
#[test] fn fuel_for_max_jump_equals_max_fuel() { /* fuel_for_jump(.., 500.0, 24.63) == Some(5.0 ± 0.01) */ }
#[test] fn jump_beyond_range_is_not_feasible() { /* 30 al → None */ }
#[test] fn engineered_optimal_mass_is_applied() { /* FSDOptimalMass 1692.6 → optimal_mass 1692.6 */ }
#[test] fn supercharge_multipliers() { /* Standard N 4.0, WD 1.5; ScoMk2 N 6.0, WD 3.0 */ }
#[test] fn formula_validated_against_game_max_jump_range() { /* Loadout sintético coherente → RangeSource::Formula */ }
#[test] fn unknown_fsd_item_falls_back_to_scaled_max_jump_range() { /* Item "int_hyperdrive_size9_class9_future", MaxJumpRange 40 → Scaled; range_now con +100 t de carga < 40 */ }
#[test] fn unknown_modifier_is_ignored_and_validation_decides() { /* Loadout con Modifier{label:"FSDFancyNew",value:1.0} → fsd_from_loadout().optimal_mass == 1050.0 igual que sin el modificador */ }
#[test] fn unclamped_range_exceeds_clamped_when_tank_above_max_fuel() { /* fuel_main 32 t: jump_range_with(.., true) == 24.63 ± 0.01 (recortado a 5 t) y jump_range_with(.., false) > 24.63 + 1.0 */ }
#[test] fn star_flags_classes() { assert!(star_flags("K").scoopable); assert_eq!(star_flags("N").special, Some(StarSpecial::Neutron)); assert_eq!(star_flags("DA").special, Some(StarSpecial::WhiteDwarf)); assert!(!star_flags("DA").scoopable); }
#[test] fn loadout_without_fsd_hides_fuel_after_jump() { /* sin módulo FSD ni MaxJumpRange → Unknown; range_now None */ }
#[test] fn fsd_table_loads_and_has_sco_and_mk2_entries() { /* tabla con >= 1 entrada kind Sco y >= 1 ScoMk2; todas con optimal_mass > 0 y max_fuel_per_jump > 0 */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** `cargo test -p tmc-modules` PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): alcance de salto y combustible con tabla FSD e ingeniería`

### Task 14: `navigation` — ruta activa y reglas de avance

**Files:** Create `crates/tmc-modules/src/navigation/route.rs`; Modify `crates/tmc-modules/src/navigation/mod.rs` (añade `pub mod route;`).

**Interfaces:**
- Consumes: `PlannedRoute`, `PlannedHop`, `RouteState` (NavRoute del juego), `LocationInfo`.
- Produces:
  - `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct ActivePlan { pub created_at: DateTime<Utc>, pub origin: String, pub destination: String, pub mode: PlanMode, pub params: serde_json::Value, pub hops: Vec<PlannedHop>, pub current_index: usize, #[serde(skip)] pub off_route: bool }`; `#[derive(.., Serialize, Deserialize)] pub enum PlanMode { Neutron, Galaxy }`. `off_route` no se persiste: se recalcula con `reconcile`.
  - `impl ActivePlan { pub fn on_jump(&mut self, system: &str, id64: u64) -> JumpOutcome; pub fn next(&self) -> Option<&PlannedHop>; pub fn upcoming(&self, n: usize) -> &[PlannedHop]; pub fn remaining_ly(&self) -> f64; pub fn arrived(&self) -> bool; pub fn reconcile(&mut self, system: &str, id64: Option<u64>) -> JumpOutcome /* igual que on_jump con el sistema actual, sin efecto si ya es el hop actual */ }`; `pub enum JumpOutcome { Advanced { skipped: usize }, OffRoute, Arrived }`. Comparación por `id64` si existe, si no por nombre sin distinguir mayúsculas.
  - `pub enum RouteSource { Plan, NavRoute, Current }`; `pub fn pick_source(plan: Option<&ActivePlan>, game: &RouteState) -> RouteSource` (plan activo manda; `NavRoute.json` solo sin plan; sin ninguno, sistema actual).

- [ ] **Step 1: Tests:**

```rust
#[test] fn jump_to_next_hop_advances_index() { /* plan A,B,C; index 0; on_jump("B", idB) → Advanced{skipped:0}, current_index==1, next()==C */ }
#[test] fn jump_skipping_hops_advances_to_reached_system() { /* plan A,B,C,D; salto a C → index en C, skipped 1 */ }
#[test] fn jump_outside_plan_marks_off_route_and_keeps_plan() { /* off_route true; hops intactos (len igual); next() sigue siendo el de antes */ }
#[test] fn returning_to_plan_clears_off_route() { /* tras OffRoute, on_jump a un hop del plan → off_route==false */ }
#[test] fn reaching_destination_reports_arrived() { /* on_jump al último hop → JumpOutcome::Arrived y arrived()==true */ }
#[test] fn plan_has_priority_over_game_navroute() { /* NavRoute distinta → RouteSource::Plan */ }
#[test] fn navroute_used_only_without_plan_and_current_as_last_resort() { /* sin plan y NavRoute con 3 saltos → NavRoute; sin ambos → Current */ }
#[test] fn active_plan_serde_roundtrip_resets_off_route() { /* plan con off_route=true → JSON → deserializado == original salvo off_route==false */ }
#[test] fn reconcile_after_restore_sets_index_or_off_route() { /* plan restaurado en index 0 con sistema actual == hop 2 → current_index==2; sistema ajeno → off_route==true */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): ruta activa con avance, saltos omitidos y fuera de ruta`

### Task 15: `navigation` — tira de ruta (ficha local + remota)

**Files:** Create `crates/tmc-modules/src/navigation/strip.rs`; Modify `navigation/mod.rs` (aquí se define `pub struct Navigation`), `registry.rs`; Create `locales/*/navigation.ftl` (claves de la tira).

**Interfaces:**
- Consumes: Tasks 13–14, `ModuleCtx::{request, store_task}`, `SourceEvent::{Net, Stored, Settings}`, `GameState.{location, route, ship, galaxy}`.
- Produces:
  - `pub struct Navigation` (`impl Module`, id `"navigation"`); registro: `Box::new(Navigation::new())` en `all_modules()`.
  - `RemoteState { status: RemoteStatus, source: Option<NetSource>, fetched_at: Option<DateTime<Utc>>, data: Option<RemoteSystem> }`; `enum RemoteStatus { Pending, Ok, NoData, Disabled, Failed }` (serializa kebab-case).
  - `SystemCard { name, id64: Option<u64>, jump_ly: Option<f64>, star_class: Option<String>, scoopable: Option<bool>, special: Option<StarSpecial /*neutron|white_dwarf*/>, fuel_after_pct: Option<f64>, reachable: Option<bool>, visits: Option<u32>, remote: RemoteState, local_politics: Option<LocalPolitics> }` (`scoopable`/`special` vía `data::star_flags`, Task 13; `local_politics` solo para el sistema actual, desde `FSDJump`/`Location`).
  - Topics: `navigation.route` = `{ source, current: SystemCard, next: Option<SystemCard>, following: Vec<SystemCard> /*≤3*/, remaining_jumps: Option<u32>, total_ly: Option<f64>, off_route: bool, copy_target: Option<String>, copy_seq: u64 }`; `navigation.jump` = `{ range_ly, range_full_ly, source: RangeSource, fuel_main, fuel_capacity, max_fuel_per_jump, supercharged: Option<f64> }` (se recalcula con cada `Loadout`, `Status` (combustible) y `Cargo`; `supercharged` = `BoostValue` del último `JetConeBoost`, a `None` tras el siguiente `FSDJump`).
  - Reglas: tras cada cambio de tira pide `NetQuery::System` para actual + siguiente + 3 (solo si `settings.online_services`, `galaxy != Legacy` y no hay petición en curso para esa clave); aplica `NetResponse` solo si su `key` sigue en la tira y `fetched_at` ≥ el aplicado; con `Disabled`/`Failed` marca la tarjeta sin bloquear lo local. Visitas: al primer evento, `store_task("visits")` con `SELECT json_extract(json,'$.SystemAddress'), COUNT(*) FROM event WHERE name='FSDJump' AND cmdr=?1 AND timestamp<=?2 GROUP BY 1` (`?2` = corte = timestamp del último evento visto al emitir la tarea) → mapa en memoria; los `FSDJump` en vivo con timestamp > corte se acumulan en un mapa pendiente que se suma al llegar `Stored` (sin doble conteo). Primer `Status` no-replay → refresco de peticiones pendientes (las del replay se descartaron).
  - Relevancia: `Active` con plan activo o `NavRoute` no vacía y en supercrucero/salto; si no, `Normal`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn strip_shows_next_and_three_following_from_navroute() { /* NavRoute de 5 sistemas → next.name==2.º, following.len()==3 con nombres 3.º,4.º,5.º; remaining_jumps==4 */ }
#[test] fn strip_advances_after_fsdjump() { /* tras FSDJump al 2.º → current.name==2.º, next.name==3.º */ }
#[test] fn strip_without_route_shows_current_system_card() { /* sin plan ni NavRoute → next None, following vacío, current.name==sistema actual */ }
#[test] fn fuel_after_jump_uses_jump_model_and_hides_when_unknown() { /* modelo Formula, depósito 16 t, salto de 10 ly → fuel_after_pct==valor calculado en el test ±0.1; JumpModel Unknown → None */ }
#[test] fn scoopable_and_special_star_flags() { /* "K"→scoopable; "N"→neutron; "DA"→white_dwarf, no scoopable (vía data::star_flags) */ }
#[test] fn requests_remote_for_current_next_and_three() { /* 5 peticiones con keys "system:<id64>" distintas, ids 1..5 */ }
#[test] fn late_response_for_system_no_longer_in_strip_is_not_applied() { /* respuesta para key fuera de la tira → remote de las tarjetas visibles sin cambios (status Pending) */ }
#[test] fn older_response_does_not_overwrite_newer() { /* respuesta con fetched_at t2 aplicada; luego otra con t1<t2 → data y fetched_at siguen en t2 */ }
#[test] fn strip_shows_local_card_and_no_data_when_services_disabled() { /* Settings online_services=false → 0 peticiones, remote.status "disabled", campos locales (jump_ly, star_class) presentes */ }
#[test] fn legacy_galaxy_never_requests() { /* galaxy Legacy → 0 peticiones aunque online_services=true */ }
#[test] fn visits_loaded_from_history_and_incremented_live() { /* Stored visits {A:2}; FSDJump en vivo a A → visits==3 */ }
#[test] fn visits_do_not_double_count_jumps_during_query() { /* FSDJump con timestamp > corte llega antes de Stored{A:2} → visits==3; con timestamp <= corte → 2 */ }
#[test] fn jump_topic_recomputes_on_loadout_status_and_cargo() { /* Loadout sintético (FSD de prueba): range_ly==jump_range calculado en el test ±0.01; Status con menos combustible → range_ly menor; Cargo de 40 t → range_ly menor aún; Unknown → topic con range_ly None */ }
#[test] fn jet_cone_boost_sets_supercharged_until_next_jump() { /* JetConeBoost{BoostValue:4.0} → supercharged==Some(4.0); tras FSDJump → None */ }
#[test] fn route_topic_serializes_under_8_kib() { assert!(serde_json::to_vec(&topic).unwrap().len() < 8192); }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): tira de ruta con ficha local y remota correlacionada`

### Task 16: `navigation` — planificador Spansh y persistencia `nav_route_plan`

**Files:** Create `crates/tmc-modules/src/navigation/{planner.rs, persist.rs}`; Modify `navigation/mod.rs`, `locales/*/navigation.ftl`.

**Interfaces:**
- Consumes: `SourceEvent::Action`, `NetQuery::{NeutronRoute, GalaxyRoute}`, `JumpModel`, `ModuleCtx::store_task`.
- Produces:
  - Migración `Migration { id: "navigation-001", sql: "CREATE TABLE nav_route_plan (cmdr TEXT PRIMARY KEY, created_at TEXT NOT NULL, origin TEXT NOT NULL, destination TEXT NOT NULL, mode TEXT NOT NULL, params TEXT NOT NULL, hops TEXT NOT NULL, current_index INTEGER NOT NULL);" }` en `Navigation::migrations()`.
  - `persist::save_plan(cmdr: String, plan: ActivePlan) -> StoreTask`, `persist::load_plan(cmdr: String) -> StoreTask`, `persist::delete_plan(cmdr: String) -> StoreTask`. El plan se serializa con los derives de `ActivePlan` (Task 14; `off_route` no se guarda). Al llegar `Stored{tag:"plan"}` tras reproducir `FSDJump`, `Navigation` aplica `ActivePlan::reconcile` con el sistema actual (índice y `off_route` correctos aunque el plan llegue tarde).
  - Acciones: `navigation.plan` (`args: { to: String, from?: String, mode: "neutron"|"galaxy", efficiency?: u8, use_ship?: bool, refuel_scoopable?: bool, use_injections?: bool }`), `navigation.accept-plan`, `navigation.clear-plan`, `navigation.recalculate` (desde el sistema actual con los parámetros del plan).
  - Topic `navigation.plan` = `{ status: "idle"|"computing"|"ready"|"active"|"failed"|"disabled", request: Option<Value>, result: Option<PlannedRoute>, active: Option<{ destination, current_index, total, off_route }>, error_key: Option<String> }`. `use_ship` usa `JumpModel::range_now` con depósito lleno y carga actual; `Unknown` → `failed` con `nav-plan-error-no-range`. Modo `galaxy` necesita `JumpModel.fsd` (`ShipParams`); con `Scaled`/`Unknown` (sin FSD) → `failed` con `nav-plan-error-no-fsd` y el modo `neutron` sigue disponible con el alcance escalado. Galaxia Legacy → `disabled` con `nav-plan-disabled-legacy` y 0 peticiones. Servicios desactivados → `disabled` con `nav-plan-disabled-offline`.
  - Persistencia: aceptar/avanzar/fuera de ruta → `save_plan`; llegada o `clear-plan` → `delete_plan`; al cargar el comandante (`LoadGame`/`Commander`) → `load_plan`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn plan_action_requests_neutron_route_with_current_range() { /* NetQuery::NeutronRoute con range_ly == range_now del JumpModel, efficiency==60 (Settings por defecto), from==sistema actual, to=="Colonia" */ }
#[test] fn route_response_moves_to_ready_and_accept_activates() { /* NetResponse Route(2 hops) → status "ready", result.hops.len()==2; accept-plan → status "active", active.total==2, current_index==0 */ }
#[test] fn planner_disabled_without_online_services() { /* online_services=false → status "disabled", error_key=="nav-plan-disabled-offline", 0 peticiones */ }
#[test] fn legacy_galaxy_never_requests() { /* galaxy Legacy + online → plan → status "disabled", error_key=="nav-plan-disabled-legacy", 0 peticiones */ }
#[test] fn plan_without_known_range_fails_with_reason() { /* JumpModel Unknown, use_ship=true → status "failed", error_key=="nav-plan-error-no-range" */ }
#[test] fn galaxy_plan_without_fsd_fails_with_reason_and_neutron_still_works() { /* JumpModel Scaled (fsd None): mode galaxy → error_key=="nav-plan-error-no-fsd"; mode neutron → 1 petición NeutronRoute */ }
#[test] fn recalculate_uses_current_system_as_origin() { /* plan activo + sistema actual "X" → NetQuery.from=="X" y to == destino del plan */ }
#[test] fn active_plan_is_saved_and_restored_per_cmdr() { /* Store en memoria: aceptar → reiniciar Navigation → LoadGame mismo cmdr → plan e índice restaurados; otro cmdr → sin plan */ }
#[test] fn restored_plan_is_reconciled_with_current_system() { /* Stored con plan en index 0 llega tras FSDJump al hop 2 → current_index==2, off_route==false; con sistema ajeno → off_route==true */ }
#[test] fn arrival_deletes_plan() { /* on_jump al destino → StoreTask delete_plan emitida y topic active==None */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): planificador de rutas Spansh con ruta activa persistente`

### Task 17: `tmc-app` — copia automática y manual desde la ruta activa

**Files:** Modify `crates/tmc-app/src/{wiring.rs, clipboard.rs}`, `crates/tmc-modules/src/navigation/mod.rs`, `locales/*/navigation.ftl`.

**Interfaces:**
- Produces:
  - La tarea de copia automática observa `navigation.route.copy_seq`/`copy_target` (no `core.route`): copia cuando `copy_seq` aumenta tras inicializarse y `config.auto_copy` (misma fuente que `ModuleSettings.auto_copy_next_system`); en cada `FSDJump` **no replay** `Navigation` primero avanza el plan (`on_jump`), luego incrementa `copy_seq`, luego recalcula `copy_target`, y emite `Alert::new(Info, "nav-auto-copied").arg("system", ..)` si `settings.auto_copy_next_system`.
  - `clipboard.copy-next-system`: usa `navigation.route.copy_target` (siguiente del plan activo, o de `NavRoute`), con respaldo `core.route.next_system`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn manual_copy_prefers_active_plan_next_system() { /* plan A,B,C en A + NavRoute distinta → portapapeles falso recibe "B" */ }
#[test] fn manual_copy_falls_back_to_core_route() { /* sin plan ni copy_target, core.route.next_system=="Z" → copia "Z" */ }
#[test] fn copy_seq_only_increases_on_live_jumps() { /* replay de 3 FSDJump → copy_seq==0; 2 FSDJump en vivo → copy_seq==2 y copy_target==siguiente tras el avance del plan */ }
#[tokio::test] async fn auto_copy_follows_plan_after_each_jump() { /* start() con tempdir + plan aceptado + 2 FSDJump en vivo → last_copied == siguiente del plan */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(app): copia del siguiente sistema desde la ruta activa`

### Task 18: UI — disposiciones, ancho completo, tipografía y tira de ruta

**Files:** Create `ui/src/shell/layouts.ts`, `ui/src/modules/navigation/{index.ts, RouteStrip.svelte, SystemCard.svelte, JumpPanel.svelte}`, `ui/tests/{layouts.test.ts, navigation.test.ts}`; Modify `ui/src/shell/{Grid.svelte, Panel.svelte, Sidebar.svelte}`, `ui/src/App.svelte`, `ui/src/styles/tokens.css`, `ui/src/modules/registry.ts`, `ui/src/lib/format.ts`, `locales/*/navigation.ftl`.

**Interfaces:**
- Produces:
  - `Panel` + prop `span?: 'full' | 1 | 2` (`full` → `grid-column: 1 / -1`) y `size?: 'normal' | 'large'` (`large` usa `--font-body-lg` y `--font-title-lg`).
  - `tokens.css`: `--font-body-lg: 15px; --font-title-lg: 20px;` y en `@media (min-width: 2400px)`: `17px` y `24px`.
  - `layouts.ts`: `export interface LayoutSlot { id: string; panel: Component; span: 'full' | 1 | 2; when?: (ctx: LayoutCtx) => boolean }`, `export interface LayoutDef { id: string; titleKey: string; icon: string; slots: LayoutSlot[] }`, `export function contextPanel(mods: { id: string; relevance: string }[], manual: string | null): 'exploration' | 'exobiology'` (manual gana; si no, `exobiology` cuando su relevancia es `active`/`urgent`; si no, `exploration`), `export const layouts: LayoutDef[]` (se rellena en Task 31).
  - `registry.ts`: entrada `{ id: 'navigation', icon: 'route', titleKey: 'nav-navigation', panels: navigationPanels }`.
  - `RouteStrip.svelte` (span `full`, size `large`): bloque principal ≈ 3/4 con 4 columnas (Salto · Exploración · Política · Sistema, spec §3.2) + 3 bloques pequeños; «sin datos» (`t('empty-no-data')`) en lo remoto no disponible; guerra/disturbios/brote resaltados con `--hud-danger`; antigüedad de caché si `source == 'cache'`; insignia «fuera de ruta» con botón recalcular (`action('navigation.recalculate')`).
  - `JumpPanel.svelte` (span 1): lee el topic `navigation.jump`; muestra alcance actual y máximo, combustible y máx. por salto, indicador «supercargado ×N» si `supercharged` y «sin datos» si `source == 'unknown'`; registrado en `navigation/index.ts` como panel `jump`.
  - `format.ts` + `ageShort(seconds)` y `starClassLabel(cls)`.

- [ ] **Step 1: Tests:**

```ts
test('contextPanel prefers manual, then active exobiology, else exploration', ...)
test('route strip renders next star card and three following', ...)        // topics simulados → 4 tarjetas, nombre del siguiente visible
test('remote missing shows no-data without hiding local fields', ...)      // remote.status 'failed' → texto empty-no-data y jump_ly visible
test('war and outbreak states are highlighted', ...)
test('off-route badge shows recalculate button that calls action', ...)
test('full span panel spans all grid columns', ...)
test('jump panel shows range, fuel and supercharge', ...)       // range_ly 24.6, supercharged 4 → '24.6' y '×4' visibles; source 'unknown' → empty-no-data
```

- [ ] **Step 2:** `npm --prefix ui test` → FAIL. **Step 3:** implementar (sin animaciones infinitas; transiciones ≤ 200 ms). **Step 4:** PASS; Checks UI + Rust (test de claves).
- [ ] **Step 5: Commit** `feat(ui): tira de ruta a todo el ancho con tipografía mayor`

### Task 19: UI — panel del planificador

**Files:** Create `ui/src/modules/navigation/PlannerPanel.svelte`; Modify `ui/src/modules/navigation/index.ts`, `ui/tests/navigation.test.ts`, `locales/*/navigation.ftl`.

**Interfaces:**
- Consumes: topic `navigation.plan`, `navigation.jump`, `GET /api/navigation/autocomplete`, `action(name, args)`.
- Produces: formulario (origen = sistema actual por defecto, destino con autocompletado con espera de 300 ms tras teclear y mínimo 3 caracteres, modo Neutrones/Galaxia, eficiencia, «usar nave actual» por defecto sí, opciones de Galaxia), estado `computing` sin spinner infinito (texto), resultado (sistema, distancia, neutrón, repostar; totales) con botón «Aceptar» → `navigation.accept-plan`, «Borrar plan». Con `status == 'disabled'` el formulario se deshabilita y muestra la explicación (`nav-plan-disabled-offline`).

- [ ] **Step 1: Tests:**

```ts
test('autocomplete debounces and requires 3 chars', ...)   // fake timers: 2 letras → 0 fetch; 3 letras + 300 ms → 1 fetch
test('submit sends navigation.plan with mode and efficiency', ...)
test('ready result lists hops and accept calls action', ...)
test('disabled planner explains why', ...)
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks UI.
- [ ] **Step 5:** Validación visual con el propietario (spec §3.5: no hay maqueta) — captura con `tmc-headless` + replay; anotar su respuesta en `status.md`.
- [ ] **Step 6: Commit** `feat(ui): panel del planificador de rutas`

---

## Bloque 3 — Exploración

### Task 20: `exploration` — valor de cuerpos y sondas

**Files:** Create `crates/tmc-modules/src/exploration/{mod.rs, value.rs}` (`mod.rs` solo declara `pub mod value;`; el struct `Exploration` lo define Task 21), `crates/tmc-modules/data/probes.json`.

**Interfaces:**
- Consumes: `Scan`, `SaaScanComplete`.
- Produces:
  - `pub enum BodyKind { Star { k: f64 }, Planet { k: f64 } }`; `pub fn body_kind(scan: &Scan) -> Option<BodyKind>` (k por clase/terraformable según `player_roles_domain.md` §1; cinturones/anillos → `None`).
  - `pub struct ValueFlags { pub first_discovery: bool, pub first_mapped: bool, pub efficiency: bool, pub odyssey: bool }`.
  - `pub fn star_value(k: f64, stellar_mass: f64, first_discovery: bool) -> u64`, `pub fn planet_base(k: f64, mass_em: f64) -> f64`, `pub fn planet_scanned_value(base: f64, first_discovery: bool) -> u64`, `pub fn planet_mapped_value(base: f64, f: &ValueFlags) -> u64`, `pub fn body_values(scan: &Scan, odyssey: bool) -> BodyValues { scanned: u64, mapped: Option<u64> }` (primer descubrimiento = `WasDiscovered == Some(false)`; primer mapeo = `WasMapped == Some(false)`).
  - `pub fn estimate_probes(radius_m: f64) -> u8` a partir de `probes.json` (tramos por radio; valores iniciales del redactor, ajustados con `tmc-headless eval` contra `SAAScanComplete.EfficiencyTarget`).
- Fórmulas (hechos públicos; implementación propia): estrella `k + m·k/66,25` (×2,6 si primer descubrimiento); planeta `base = max(k + k·0,56591828·M^0,2, 500)`; FSS = base (×2,6 primer descubrimiento); mapeado: ×3,3333333 (ya descubierto y mapeado), ×8,0956 (primer mapeo sin primer descubrimiento), ×3,699622554×2,6 (ambos); eficiencia ×1,25; Odyssey `v + max(0,3 · v ; 555)` (`0,3` es el decimal 0.3; `;` separa los argumentos del máximo).

- [ ] **Step 1: Tests:**

```rust
#[test] fn earthlike_base_value() { assert!((planet_base(181_126.0, 1.0) - 283_628.5).abs() < 1.0); }
#[test] fn earthlike_scanned_first_discovery() { /* 737_434 ± 2 */ }
#[test] fn earthlike_mapped_known_with_efficiency_and_odyssey() { /* 1_536_321 ± 0.5 % */ }
#[test] fn first_mapping_without_first_discovery_uses_8_0956() { /* base 283_628.5, WasDiscovered true, WasMapped false → 2_296_143 ± 0.1 % */ }
#[test] fn first_discovery_and_first_mapping_use_combined_multiplier() { /* base 283_628.5, ambos primeros → ×3.699622554×2.6 = 2_728_228 ± 0.1 % */ }
#[test] fn small_rocky_body_has_floor_of_500() { /* planet_base(k 300, M 0.0001) == 500.0 */ }
#[test] fn neutron_star_value() { /* k 22628, m 1.4 → 23_106 ± 1 */ }
#[test] fn g_star_value() { /* k 1200, m 1.0 → 1_218 ± 1 */ }
#[test] fn belt_cluster_has_no_value() { /* Scan con body_name "... Belt Cluster 1" y sin PlanetClass/StarType → body_kind == None */ }
#[test] fn probe_estimate_is_monotonic_in_radius() { /* radios 500 km, 3000 km, 8000 km → n1 <= n2 <= n3 y n1 >= 1; estimate_probes(0.0) == 1 */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): fórmula de valor de cuerpos y estimación de sondas`

### Task 21: `exploration` — cuerpos del sistema, FSS y consejo

**Files:** Create `crates/tmc-modules/src/exploration/system.rs`; Modify `exploration/mod.rs` (aquí se define `pub struct Exploration`), `registry.rs`, `locales/*/exploration.ftl`; Create `tests/fixtures/journal/exploration_big_system.log` (generado por el test o escrito por script en el scratchpad: 300 `Scan` sintéticos).

**Interfaces:**
- Consumes: Task 20, `FssDiscoveryScan`, `FssAllBodiesFound`, `FssBodySignals`, `SaaScanComplete`, `SaaSignalsFound`, `Scan`, `Status.gui_focus`, `ModuleSettings.map_value_threshold`.
- Produces:
  - `pub struct Exploration` (`impl Module`, id `"exploration"`), registrada en `all_modules()`.
  - `BodyRow { body_id: u32, short_name, kind_key: String, distance_ls: Option<f64>, scanned_value: u64, mapped_value: Option<u64>, bio: u32, geo: u32, terraformable, landable, scoopable /* vía data::star_flags */, first_discovery, already_mapped, mapped_by_you, probes: Option<ProbeHint { n: u8, estimated: bool }>, advice: Vec<Advice> }`; `enum Advice { Map, Land, NotWorth }` (Map si `mapped_value ≥ umbral`; Land si bio > 0 y aterrizable; combinables; si ninguno, NotWorth).
  - Topic `exploration.system` = `{ system, system_address, discovered: Option<bool>, fss: { progress, body_count, scanned, complete }, value_now: u64, value_if_mapped: u64, unidentified: Option<u32>, rows: Vec<BodyRow> /*ordenadas por mapped_value desc, luego scanned*/ }` en forma compacta (campos `Option` omitidos con `skip_serializing_if`).
  - Serialización compacta (2026-10-18): las seis marcas booleanas de `BodyRow` se envían en `marks: u8` (ausente = 0): 1 terraformable, 2 landable, 4 scoopable, 8 first_discovery, 16 already_mapped, 32 mapped_by_you. Los campos Rust conservan sus nombres/tipos; los seis campos booleanos no se repiten en JSON. `probes.estimated` omitido equivale a `false`; `advice` omitido equivale a `['not_worth']`; `Map`/`Land` siempre explícitos. La UI de Task 23 debe decodificar ese contrato.
  - Se reinicia en `FSDJump`/`CarrierJump`/`Location` de otro sistema. `Scan` repetido del mismo `BodyID` actualiza la fila (sin duplicar).
  - Relevancia: `Active` con `GuiFocus::Fss` o con `body_count > scanned`; si no, `Normal`.

- [x] **Step 1: Tests:**

```rust
#[test] fn fss_progress_counts_scanned_vs_body_count() { /* FSSDiscoveryScan{BodyCount:10} + 4 Scan de BodyID distintos → fss.body_count==10, fss.scanned==4, complete==false */ }
#[test] fn all_bodies_found_marks_complete() { /* FSSAllBodiesFound{Count:10} → fss.complete==true */ }
#[test] fn rows_sorted_by_mapped_value_desc() { /* mapped 1.5M, 300k, sin mapeo (estrella) con scanned 2k → orden [1.5M, 300k, estrella] */ }
#[test] fn advice_map_over_threshold_and_land_with_bio() { /* umbral 500k: fila 1.5M → [Map]; aterrizable bio 2 y 200k → [Land]; 1.5M con bio 2 aterrizable → [Map, Land]; 200k sin bio → [NotWorth] */ }
#[test] fn threshold_change_via_settings_recomputes_advice() { /* fila 600k con umbral 500k → [Map]; Settings{map_value_threshold:1_000_000} → [NotWorth] */ }
#[test] fn saa_complete_replaces_estimated_probes() { /* EfficiencyTarget 6 → ProbeHint{n:6, estimated:false}, mapped_by_you true */ }
#[test] fn duplicate_scan_updates_row_not_duplicates() { /* dos Scan del BodyID 7 (el 2.º con valores distintos) → rows.len()==1 y valores del 2.º */ }
#[test] fn jump_resets_system() { /* tras FSDJump a otro sistema → rows vacío, fss.scanned==0, value_now==0 */ }
#[test] fn body_row_scoopable_uses_shared_star_flags() { /* Scan de estrella "K" → scoopable==true; "N" → false */ }
#[test] fn relevance_active_in_fss_or_with_unscanned_bodies() { /* body_count 10, scanned 4 → Relevance::Active; scanned 10 y GuiFocus no Fss → Normal */ }
#[test] fn system_with_300_bodies_keeps_topic_under_64_kib() { /* 300 Scan → serde_json::to_vec(topic).len() < 65_536 y orden correcto */ }
```

- [x] **Step 2:** FAIL de regresiones observado durante revisión; ciclo inicial de 9940901 no reconstruido. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [x] **Step 5: Commit** `feat(modules): cuerpos del sistema con progreso FSS, valores y consejo`

### Task 22: `exploration` — datos sin vender

**Files:** Create `crates/tmc-modules/src/exploration/unsold.rs`, `exploration/unsold/history.rs`; Modify `exploration/mod.rs`.

**Interfaces:**
- Produces:
  - `pub struct UnsoldLedger { by_body: BTreeMap<(String, u32), u64> /* clave (sistema, body_id) */, mapped_pending: BTreeMap<(String, u32), u64> }` con `fn set(&mut self, system: &str, body_id: u32, value: u64)` (idempotente: sobrescribe, así un `Scan` repetido no duplica y el mapeo sube el valor), `fn apply_event(&mut self, current_system: &str, ev: &EventKind, odyssey: bool)` (**única vía de actualización**, en vivo, en `rebuild_from_history` y en `eval`: `Scan` → `set` con el valor de escaneo y guarda el de mapeo pendiente; `SaaScanComplete` → `set` con el valor de mapeo, con eficiencia si `probes_used <= efficiency_target`; `SellExplorationData` → `sell(systems)`; `MultiSellExplorationData` → `sell(discovered)` o todo si la lista está vacía; `Died` → `died()`), `fn sell(&mut self, systems: &[String])`, `fn sell_all(&mut self)`, `fn died(&mut self)`, `fn total(&self) -> u64`, `fn system_count(&self) -> u32`.
  - `pub fn rebuild_from_history(cmdr: String) -> StoreTask`: lee desde el último `Died`/venta total los eventos del comandante (consulta por `name IN (...)`, snapshot `id <= cut` y orden cronológico `(ts, journal_file.name, line, id)` con páginas de 256, sin cargar todo el historial en memoria), los pasa por `UnsoldLedger::apply_event` y devuelve el libro serializado (sin lógica duplicada).
  - Topic `exploration.unsold` = `{ total: u64, systems: u32, at_risk: bool }`.

- [x] **Step 1: Tests:**

```rust
#[test] fn partial_sale_removes_only_listed_systems() { /* A: cuerpos 100 y 200; B: 300 → sell(["A"]) → total()==300, system_count()==1 */ }
#[test] fn multi_sell_removes_listed_systems() { /* MultiSell con Discovered [A] → total()==300 (queda B); con lista vacía → total()==0 */ }
#[test] fn death_zeroes_unsold() { /* tras Died → total()==0 y system_count()==0 */ }
#[test] fn repeated_scan_does_not_double_count() { /* mismo (A, body 7) con Scan dos veces, valor 1000 → total()==1000 */ }
#[test] fn mapping_upgrades_value_without_double_counting() { /* Scan 1000 + SaaScanComplete (mapeo 5000) del mismo cuerpo → total()==5000, no 6000 */ }
#[test] fn rebuild_from_history_matches_live_accumulation() { /* Store en memoria con exploration_session.log importado → total de rebuild_from_history == total tras aplicar apply_event en vivo (valor numérico idéntico) */ }
```

- [x] **Step 2:** RED observado: `cargo test -p tmc-modules --offline exploration::unsold` → 0/14 PASS con stubs (2026-10-18). Incluye casos del plan, paginación, contexto tras reinicio, ventas parciales, aislamiento por comandante y eventos en vuelo.
- [x] **Step 3:** implementar. **Step 4:** PASS; revisión independiente aprobada (2026-10-18). `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --offline -- -D warnings`, `cargo test --workspace --offline --quiet` → 413 tests PASS (153 módulos); `git diff --check` PASS. UI: `npm.cmd --prefix ui run check` → 0 errores/avisos; `test` → 46 PASS; `build` PASS. Regresiones RED/GREEN adicionales: orden histórico, retry tras fallo SQL, checkpoints de 600 Scan, importación tardía y contexto Odyssey ausente. La medición de 50 ms con el historial del propietario (`tmc-headless eval --timings`) **se hace en Task 29 Step 5** (esta tarea no depende de `eval`): si > 50 ms, anotar en «Desviaciones» la tabla de resumen (spec §7).
- [x] **Step 5: Commit** `feat(modules): datos de exploración sin vender reconstruidos del historial`

### Task 23: UI — panel «Cuerpos del sistema»

**Files:** Create `ui/src/shell/VirtualList.svelte`, `ui/src/modules/exploration/{index.ts, SystemBodiesPanel.svelte}`, `ui/tests/exploration.test.ts`; Modify `ui/src/modules/registry.ts`, `locales/*/exploration.ftl`.

**Interfaces:**
- Produces:
  - `VirtualList.svelte`: props `items: T[]`, `rowHeight: number`, `height: number`, snippet `row`; renderiza solo las filas visibles + 5 de margen; recalcula **solo** en `scroll`/cambio de `items` (sin rAF continuo).
  - `SystemBodiesPanel.svelte` (size `large`): cabecera (sistema, descubierto, barra FSS, valor actual y si se mapea), tabla (columnas de spec §3.3; «—» en mapeo de estrellas; «≈» en sondas estimadas; filas `Map` resaltadas), pie (sin identificar + datos sin vender de `exploration.unsold`).
  - Contrato JSON de Task 21: en cada fila, `advice ?? ['not_worth']`, `probes.estimated ?? false`, marcas desde `marks ?? 0` (bits 1 terraformable, 2 landable, 4 scoopable, 8 first_discovery, 16 already_mapped, 32 mapped_by_you), `bio`/`geo` ausentes = 0; `kind_key` se traduce con `exploration-kind-<id>` y el consejo `not_worth` usa `exploration-advice-not-worth`.
  - `registry.ts`: entrada `exploration`.

- [ ] **Step 1: Tests:**

```ts
test('body_table_renders_only_visible_rows', ...)      // 300 filas, height 600, rowHeight 30 → ≤ 30 filas en el DOM
test('map advice rows are highlighted and show probes', ...)
test('stars show dash in mapped column', ...)
test('footer shows unsold total and unidentified count', ...)
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks UI.
- [ ] **Step 5: Commit** `feat(ui): panel de cuerpos del sistema con tabla virtualizada`

---

## Bloque 4 — Exobiología

### Task 24: `exobiology` — tabla de especies y geodesia

**Files:** Create `crates/tmc-modules/data/species.json`, `crates/tmc-modules/src/exobiology/{mod.rs, species.rs, geo.rs}`; Modify `THIRD_PARTY_NOTICES.md` si Task 1 lo exige.

**Interfaces:**
- Produces:
  - `species.json`: cabecera fuente/licencia/versión + `genera: [{ genus: "$Codex_Ent_Bacterial_Genus_Name;", name: "Bacterium", ccr_m: 500, species: [{ species: "$Codex_Ent_Bacterial_01_Name;", name: "Bacterium Aurasus", value: 1000000 }] }]`. Los nombres visibles de género/especie salen de `_Localised` del diario (`GenusRef.genus_localised`, `ScanOrganic.species_localised`) y los nombres latinos de esta tabla son el respaldo cuando falta; **no hay tabla propia de nombres por idioma en F1** (aplazada; ver «Desviaciones», spec §8).
  - `pub struct SpeciesTable`; `data::species_table() -> &'static SpeciesTable`; `fn genus(&self, symbol: &str) -> Option<&GenusInfo>`, `fn species(&self, symbol: &str) -> Option<&SpeciesInfo>`, `fn species_of_genus(&self, genus: &str) -> &[SpeciesInfo]`. `GenusInfo { symbol, name, ccr_m: u32 }`, `SpeciesInfo { symbol, genus, name, value: u64 }`.
  - `geo.rs`: `pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64, radius_m: f64) -> f64`, `pub fn bearing_deg(lat1, lon1, lat2, lon2) -> f64` (0 = N, sentido horario), `pub fn relative_bearing(bearing: f64, heading: f64) -> f64` (0..360), `pub enum RelDir { Ahead, AheadRight, Right, BehindRight, Behind, BehindLeft, Left, AheadLeft }` + `fn rel_dir(rel: f64) -> RelDir`, `pub fn compass(bearing: f64) -> &'static str` (`"N"`, `"NE"`, …; la UI traduce con clave `compass-<x>`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn haversine_on_equator_matches_reference() { /* R=1_000_000 m, (0,0)→(0,0.1) → 1745.33 ± 1.0 m */ }
#[test] fn haversine_symmetric_and_zero_for_same_point() { /* d(a,b)==d(b,a) ± 1e-6 m y d(a,a)==0.0 */ }
#[test] fn bearing_east_and_north() { /* (0,0)→(0,0.1) = 90 ± 0.01; (0,0)→(0.1,0) = 0 ± 0.01 */ }
#[test] fn relative_direction_behind() { /* bearing 90, heading 270 → 180 → Behind; compass(225) == "SW" */ }
#[test] fn species_table_has_ccr_for_every_genus_and_values_positive() { /* todos los géneros con ccr_m > 0 y todas las especies con value > 0; species_of_genus no vacío */ }
#[test] fn bacterium_ccr_is_500_and_osseus_800() { assert_eq!(table.genus("$Codex_Ent_Bacterial_Genus_Name;").unwrap().ccr_m, 500); /* y el género Osseus tiene ccr_m == 800 */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): tabla de especies con CCR y cálculo de distancia y rumbo`

### Task 25: `exobiology` — predicción (reglas + historial + Canonn)

**Files:** Create `crates/tmc-modules/data/bio_rules.json`, `crates/tmc-modules/src/exobiology/predict.rs`.

**Interfaces:**
- Consumes: `Scan` (perfil del cuerpo), estrella principal del sistema (primer `Scan` de estrella con `DistanceFromArrivalLS == 0`), `SaaSignalsFound.genuses`, `RemoteBodyBio`.
- Produces:
  - `bio_rules.json` (datos versionados, escritos por TMC desde hechos públicos; **nunca** derivados de SrvSurvey): `{ "version": "...", "rules": [{ "species": "<símbolo>", "body_class": [...]?, "atmosphere_type": [...]?, "gravity_max": f64?, "gravity_min"?, "temp_min"?, "temp_max"?, "pressure_max"?, "volcanism": "any|none|[...]"?, "star_class": [...]?, "source": "texto/URL" }] }` (cláusulas omitidas = sin restricción; AND entre cláusulas; varias reglas por especie = OR).
  - `pub fn body_profile(scan: &Scan, star_class: Option<&str>) -> BodyProfile` (Task 6): convierte unidades del diario → `gravity_g = SurfaceGravity / 9,80665` (m/s² → g), `pressure_atm = SurfacePressure / 101 325` (Pa → atm), `temperature_k = SurfaceTemperature`. `impl From<&BodyProfile> for BodyProfileKey` (tramos descritos abajo).
  - `pub enum Likelihood { High, Medium, Low, Unknown }`; `pub enum ProbSource { Rules, History, Canonn }`; `pub struct Candidate { pub species: String, pub genus: String, pub value: u64, pub likelihood: Likelihood, pub sources: Vec<ProbSource> }`.
  - `pub struct HistoryStats` — `fn from_rows(rows: impl Iterator<Item = (BodyProfileKey, String /*species*/)>) -> Self`, `fn count(&self, key: &BodyProfileKey, species: &str) -> u32`, `fn total(&self, key) -> u32`; `BodyProfileKey` = (clase, tipo de atmósfera, gravedad en tramos de 0,05 g, temperatura en tramos de 20 K, volcanismo sí/no, clase de estrella). `pub fn load_history(cmdr: String) -> StoreTask` (une `Scan` y `ScanOrganic` por `SystemAddress`+`BodyID`).
  - `pub fn predict(profile: &BodyProfile, confirmed_genera: Option<&[String]>, history: &HistoryStats, canonn: Option<&RemoteBodyBio>, rules: &BioRules, table: &SpeciesTable) -> Vec<Candidate>`: reglas que casan → `Medium` (+`Rules`); historial (+`History`): con regla, ≥ 3 casos para la clave y especie → `High` y 1–2 casos → sube un nivel; **sin regla** (solo historial), ≥ 3 casos → `Medium` y 1–2 casos → `Low`; Canonn `weight ≥ 0,5` → sube un nivel, `< 0,1` → baja uno con mínimo `Low` (+`Canonn`); `Unknown` = género sin reglas, sin historial y sin pista de Canonn; con DSS solo géneros confirmados. Orden: valor desc.

- [ ] **Step 1: Tests:**

```rust
#[test] fn body_profile_converts_units() { /* Scan SurfaceGravity 19.6133 → gravity_g 2.000 ± 0.001; SurfacePressure 202650 → pressure_atm 2.000 ± 0.001; SurfaceTemperature 250 → temperature_k 250.0 */ }
#[test] fn rule_match_gives_medium_from_rules() { /* perfil que casa 1 regla → Candidate{likelihood:Medium, sources:[Rules]} */ }
#[test] fn history_with_three_cases_gives_high() { /* regla + 3 casos en historial → High, sources [Rules, History] */ }
#[test] fn history_only_without_rule_is_low_with_one_case() { /* sin regla y 1 caso → Low, sources [History]; con 3 casos → Medium */ }
#[test] fn canonn_hint_raises_one_level_and_is_listed_as_source() { /* regla (Medium) + weight 0.6 → High, sources contiene Canonn */ }
#[test] fn canonn_low_weight_lowers_medium_to_low() { /* regla (Medium) + weight 0.05 → Low */ }
#[test] fn without_canonn_layers_one_and_two_still_work() { /* canonn None → mismos candidatos con reglas+historial, sin Canonn en sources */ }
#[test] fn genus_without_rules_is_unknown_not_invented() { /* género confirmado sin reglas/historial → Candidate{likelihood:Unknown} y no inventa especies */ }
#[test] fn dss_genuses_filter_candidates() { /* confirmed_genera ["Bacterium"] → todos los candidatos tienen genus=="Bacterium" */ }
#[test] fn candidates_sorted_by_value_desc() { /* valores 1_000_000, 500_000, 2_000_000 → orden [2M, 1M, 500k] */ }
#[test] fn bio_rules_have_source_for_every_rule() { /* ninguna regla de bio_rules.json tiene "source" vacío; conteo de reglas > 0 */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): predicción de especies con reglas propias, historial y Canonn`

### Task 26: `exobiology` — muestreo y `exo_sample`

**Files:** Create `crates/tmc-modules/src/exobiology/{sampling.rs, persist.rs}`, `tests/fixtures/journal/exobiology_session.log`, `tests/fixtures/status/exo_{01,02,03}.json`.

**Interfaces:**
- Consumes: `ScanOrganic`, último `Status` (lat/lon/heading/planet_radius), `Died`, `SellOrganicData`, Task 24.
- Produces:
  - Migración `Migration { id: "exobiology-001", sql: "CREATE TABLE exo_sample (id INTEGER PRIMARY KEY, cmdr TEXT NOT NULL, system_address INTEGER NOT NULL, body_id INTEGER NOT NULL, species TEXT NOT NULL, scan_type TEXT NOT NULL, lat REAL, lon REAL, ts TEXT NOT NULL, UNIQUE(cmdr, system_address, body_id, species, scan_type, ts));" }`.
  - `pub struct SamplePoint { lat: f64, lon: f64, ts: DateTime<Utc> }`; `pub struct Sampling { pub system_address: u64, pub body_id: u32, pub species: String, pub genus: String, pub points: Vec<SamplePoint>, pub complete: bool }`.
  - `pub struct ExoSampleRow { pub cmdr: String, pub system_address: u64, pub body_id: u32, pub species: String, pub scan_type: String, pub lat: Option<f64>, pub lon: Option<f64>, pub ts: DateTime<Utc> }` (`Serialize + Deserialize`, una fila de `exo_sample`).
  - `pub struct SamplingTracker` — `fn on_scan_organic(&mut self, ev: &ScanOrganic, pos: Option<(f64, f64)>, ts) -> SamplingChange` (`enum SamplingChange { Started, Progressed(u8), Completed, Discarded { previous_species: String }, Ignored }`; `Log` de otra especie con muestreo incompleto → `Discarded`), `fn distance_to_previous(&self, now: (f64, f64), radius_m: f64) -> Option<f64>`, `fn direction_to_previous(&self, now: (f64, f64), heading: f64) -> Option<(RelDir, &'static str)>`, `fn completed(&self) -> &[(u64, u32, String)]`, `fn on_sold(&mut self)` (vacía `completed` tras `SellOrganicData`), `fn died(&mut self)` (vacía muestreo en curso y `completed`), `fn restore(&mut self, rows: Vec<ExoSampleRow>)`.
  - `persist::save_sample(cmdr, row: ExoSampleRow) -> StoreTask` (el módulo la emite en cada `ScanOrganic` **no replay**), `persist::load_open_samples(cmdr) -> StoreTask` (último muestreo no completado por cuerpo), `persist::clear_samples(cmdr) -> StoreTask` (en `Died` y al completar o vender).
  - Durante el replay del diario actual, la posición de cada `ScanOrganic` se toma de la fila `exo_sample` con mismo `ts` (no del `Status`, que no se reproduce).

- [ ] **Step 1: Tests:**

```rust
#[test] fn log_sample_analyse_completes_species() { /* Log→Started, Sample→Progressed(2), Analyse→Completed; completed().len()==1 */ }
#[test] fn switching_species_mid_sampling_discards_previous() { /* Log de especie A, Log de especie B → Discarded{previous_species:"A"} */ }
#[test] fn distance_to_previous_uses_planet_radius() { /* puntos de exo_01/02.json → distancia de referencia calculada con haversine_m en el test, ±1 m */ }
#[test] fn direction_to_previous_relative_to_heading() { /* previo al este (bearing 90), heading 270 → (RelDir::Behind, "E") */ }
#[test] fn sampling_resumes_after_restart_from_exo_sample() { /* Store en memoria: 2 muestras guardadas → tracker nuevo + load_open_samples → progreso 2/3 y distancia correcta, aunque el diario actual no contenga los ScanOrganic */ }
#[test] fn replayed_scan_organic_uses_stored_position() { /* ScanOrganic en replay con fila exo_sample de mismo ts (lat 10.0, lon 20.0) → punto (10.0, 20.0), no el Status */ }
#[test] fn live_scan_organic_emits_save_sample_task_and_replay_does_not() { /* en vivo: 1 StoreTask "save_sample"; replay: 0 */ }
#[test] fn on_sold_clears_completed() { /* 2 completadas → on_sold() → completed().is_empty() */ }
#[test] fn died_clears_sampling_tracker() { /* muestreo en curso + 1 completada → died() → sin muestreo y completed().is_empty() */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): máquina de muestreo con posiciones persistentes`

### Task 27: `exobiology` — módulo, bonus ×5, bio sin vender y relevancia

**Files:** Modify `crates/tmc-modules/src/exobiology/mod.rs`, `registry.rs`, `locales/*/exobiology.ftl`.

**Interfaces:**
- Consumes: Tasks 24–26, `FssBodySignals`, `SaaSignalsFound`, `Touchdown`, `Disembark`, `Liftoff`, `Embark`, `ApproachBody`, `Status`, `SourceEvent::{Net, Stored, Settings}`.
- Produces:
  - `pub struct Exobiology` (`impl Module`, id `"exobiology"`), registrada en `all_modules()`.
  - `pub fn first_logged_estimate(scan: Option<&Scan>, system_discovered: Option<bool>) -> Option<bool>` (`WasFootfalled == Some(false)` → `true`; sin campo → `!system_discovered`; sin datos → `None`). Se confirma con `SellOrganicData.Bonus > 0` y se registra la tasa de acierto para `eval`.
  - Topics: `exobiology.body` = `{ body, body_id, kind, gravity_g, temperature_k, atmosphere, bio_signals, first_logged: Option<bool> /*estimado*/, candidates: Vec<Candidate + status: "none"|"in-progress"(n)|"done">, confirmed_genera: Vec<String>, completed: u32 }`; `exobiology.sampling` = `{ species, points: u8, distance_m: Option<f64>, min_m: u32, remaining_m: Option<f64>, direction: Option<{ rel, compass }>, unsold_after: u64, discarded_species: Option<String> /* spec §5.3: muestreo descartado por cambio de especie */ }`; `exobiology.unsold` = `{ total: u64, species: u32 }` donde `total` = Σ(`value` + bonus estimado) de las especies completadas (bonus = 4 × `value` si `first_logged == Some(true)`, es decir ×5 en total; 0 en otro caso); se pone a 0 con `SellOrganicData` (`SamplingTracker::on_sold`) o `Died` (`SamplingTracker::died`); reconstruido con `StoreTask` al arrancar.
  - Petición Canonn: `NetQuery::BodyBio` al detectar señales biológicas o al aterrizar (si `online_services` y no Legacy), clave `"bio:<addr>:<body>"`; respuesta aplicada solo si el cuerpo sigue siendo el actual o el de las señales.
  - Relevancia: `Active` si aterrizado / a pie / en SRV en un cuerpo con señales biológicas (spec §3.1) o con muestreo en curso en ese cuerpo; `ScanOrganic` sin señales previas también activa; si no, `Normal`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn touchdown_on_bio_body_sets_active_relevance() { /* FSSBodySignals con Biological Count 3 + Touchdown en ese BodyID → relevance()==Active */ }
#[test] fn touchdown_without_signals_stays_normal_until_scan_organic() { /* Touchdown sin señales → Normal; tras ScanOrganic → Active */ }
#[test] fn exact_species_replaces_same_genus_candidates() { /* candidatos del género Bacterium (3 especies) + ScanOrganic de una especie → candidates solo con esa especie del género */ }
#[test] fn first_logged_estimate_uses_footfall_then_discovery() { assert_eq!(first_logged_estimate(Some(&scan_footfalled_false), None), Some(true)); assert_eq!(first_logged_estimate(None, Some(false)), Some(true)); assert_eq!(first_logged_estimate(None, None), None); }
#[test] fn unsold_bio_equals_following_sell_organic_data() { /* exobiology_session.log: total antes de vender == Σ(Value+Bonus) de SellOrganicData del fixture (valor numérico fijo en el test) */ }
#[test] fn died_zeroes_unsold_bio() { /* 2 especies completadas, total>0 → Died → exobiology.unsold.total==0 y species==0 */ }
#[test] fn switching_species_sets_discarded_species_in_topic() { /* Log de A y luego Log de B → exobiology.sampling.discarded_species==Some("A") */ }
#[test] fn canonn_request_on_bio_signals_only_when_online() { /* online_services=true → 1 NetQuery::BodyBio con key "bio:<addr>:<body>"; false → 0; Legacy → 0 */ }
#[test] fn late_canonn_response_for_other_body_is_ignored() { /* respuesta con key de otro cuerpo → candidates sin cambios y sin Canonn en sources */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(modules): módulo de exobiología con candidatos, muestreo y valor sin vender`

### Task 28: UI — panel de exobiología

**Files:** Create `ui/src/modules/exobiology/{index.ts, ExobiologyPanel.svelte}`, `ui/tests/exobiology.test.ts`; Modify `ui/src/modules/registry.ts`, `locales/*/exobiology.ftl`.

**Interfaces:**
- Produces: panel (size `large`) con cabecera (cuerpo, tipo, gravedad, temperatura, atmósfera, nº señales, insignia «SIN REGISTRAR · ×5 (estimado)»), lista de posibles especies (especie, valor base, valor ×5, CCR, probabilidad con icono/etiqueta de origen `reglas · tu historial · Canonn`, estado — / ▶ n/3 / ✓ 3/3), géneros confirmados y completados/señales; bloque de muestreo (n/3, barra distancia vs mínima, lo que falta, dirección «a tu espalda (SO)» con claves `dir-*` y `compass-*`, valor sin vender y lo que sumará). Sin muestreo: «esperando ScanOrganic».

- [ ] **Step 1: Tests:**

```ts
test('candidates show likelihood and its origin', ...)
test('sampling bar shows distance versus minimum and remaining', ...)
test('direction text uses relative and compass keys', ...)   // rel 'behind', compass 'SW' → t('dir-behind') y t('compass-SW')
test('first logged badge is marked as estimated', ...)
test('discarded species is shown in the sampling block', ...)   // discarded_species 'Bacterium Aurasus' → aviso con ese nombre visible
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks UI.
- [ ] **Step 5: Commit** `feat(ui): panel de exobiología con candidatos y muestreo`

---

## Bloque 5 — Cierre

### Task 29: `tmc-headless eval` (criterios 3, 4 y 6)

**Decisión del propietario (2026-10-17):** el subcomando vive en `apps/headless` como `tmc-headless eval` (las apps pueden depender de todo; sin cambio de arquitectura).

**Files:** Create `crates/tmc-modules/src/eval.rs`, `apps/headless/src/eval.rs`; Modify `apps/headless/{Cargo.toml, src/main.rs}`, `crates/tmc-modules/src/lib.rs`.

**Interfaces:**
- Produces:
  - `tmc_modules::eval`: `pub struct RangeReport { cases: u32, within_tolerance: u32, engineered_cases: u32, guardian_cases: u32, max_abs_error: f64, mean_abs_error: f64, full_tank_hits: u32, max_fuel_hits: u32 }` (compara `MaxJumpRange` con `jump_range_with(.., false)` en «depósito lleno» (`full_tank_hits`) **y** `jump_range_with(.., true)` en «solo combustible máximo por salto» (`max_fuel_hits`), ambas sin reutilizar el valor recortado de la otra; si solo la segunda coincide, se documenta como discrepancia con spec §5.1 en «Bloqueos»); `pub struct ValueReport { sales: u32, computed: u64, paid: u64, rel_error: f64 }`; `pub struct PredictionReport { bodies: u32, hit_high_or_medium: u32, ratio: f64, first_logged_accuracy: f64 }`; `pub struct ProbeReport { cases: u32, mean_abs_error: f64 }`; `pub fn eval_events(events: impl Iterator<Item = JournalEvent>) -> EvalReport` (puro; **reutiliza** las funciones públicas de los módulos — `build_jump_model`, `body_values`, `UnsoldLedger::apply_event`, `predict`, `data::star_flags` — sin duplicar su lógica; la predicción usa historial «hasta ese momento» para no hacer trampa).
  - CLI: `tmc-headless eval --journal-dir <carpeta> [--timings]` → imprime **solo** métricas agregadas (números, sin nombres de comandante ni sistemas) y `PASS`/`FAIL` por criterio (3: ≥ 5 loadouts, uno con ingeniería y uno con booster, todos ±0,05 al; 4: ±5 %; 6: ≥ 70 %). `--timings` añade el tiempo de las consultas de historial (visitas, sin vender, estadística bio).

- [ ] **Step 1: Tests:**

```rust
#[test] fn eval_report_contains_no_names() { /* salida de eval sobre exploration_session.log no contiene "CMDR TESTER" ni nombres de sistemas del fixture */ }
#[test] fn range_eval_counts_engineered_and_guardian_cases() { /* 3 Loadout sintéticos (1 con Engineering, 1 con booster Guardian) → engineered_cases==1, guardian_cases==1, cases==3 */ }
#[test] fn full_tank_and_max_fuel_hits_are_computed_independently() { /* Loadout con depósito de 32 t y MaxJumpRange = alcance recortado a 5 t → max_fuel_hits==1, full_tank_hits==0 */ }
#[test] fn value_eval_matches_fixture_sale_within_5_percent() { /* fixture con TotalEarnings 1_000_000 y valores calculados → rel_error <= 0.05 */ }
#[test] fn prediction_eval_uses_only_past_history() { /* 2 cuerpos con la misma clave: el 1.º se evalúa con historial vacío (solo reglas); el 2.º ve 1 caso del 1.º; hit counts == [valores fijos del test] */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Ejecutar en local** sobre los diarios del propietario: `cargo run -p tmc-headless -- eval --journal-dir "%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous" --timings`; guardar la salida (agregada) como evidencia en este plan («Verificación F1»). Medir también `--timings` (visitas, sin vender, estadística bio): si alguna consulta > 50 ms, anotar en «Desviaciones» la tabla de resumen (spec §7; medición movida desde Task 22). Ajustar `probes.json` y, si hace falta, la heurística ×5 y `bio_rules.json` (cada cambio con su fuente).
- [ ] **Step 6: Commit** `feat(headless): evaluación local de alcance, valores y predicción`

### Task 30: Fixtures anonimizados y pruebas de extremo a extremo (criterios 2 y 5)

**Files:** Create `tests/fixtures/journal/real_explo_01.log`, `real_exobio_01.log` (+ `status_*.json` si hacen falta); Modify `crates/tmc-app/src/lib.rs` (tests), `crates/tmc-model` (snapshot `insta` de los nuevos fixtures si aplica).

- [ ] **Step 1:** Extraer de los diarios del propietario una sesión de exploración (saltos con `NavRoute`, FSS, DSS, venta) y una de exobiología (aterrizaje, 3 muestras, venta) con `tmc-replay anonymize`; revisar a mano (`grep -i` del nombre real del comandante, FID, nave, chats) y dejar solo lo necesario.
- [ ] **Step 2: Tests:**

```rust
#[tokio::test] async fn route_strip_updates_within_500ms_p95() { /* start() con tempdir, servicios desactivados; escribir 20 FSDJump en vivo; medir línea escrita → WS update de navigation.route; p95 < 500 ms; remote.status "disabled" sin errores */ }
#[tokio::test] async fn strip_shows_remote_when_fake_service_answers() { /* NetClient con FakeTransport → remote.status "ok" */ }
#[tokio::test] async fn exobiology_fixture_end_to_end() { /* real_exobio_01: panel Active al aterrizar, candidatos no vacíos, 3 muestras con distancias ±1 m frente a haversine de referencia, unsold == SellOrganicData */ }
```

- [ ] **Step 3:** FAIL → implementar lo que falte → PASS; Checks.
- [ ] **Step 4: Commit** `test: fixtures anonimizados de exploración y exobiología y pruebas de extremo a extremo`

### Task 31: Disposición «Exploración» por defecto y ajustes nuevos en la UI

**Files:** Modify `ui/src/shell/layouts.ts`, `ui/src/App.svelte`, `ui/src/shell/Sidebar.svelte`, `ui/src/settings/Settings.svelte`, `ui/tests/{layouts.test.ts, dashboard.test.ts}`, `ui/tests/e2e/smoke.spec.ts`, `locales/*/main.ftl`.

**Interfaces:**
- Produces:
  - `layouts` = `[{ id: 'exploration', titleKey: 'layout-exploration', icon: 'compass', slots: [RouteStrip (full), contexto (span 2: SystemBodiesPanel o ExobiologyPanel según `contextPanel`), PlannerPanel (1), `JumpPanel` de alcance/combustible (1, Task 18)] }]`; la barra lateral muestra las disposiciones antes de los módulos; botones manuales «Cuerpos»/«Exobiología» fijan `manual` hasta el siguiente cambio de cuerpo.
  - Ajustes: «Consultar servicios online (Spansh, EDSM, Canonn)» (por defecto sí, texto de ayuda con qué se envía), copia automática (existente), umbral de «Mapear» (número, cr), eficiencia por defecto del planificador (0–100). Guardan con `PUT /api/config` (Task 11 los propaga en caliente).

- [ ] **Step 1: Tests:**

```ts
test('exploration layout puts route strip first at full width', ...)
test('context slot switches to exobiology when it becomes active', ...)
test('settings save online services, map threshold and efficiency', ...)
```
`smoke.spec.ts`: con `tmc-replay play` de `real_explo_01.log`, la disposición «Exploración» muestra la tira y el panel de cuerpos, sin errores de consola.

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS (`npm test`, `npx playwright test`); Checks.
- [ ] **Step 5: Commit** `feat(ui): disposición Exploración por defecto y ajustes de F1`

### Task 32: Consumo, verificación de los 10 criterios y cierre de F1

**Files:** Modify `THIRD_PARTY_NOTICES.md`, `README.md` (servicios online, eval), este plan («Verificación F1»), `status.md`.

- [ ] **Step 1:** `cargo tauri build` y `pwsh scripts/measure-footprint.ps1 -Exe target/release/TMC.exe -JournalDir <carpeta> -Seconds 120` con la disposición «Exploración» visible y `real_explo_01.log` → `PASS` (< 150 MB, < 1 % CPU en reposo). Si falla, no se cierra F1: anotar números en «Bloqueos».
- [ ] **Step 2:** `THIRD_PARTY_NOTICES.md`: `reqwest`, `rustls` y transitivas con licencia propia (re-verificar con `cargo deny check licenses` o `cargo metadata` que no entra GPL/AGPL/CC-BY(-SA) sin aprobación); fuentes y licencias de `fsd.json`, `species.json`, `bio_rules.json`, `probes.json` (según Task 1). Comprobar que no hay datos GPL/AGPL (`grep -ri srvsurvey crates/ ui/ locales/` sin coincidencias en datos).
- [ ] **Step 3:** Verificar los criterios §10.1–§10.10 uno a uno con evidencia (comando + salida) en «Verificación F1»: 1 Checks + CI; 2 Task 30 (`route_strip_updates_within_500ms_p95`) + replay manual con servicios on/off; 3/4/6 salida de `tmc-headless eval`; 5 `exobiology_fixture_end_to_end`; 7 ruta real de neutrones con Spansh aceptada y copia automática tras 2 saltos (manual, con el juego o replay); 8 test de claves; 9 Step 1; 10 Step 2.
- [ ] **Step 4:** Revisión completa de la rama contra la spec; registrar desviaciones conscientes; actualizar `status.md` (F1 ✅, siguiente: spec de F2).
- [ ] **Step 5: Commit** `chore: verificación y cierre de F1`

---

## Auto-revisión (cobertura de la spec)

| Spec | Tareas |
|---|---|
| §3.1 Disposición (ancho completo, panel de contexto, relevancia, tipografía) | 15, 18, 21, 27, 31 |
| §3.2 Tira de ruta (4 columnas, 3 siguientes, origen plan/NavRoute/actual, reglas de avance, «sin datos», combustible) | 13, 14, 15, 18 |
| §3.3 Cuerpos del sistema (cabecera, tabla, consejo, umbral, sondas, pie) | 20, 21, 22, 23 |
| §3.4 Exobiología (cabecera, candidatos, DSS, muestreo, dirección, valor) | 24–28 |
| §3.5 Planificador (formulario, resultado, aceptar, copia automática, persistencia, validación visual) | 16, 17, 19 |
| §4 Arquitectura (`tmc-net`, `ctx.request`, `NetService`, correlación/agrupación, 3 módulos, datos, eventos) | 4–7, 9–11, 13–27 |
| §5.1 Alcance y combustible (tabla, ingeniería, booster, validación ±0,05, respaldo, supercarga) | 13, 15, 29 |
| §5.2 Valor de cuerpos (fórmulas, sondas, sin vender, ±5 %) | 20, 22, 29 |
| §5.3 Exobiología (tabla, reglas, historial, Canonn, origen, «desconocida», máquina de estados, haversine) | 24–27 |
| §5.4 Bonus ×5 estimado | 27, 29 |
| §6 Servicios (límites, caché, opt-out, degradación, Legacy, spikes) | 0, 2, 9–11, 15 |
| §7 Persistencia (`nav_route_plan`, `exo_sample`, derivados de `event`, 50 ms) | 15, 16, 22, 26, 29 |
| §8 Protocolo, i18n, ajustes (topics, acciones, autocompletado REST, `.ftl`, ajustes) | 8, 11, 12, 15–17, 21, 22, 27, 31 |
| §9 Rendimiento y robustez (virtualización, carga única, respuestas corruptas, actor no bloqueado) | 7, 10, 13, 21, 23, 32 |
| §10 Criterios 1–10 | 29, 30, 32 (y Checks en cada tarea) |
| §11 Riesgos y spikes | 0, 1, 2, 3 |

Consistencia de nombres revisada: `NetQuery`/`NetRequest`/`NetResponse`/`NetOutcome`/`NetSource` (Task 6) usados igual en 7, 9–11, 15, 16, 25, 27; `ModuleSettings` (6) en 11, 15, 16, 21, 27; `StoreTask`/`store_task` (7) en 15, 16, 22, 25, 26; `JumpModel`/`RangeSource` (13) en 15, 16, 29; `ActivePlan`/`RouteSource` (14) en 15–17; `Candidate`/`Likelihood`/`ProbSource` (25) en 27–29.

Interfaces a confirmar al ejecutar (verificar en código al ejecutar): nombre de la feature TLS de `reqwest` en la versión fijada; tipo exacto de `Modifier.Value` no numérico en el `Loadout` real; `SellExplorationData.Systems` frente a `Discovered` en diarios actuales; campos de Spansh/Canonn (dependen de los spikes).

## Verificación F1

(Se rellena en las Tasks 29 y 32.)

## Desviaciones

- 2026-10-18 · Task 22: conciliación interna por comandante/generación y archivo/línea; checkpoints a 256 eventos/60 s y retry a 60 s para tareas SQLite sin respuesta. El libro JSON interno representa claves tupla como filas. Si SQLite falla persistentemente se conserva la cola para evitar perder saldo; la poda de eventos confirmados se verifica con DB operativa. API pública y topic de tres campos intactos; sin dependencias nuevas.

- 2026-10-18 · Task 22: el plan asumía que id de SQLite representaba cronología; watcher e importación pueden insertar fuera de orden. Para cumplir spec §5.2 se usa (ts, nombre de diario, línea, id) con paginación de 256 y corte estable id <= cut. 	s ya se guarda normalizado a UTC con nanosegundos por writer/import. Regresión observada RED (5 PASS/2 FAIL) → GREEN 7/7 history: muerte antigua insertada después del Scan y empates de segundo insertados en orden inverso. Sin cambio de API pública ni alcance.

(Anotar aquí cualquier cambio respecto a este plan o a la spec, con fecha, motivo y tareas afectadas.)

- 2026-10-17 · Propietario: `eval` (spec F1 §10.1) se implementa como `tmc-headless eval` en lugar de `tmc-replay eval`, para no cambiar las dependencias de §3.1. Afecta a Tasks 3, 20, 22, 29 y 32. Spec §10.1 actualizada.
- 2026-10-17 · Task 24 (pendiente de aprobación del propietario): spec §8 pide tabla de nombres de especies por idioma; F1 usa `_Localised` del diario y el nombre latino como respaldo; la tabla propia por idioma queda aplazada. Afecta a Tasks 24, 25 y 28.
- 2026-10-17 · Spike 1 (Spansh): política y Powerplay vienen de Spansh (`/api/system/{id64}`); «descubierto» no está en Spansh → EDSM; `PlannedHop` gana `jumps: u32` y `refuel: Option<bool>`; el planificador de galaxia necesita `fuel_power`/`fuel_multiplier` de `fsd.json` (FSD sin tabla → modo Galaxia desactivado). Afecta a Tasks 6, 10, 13, 15, 16, 19.
- 2026-10-17 · Spike 2 (licencias): `fsd.json`, `species.json`, `bio_rules.json` y `probes.json` se construyen desde hechos con procedencia por entrada y `verified`; ninguna fuente externa se incorpora. Afecta a Tasks 13, 20, 24, 25, 29.
- 2026-10-17 · Spike 3 (Canonn) — **pendiente del propietario**: la API es viable (`getSystemPoi`, `codex/ref`), pero su README exige solicitud formal de uso y pide enviar datos de usuarios. La capa 3 se implementa **desactivada** (`NetOutcome::Disabled`) hasta que el propietario decida; desvía la spec §5.3 («activada por defecto»). Afecta a Tasks 10, 25, 27, 31.
- 2026-10-17 · Preflight: aplicadas las resoluciones C1–C31 de la revisión previa (ver ledger de ejecución).


### Retoma y revisión de Task 21 (2026-10-18)

- Implementación existente: 9940901; no se repite su TDD original. La revisión detectó dos P2: payload de 300 cuerpos tras mapeo/señales/marcas (68.696 bytes en regresión RED) y marca Terraforming incoherente con fórmula Task 20. Correcciones TDD en curso.
- Verificación inicial: cargo fmt --all --check y cargo clippy --workspace --all-targets --offline -- -D warnings PASS; UI npm.cmd run check/test/build PASS (46 tests). Dos tests de rename fallan dentro del sandbox y pasan sin cambios con ejecución escalada: invalid_config_is_backed_up_and_defaulted y corrupt_database_is_renamed_and_rebuilt.
- Subpaso de verificación segura: el test preexistente config_host_roundtrips_and_validates_language se aísla con tempdir y AppActions::with_save_path para no modificar la configuración real al verificar fuera del sandbox; solo cambia el test, sin cambios de producción.

- Desviación de serialización Task 21 (2026-10-18): se compactan las seis marcas en marks y se omiten defaults estimated=false/advice=[not_worth], conservando filas e información. Interfaces Tasks 21/23 sincronizadas; la spec no fija claves JSON. Corpus finales medidos: base 53.172 bytes; mapeados mixtos 48.873; previamente mapeados mixtos 48.759; homogéneo con señales en 300 cuerpos 54.560 (RED anterior 68.960). Un test de 64 combinaciones verifica la decodificación completa de las marcas. Terraforming y Terraformable comparten el mismo predicado de marca/valor; Terraformed queda excluido.

- Cierre Task 21 (2026-10-18): revisión final aprobada; regresiones de payload y Terraforming RED→GREEN. cargo fmt --all --check, cargo clippy --workspace --all-targets --offline -- -D warnings, cargo test --workspace --offline --quiet y git diff --check: código 0, 392 tests Rust PASS (132 modules). UI npm.cmd --prefix ui run check/test/build: código 0, 46 tests PASS. Los tests de rename requieren ejecución escalada en este entorno; no son regresiones. Aislamiento del test de configuración registrado en 4e01e51. Topic máximo medido 54.560 bytes (<65.536), 300 filas intactas y 64 combinaciones de marks verificadas.
