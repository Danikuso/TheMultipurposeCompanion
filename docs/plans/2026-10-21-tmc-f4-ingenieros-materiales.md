# TMC F4 «Ingenieros y materiales» — Plan de implementación

> **Para agentes:** SUB-SKILL OBLIGATORIA: usar superpowers:subagent-driven-development (recomendado) o superpowers:executing-plans para ejecutar este plan tarea a tarea. Los pasos usan casillas (`- [ ]`) para el seguimiento. Lo marcado «(verificar en código al ejecutar)» es una interfaz existente que el redactor no confirmó: se comprueba antes de escribir el test y, si difiere, se adapta el nombre sin cambiar el comportamiento y se anota en «Desviaciones».

**Goal:** Que ingenierizar deje de ser una hoja de cálculo aparte: eliges un módulo de tu nave, blueprint, experimental y grado, y TMC calcula los materiales (por grado y tiradas), lo que tienes, los intercambios concretos con tus excedentes, el comerciante y el ingeniero más cercanos; mantiene una lista de la compra persistente y vigila el inventario (máximos, cambios de la sesión, taquilla de a pie).

**Architecture:** Se conserva la de F0–F3 (actor único dueño del estado, módulos sin E/S, difusión a 10 Hz, red vía `ModuleCtx::request` → `SourceEvent::Net`, SQLite vía `ModuleCtx::store_task` → `SourceEvent::Stored`, `core.clock` como base de tiempo, topics sin reloj publicados solo al cambiar). Nuevo: **un** módulo `engineering` (`crates/tmc-modules/src/engineering/` + `ui/src/modules/engineering/`) con piezas puras (`inventory`, `locker`, `engineers`, `ship`, `plan`, `trades`, `goals`, `sources`, `alerts`) y un `mod.rs` que las cablea. `tmc-model` tipa los eventos de materiales/ingenieros/taquilla; `tmc-net` amplía `NearbyService` (Ardent primero, Spansh de respaldo según el spike). Datos estáticos propios en `crates/tmc-modules/data/` (`materials.json`, `blueprints.json`, `engineers.json`, `trader_rules.json`) y nombres oficiales de blueprints/experimentales en `locales/*/engineering-data.ftl`.

**Tech Stack:** el de F3, sin dependencias nuevas previstas (si el spike A incorpora un dataset con licencia permisiva, se anota en `THIRD_PARTY_NOTICES.md` en la tarea que lo importa).

**Spec:** [`docs/specs/2026-10-21-tmc-f4-ingenieros-materiales-design.md`](../specs/2026-10-21-tmc-f4-ingenieros-materiales-design.md) (aprobada 2026-10-21; manda sobre este plan). Base: specs de arquitectura/F0, F1, F2, F3, [Auto-companion](../specs/2026-10-21-tmc-auto-companion-design.md) y [diagnóstico debug](../specs/2026-10-20-diagnostico-debug-design.md).

## Restricciones globales

- Código e identificadores en inglés; documentación, commits y textos de referencia (`es`) en español.
- Presupuesto (requisito): **< 150 MB** RAM del árbol de procesos y **< 1 %** CPU en reposo con la disposición «Ingeniería» visible; difusión **máx. 10 Hz**; latencia evento→UI **p95 < 500 ms**. Sin animaciones infinitas, sin `requestAnimationFrame`, sin `setInterval` (F4 no tiene cuentas atrás).
- **Todo test afirma un valor concreto; no hay tests vacíos.** Los `/* … */` describen entrada y valor esperado; el test real los escribe como datos y `assert_eq!`. Los `<valor>` de tests que dependen de los datos curados (`blueprints.json`) o de un fixture se sustituyen por el número concreto en el mismo commit en que se fija el dato. Literales de structs con todos los campos (`..Default::default()` solo si el tipo implementa `Default`).
- **Cada tarea compila y prueba lo suyo:** Task 12 añade `pub mod engineering;` a `crates/tmc-modules/src/lib.rs` y el `mod.rs` esqueleto; cada tarea siguiente declara su `mod x;`. **Cada tarea crea el fixture que usan sus tests**: Task 12 crea `tests/fixtures/journal/engineering_session.log` con **todos** los eventos que necesitan Tasks 12–20 (lista en Task 12); toda tarea posterior que lo amplíe lo lista en *Files* y actualiza en el mismo commit los valores de tests y del contrato que cambien. Task 27 usa su propio `engineering_eval.log`. Task 26 no cambia el fixture salvo para e2e (y entonces actualiza también los tests de UI 21–24).
- Tipos: variantes grandes en `Box` (clippy `large_enum_variant`). Eventos nuevos siguen el patrón de `classify` (`de(name, raw).map(...)`; lo que no deserializa cae en `Other`, nunca error) y los ayudantes tolerantes `lo`/`ld`/`lv` de `events.rs`: un campo mal formado nunca tira la línea entera.
- **Módulo `engineering`** (lecciones F1–F3, obligatorias):
  - nunca emite `NetQuery` ni `StoreTask` de **escritura** en replay ni con `Galaxy::Legacy`. Convención: toda `StoreTask` que escribe lleva tag terminado en `.write` (`engineering.goals.write`); las de lectura, no. Test de replay: 0 `NetRequest` y 0 tags `*.write`;
  - todo `SourceEvent::Stored` comprueba comandante (`value["cmdr"]`, patrón `stored_cmdr_ok`/`check_cmdr` de `missions`) antes de aplicarse; al **cambiar de comandante** se vacía todo el estado (inventario, taquilla, ingenieros, nave, flujo, objetivos, sesión, avisos, comerciantes) y se relee la lista de la compra del nuevo;
  - control de vuelo **solo** con `crate::inflight::Inflight` (`INFLIGHT_TTL_S = 60`, deduplicación, descarte de tardías); `Failed` se reintenta solo tras `NetClient::retry_at` (verificar en código al ejecutar); respuestas de un sistema anterior no se aplican;
  - un topic se marca sucio **solo si cambió** ignorando `as_of`, con `crate::topicx::set_topic` / `strip_as_of`; cada topic < **16 KiB** con test en el peor caso;
  - `relevance(&self, &GameState)` sin reloj: usa estado derivado de eventos (`last_ts` del último evento, contexto de taller) y los tests avanzan el tiempo con un evento posterior;
  - toda acción `engineering.*` se declara como `const ACTION_*` (lo escanea `registry::tests::every_module_topic_and_action_is_in_the_diagnostics_registry`) y el prefijo `"engineering."` se añade a `MODULE_PREFIXES` de `crates/tmc-app/src/clipboard.rs` (array de 7 → 8) con test de integración (**lección F2**: sin él la acción nunca llega al núcleo).
- **Diagnóstico (regla global, orden del propietario 2026-10-20):** cada topic, acción, módulo, evento del diario, valor de `NearbyService`/endpoint de red, ranura/panel/disposición de UI y campo de configuración nuevos se registra en `crates/tmc-model/src/diagnostics.rs` (`TOPICS`, `ACTIONS`, `MODULES`, `EVENTS`, `UI_LAYOUTS`, `UI_SLOTS`, `UI_PANELS`, claves públicas) o en `tmc-net` `endpoint_of` y `Config::diagnostic_snapshot` **en la misma tarea que lo añade**, con test canario (`engineering.CANARY` → `[redacted]`; nombre de comandante nunca presente). Instrumentación con `decide!`/`decide_if!` en entradas, decisiones, descartes y resultados; solo `debug_assertions`.
- Red: nunca con `Galaxy::Legacy` ni con «Consultar servicios online» desactivado. Ardent 1 petición/s y **caché 1 h** para comerciantes/broker; Spansh de respaldo con su límite de F1 y caché 1 h. Solo se envía el sistema actual (`system_address`), el tipo de servicio y la plataforma mínima: nada del comandante ni de su inventario. Sin servicios: «sin datos» y el resto funciona (distancias a ingenieros desde `engineers.json` y la posición).
- **Datos estáticos propios** (`materials.json`, `blueprints.json`, `engineers.json`, `trader_rules.json`): `source` y `verified` por entrada (o por bloque, si el spike lo justifica y se documenta), carga tolerante con `OnceLock` y recuperación por fila (`tracing::warn!`, nunca `panic!`), test de carga real y corrupta, patrón de `data.rs` de F3 (`rare_goods`). Nada de fuentes GPL/AGPL ni wikis CC BY-SA; lo no verificado lleva `verified:false` y se marca en la UI («sin verificar»). Todo dato importado (spike A) se anota en `THIRD_PARTY_NOTICES.md` en la tarea que lo incorpora.
- **Una sola fuente por concepto:** inventario de materiales **solo** con `MaterialInventory::apply_event` (en vivo, en `eval` y en tests); taquilla **solo** con `LockerInventory::apply_event`; coste de ingenierizar **solo** con `plan::plan_cost`; tasas **solo** con `trades::rate`; reservas y excedentes **solo** con `goals::reserve`.
- Privacidad: los spikes sobre diarios del propietario emiten **solo agregados** (recuentos, presencia de campos, sí/no); nunca nombres de comandante, FID, nombres de nave, chats ni nombres de sistemas privados. Los fixtures son sintéticos (`CMDR TESTER`).
- i18n: claves en `locales/{es,en,fr,de,pt}/engineering.ftl` (UI) y `engineering-data.ftl` (nombres oficiales de blueprints `bp-<clave>`, experimentales `exp-<clave>`, tipos de módulo `modtype-<tipo>`, requisitos de desbloqueo `unlock-<id>`), mismas claves en los 5 idiomas; nombre sin traducción oficial conocida = nombre inglés del juego **marcado** con el atributo Fluent `.pending = 1`. Nombres de materiales desde `_Localised` con respaldo `name_en` de `materials.json`. Toda variable Fluent nueva (`count`, `max`, `grade`, `rolls`, `percent`, `give`, `get`, `missing`, `distance`, `engineer`, `rank`, `goal`…) se añade a la lista fija de `embedded_resources_parse_and_format_all_messages_without_errors` (`crates/tmc-i18n/src/lib.rs`) en el mismo commit.
- UI: números, distancias y porcentajes **solo** con `ui/src/lib/format.ts`; acciones y `PUT /api/config` con JSON y `Content-Type: application/json`; el cambio automático de disposición solo con Auto-companion activado (`prefs.autoCompanion`).
- **Contrato JSON Rust→UI:** `ui/tests/fixtures/engineering-topics.json` se genera desde Rust con `UPDATE_CONTRACTS=1` (mismo mecanismo que `trade-topics.json`/`missions-topics.json`; verificar en código al ejecutar dónde vive el generador) y los tests de UI lo consumen; cambiar un topic exige regenerarlo en el mismo commit.
- Consumo: `scripts/measure-footprint.ps1` con un **directorio de destino aparte** (`CARGO_TARGET_DIR=target-f4`); **nunca** se sobrescribe `target/release/TMC.exe` (release de prueba del propietario).
- **Checks** antes de cada commit: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; si se toca `ui/`: `npm --prefix ui run check`, `npm --prefix ui test`, `npm --prefix ui run build`.
- Commits pequeños (uno por tarea) en español con prefijo convencional y `Co-Authored-By: <modelo> <noreply@anthropic.com>`. **Sin push.** `status.md` al empezar y terminar cada tarea.

## Foco de revisión

Fallos reales que la spec implica y que los tests «felices» no cubrirían. Cada uno tiene su test en la tarea dueña:

1. **Inventario que deriva** (cambios con la app cerrada, evento consumidor no tipado, recompensa de misión sin categoría, recogida por encima del máximo, material no catalogado, instantánea `Materials` nueva que debe mandar) → `snapshot_resyncs_after_drift`, `mission_reward_category_from_catalog`, `collected_beyond_cap_is_clamped_and_flagged`, `unknown_material_kept_with_journal_name` (Task 12), `eval_inventory_reconciles_on_fixture` (Task 27, fixture propio `engineering_eval.log`).
2. **Cálculo de ingenierizar engañoso** (módulo a medias en su grado, otro blueprint ya aplicado, objetivo menor que el actual, experimental ya aplicado, blueprint o módulo sin datos, receta `verified:false`) → `partial_grade_counts_remaining_rolls`, `other_blueprint_restarts_at_g1_with_warning`, `target_below_current_costs_nothing`, `same_experimental_not_counted_twice` (Task 16), `unknown_blueprint_is_no_data` (Task 20).
3. **Intercambios que roban o desbordan** (material reservado por otro objetivo, excedente del propio objetivo, recibir por encima del máximo del grado, cruzar tipo de comerciante, Guardian/Thargoid no intercambiables) → `reserved_by_other_goal_never_offered`, `trade_never_exceeds_grade_cap`, `never_crosses_trader_type`, `non_tradeable_materials_skipped`, `trade_never_exceeds_grade_cap` (Task 17), `reservations_follow_pinned_order`, `reserved_by_other_goal_never_offered` (Task 18).
4. **Avisos en ráfaga o falsos** (instantáneas de arranque, replay, `EngineerProgress` inicial con lista, material que ya estaba al máximo, objetivo que ya era completable) → `startup_snapshots_raise_no_alerts`, `alert_only_on_transition` (Task 19), `startup_engineer_list_does_not_activate_layout` (Task 20).
5. **Cambio de comandante, Legacy, replay y respuestas tardías** (lista de la compra de otro CMDR, comerciante del sistema anterior, peticiones en Legacy/replay) → `commander_switch_clears_and_reloads_goals`, `stored_goals_for_other_cmdr_ignored`, `late_trader_for_previous_system_not_applied`, `replay_emits_no_requests_or_store_writes`, `no_requests_in_legacy_or_offline` (Task 20).

## Resumen de tareas

| # | Tarea | Bloque |
|---|---|---|
| 0 | Spike de datos y licencias (A→B) y traducciones oficiales | 0 |
| 1 | Spike de diario — formatos en diarios del propietario (agregados) | 0 |
| 2 | Spike de red — Ardent comerciantes de materiales y broker; respaldo Spansh | 0 |
| 3 | Aprobaciones del propietario tras los spikes | 0 |
| 4 | `tmc-model` — eventos de materiales | 1 |
| 5 | `tmc-model` — eventos de ingenieros, `Loadout` ampliado, taquilla y mochila | 1 |
| 6 | `tmc-model` + `tmc-app` — servicios cercanos nuevos y tiradas por defecto en ajustes | 1 |
| 7 | `tmc-net` — comerciantes de materiales y broker (Ardent, respaldo Spansh) | 1 |
| 8 | Datos estáticos `materials.json` y `trader_rules.json` | 1 |
| 9 | Datos estáticos `blueprints.json` (todos los de módulos de nave) | 1 |
| 10 | Datos estáticos `engineers.json` | 1 |
| 11 | i18n — `engineering.ftl` y `engineering-data.ftl` (nombres oficiales) | 1 |
| 12 | `engineering` — inventario de materiales y cambios de la sesión | 2 |
| 13 | `engineering` — taquilla de a pie | 2 |
| 14 | `engineering` — ingenieros: estado, distancia y siguiente paso | 2 |
| 15 | `engineering` — mi nave modificada | 2 |
| 16 | `engineering` — cálculo de ingenierizar | 3 |
| 17 | `engineering` — planificador de intercambios | 3 |
| 18 | `engineering` — lista de la compra, reservas y persistencia | 3 |
| 19 | `engineering` — dónde conseguirlo, ingeniero recomendado y avisos | 3 |
| 20 | `engineering` — módulo, topics, relevancia, red, almacén y registro | 3 |
| 21 | UI — «Materiales» (inventario, taquilla, sesión) | 4 |
| 22 | UI — «Ingenieros» | 4 |
| 23 | UI — «Mi nave» e «Ingenierizar un módulo» | 4 |
| 24 | UI — «Lista de la compra», «Intercambios», «Dónde conseguirlo» y avisos | 4 |
| 25 | UI — disposición «Ingeniería», prioridad, Auto-companion y ajuste de tiradas | 5 |
| 26 | Fixtures sintéticos y pruebas de extremo a extremo | 5 |
| 27 | `tmc-headless eval` — conciliación del inventario de materiales | 5 |
| 28 | Avisos de terceros, consumo, verificación de los 10 criterios y cierre | 5 |

## Mapa de archivos

```
crates/tmc-model/src/{events.rs*, net.rs*, settings.rs*, diagnostics.rs*}
crates/tmc-net/src/{ardent.rs*, spansh.rs*, client.rs*, cache_policy.rs*}
crates/tmc-app/src/{config.rs*, clipboard.rs*, net_service.rs*, e2e_tests.rs*}
crates/tmc-modules/data/{materials.json+, trader_rules.json+, blueprints.json+, engineers.json+}
crates/tmc-modules/src/{lib.rs*, registry.rs*, data.rs*, eval.rs*}
crates/tmc-modules/src/engineering/{mod.rs+, inventory.rs+, locker.rs+, engineers.rs+, ship.rs+, plan.rs+, trades.rs+, goals.rs+, sources.rs+, alerts.rs+, tests.rs+}
crates/tmc-i18n/src/lib.rs*   apps/headless/src/eval.rs*
locales/{es,en,fr,de,pt}/{main.ftl*, engineering.ftl+, engineering-data.ftl+}
ui/src/modules/registry.ts*   ui/src/shell/layouts.ts*   ui/src/settings/Settings.svelte*
ui/src/modules/engineering/{index.ts+, types.ts+, labels.ts+, MaterialsPanel.svelte+, EngineersPanel.svelte+, ShipPanel.svelte+, EngineerFlow.svelte+, ShoppingPanel.svelte+, TradesPanel.svelte+, SourcesPanel.svelte+, EngineeringAlerts.svelte+}
ui/tests/{engineering.test.ts+, layouts.test.ts*, settings-f4.test.ts+, fixtures/engineering-topics.json+, e2e/engineering.spec.ts+}
tests/fixtures/journal/engineering_session.log+
tests/fixtures/net/{ardent_nearest_material_trader.json+, ardent_nearest_tech_broker.json+, spansh_stations_material_trader.json+ (si el spike lo usa)}
docs/research/notes/{spike_datos_f4.md+, spike_diario_f4.md+, spike_red_f4.md+}
THIRD_PARTY_NOTICES.md*
```
(`+` crear, `*` modificar.)

---

## Bloque 0 — Spikes y aprobaciones

Sin código de producción. Programas desechables en el scratchpad (nunca en el repo); peticiones reales ≤ 1/s. Las interfaces internas de los Bloques 1–5 **no cambian** por los spikes: solo cambian los adaptadores (`tmc_net::{ardent,spansh}`), el contenido de los JSON de datos y de `engineering-data.ftl`, y las constantes marcadas «del spike».

### Task 0: Spike de datos y licencias (A→B) y traducciones oficiales

**Files:** Create `docs/research/notes/spike_datos_f4.md`.

**Interfaces:** Produces la fuente de cada archivo de datos (Tasks 8–11), las claves internas del diario de blueprints/experimentales/tipos de módulo y la fuente de nombres oficiales es/fr/de/pt.

- [ ] **Step 1 (A):** Revisar datasets comunitarios con licencia de datos permisiva (MIT/BSD/Apache/CC0/CC BY sin SA): Coriolis (`coriolis-data`), EDEngineer, EDDB/EDCD (`FDevIDs`), Inara (solo consulta). Por cada uno: licencia exacta **del dato** (no solo del código), versión/commit, cobertura (blueprints de módulos de nave, grados 1–5, experimentales, materiales con grado/categoría/familia, ingenieros con sistema y grados, tasas del comerciante, pistas de obtención) y si incluye nombres localizados.
- [ ] **Step 2 (B):** Si ninguno es utilizable para un archivo: plan de recopilación propia como hechos del juego desde fuentes públicas (manual de diario de Frontier, notas de parche, el propio juego), sin copiar tablas. Excluidos siempre: GPL/AGPL (p. ej. SrvSurvey, EDMC plugins GPL) y wikis CC BY-SA.
- [ ] **Step 3 (traducciones):** Fuentes de los nombres oficiales del juego de blueprints y experimentales en es/fr/de/pt (archivos de localización publicados, `_Localised` del diario si aparece, capturas propias del juego): licencia, cobertura por idioma, procedimiento para marcar pendiente.
- [ ] **Step 4:** Documentar en la nota la decisión por archivo (`materials`, `blueprints`, `engineers`, `trader_rules`, hints, nombres por idioma), su licencia y el texto para `THIRD_PARTY_NOTICES.md` si es A.
- [ ] **Criterio de decisión:** A solo si la licencia del dato es compatible con PolyForm Noncommercial y no hay SA/copyleft; en otro caso B. Con duda → B y pregunta en Task 3.
- [ ] **Step 5: Commit** `docs: spike de datos y licencias para F4 (recetas, ingenieros, traducciones)`

### Task 1: Spike de diario — formatos en diarios del propietario

**Files:** Create `docs/research/notes/spike_diario_f4.md`.

**Interfaces:** Produces los campos reales de los eventos de Tasks 4–5 y las claves de blueprint/experimental para los fixtures.

- [ ] **Step 1:** Script desechable sobre `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous` (**solo lectura, solo agregados**): recuentos de `Materials`, `MaterialCollected`, `MaterialDiscarded`, `MaterialTrade`, `Synthesis`, `EngineerCraft`, `EngineerProgress` (con lista `Engineers` vs. actualización suelta), `EngineerContribution`, `TechnologyBroker`, `ShipLocker` (con contenido vs. stub + `ShipLocker.json`), `BackpackChange`, `MissionCompleted.MaterialsReward` (¿trae `Category`?); campos presentes por evento; valores distintos de `Progress`, `TraderType`, `Category`, `BrokerType`; presencia en `Loadout.Modules[].Engineering` de `Engineer`, `EngineerID`, `BlueprintID`, `BlueprintName`, `Level`, `Quality`, `ExperimentalEffect`, `ExperimentalEffect_Localised`; recuento de claves `BlueprintName`/`ExperimentalEffect` distintas (las claves son datos del juego, se pueden anotar).
- [ ] **Step 2:** ¿`Materials` es una instantánea completa en cada `LoadGame`? ¿Hay eventos que consumen materiales no listados en la spec (p. ej. `ScientificResearch`)? Completar con el manual del diario lo no visto («no visto / documentación»).
- [ ] **Criterio de decisión:** consumidores no listados → no se tipan en F4; la siguiente instantánea `Materials` resincroniza y se anota en «Desviaciones». Sin `Category` en `MaterialsReward` → categoría desde `materials.json`.
- [ ] **Step 3: Commit** `docs: spike de diario para F4 (materiales, ingenieros, taquilla; agregados)`

### Task 2: Spike de red — comerciantes de materiales y broker

**Files:** Create `docs/research/notes/spike_red_f4.md`, `tests/fixtures/net/{ardent_nearest_material_trader.json, ardent_nearest_tech_broker.json}` (sintéticos con la forma real) y, solo si se usa, `spansh_stations_material_trader.json`.

**Interfaces:** Produces el segmento de URL por servicio para `ardent::nearest_url` y, si hace falta, la consulta de respaldo de Spansh (Task 7).

- [ ] **Step 1:** Ardent `GET /v2/system/address/{addr}/nearest/{svc}?minLandingPadSize=` (patrón de F2 `interstellar-factors`): probar `material-trader`, `raw-material-trader`, `manufactured-material-trader`, `encoded-material-trader`, `technology-broker`, `guardian-technology-broker`, `human-technology-broker` (u otros nombres de la documentación de Ardent). Anotar si distingue tipo de comerciante y de broker, y la forma de la respuesta.
- [ ] **Step 2:** Si Ardent no distingue el tipo: comprobar Spansh búsqueda de estaciones (`POST /api/stations/search` con filtro de servicio «Material Trader»/«Technology Broker» y tipo, por cercanía a un sistema) y si la respuesta lo trae.
- [ ] **Criterio de decisión:** Ardent primero; si no ofrece un servicio, Spansh para ese servicio; si ninguno, «sin datos» para ese servicio y desviación documentada (spec §10.2.9). La interfaz `NetQuery::NearestService` no cambia.
- [ ] **Step 3: Commit** `docs: spike de red para F4 (comerciantes de materiales y broker)`

### Task 3: Aprobaciones del propietario tras los spikes

**Files:** Modify `status.md` («Bloqueos»), este plan («Desviaciones»).

- [ ] **Step 1:** Pedir aprobación solo de lo que cambie alcance o licencia: fuente A/B por archivo y su licencia, fuente de traducciones, servicios «sin datos», resoluciones del redactor (ver «Desviaciones»).
- [ ] **Step 2:** Tasks 4–7 avanzan mientras tanto. Tasks 8–10 escriben ya **el código** (cargadores, structs, tipos y tests) con un **conjunto provisional mínimo** de datos (`verified:false`, pocas entradas, siempre `Engine_Dirty` G1–G5 + Drag Drives del criterio 4 y los materiales que usan); solo el **contenido completo** de los JSON y de `engineering-data.ftl` espera la decisión del propietario sobre la fuente. Tasks 12–15 avanzan en cuanto está el código de 8–10. **Step 3: Commit** `docs(status): aprobaciones pendientes tras los spikes de F4`

---

## Bloque 1 — Eventos, red, ajustes, datos e i18n

### Task 4: `tmc-model` — eventos de materiales

**Files:** Modify `crates/tmc-model/src/events.rs`, `crates/tmc-model/src/diagnostics.rs` (`EVENTS`).

**Interfaces:**
- Consumes: `lo`/`ld`/`lv`, `MaterialReward` (F3).
- Produces (todo tolerante, `serde(default, rename_all = "PascalCase")`):
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)] enum MaterialCategory { Raw, Manufactured, Encoded }` con `fn from_journal(s: &str) -> Option<Self>` (acepta `"Raw"`, `"Manufactured"`, `"Encoded"` y `"$MICRORESOURCE_CATEGORY_Encoded;"`, sin distinguir mayúsculas).
  - `MaterialCount { name: String, name_localised: Option<String> /* falta en algunas entradas Raw */, count: u32 /* estricto: un Count no numérico invalida la entrada */ }` (listas con `lv`: la entrada inválida se omite); `Materials { raw: Vec<MaterialCount>, manufactured: Vec<..>, encoded: Vec<..> }`.
  - `MaterialCollected { category: String, name: String, name_localised: Option<String>, count: u32 }`; `MaterialDiscarded` (misma forma).
  - `MaterialTradeSide { material: String, material_localised: Option<String>, category: String, quantity: u32 }`; `MaterialTrade { market_id: Option<u64> (rename "MarketID"), trader_type: String, paid: MaterialTradeSide, received: MaterialTradeSide }`.
  - `MaterialUse { name: String, count: u32 }` (alias `"Material"`→`name` si el spike lo ve); `Synthesis { name: String, materials: Vec<MaterialUse> }`.
  - `pub fn material_key(name: &str) -> String` (minúsculas, sin espacios: clave única en todo F4).
  - `MaterialReward` (F3) + `category: Option<String>` (tolerante, `lo`) y `pub fn reward_category(r: &MaterialReward) -> Option<MaterialCategory>` (normaliza `$MICRORESOURCE_CATEGORY_<X>;` con `MaterialCategory::from_journal`; ausente o desconocida → `None` y Task 12 usa `materials.json`).
  - `EventKind` + `Materials(Box<Materials>)`, `MaterialCollected(..)`, `MaterialDiscarded(..)`, `MaterialTrade(Box<MaterialTrade>)`, `Synthesis(..)`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn materials_snapshot_parses_three_categories() { /* Raw [{Name:"iron",Count:120}], Manufactured [{Name:"conductiveceramics",Name_Localised:"Cerámica conductora",Count:30}], Encoded [{Name:"shieldpatternanalysis",Count:12}] → raw[0].count 120, manufactured[0].name_localised Some("Cerámica conductora") */ }
#[test] fn material_trade_parses_paid_and_received() { /* TraderType "encoded", Paid{Material:"scandatabanks",Category:"Encoded",Quantity:6}, Received{Material:"encodedscandata",Quantity:1} → paid.quantity 6, received.material "encodedscandata" */ }
#[test] fn malformed_material_entry_is_skipped_not_fatal() { /* Raw [{Name:"iron",Count:"x"},{Name:"nickel",Count:5}] → kind Materials, raw.len()==1, raw[0].name=="nickel" */ }
#[test] fn material_key_normalizes() { /* "Iron" → "iron"; " Conductive Ceramics " → "conductiveceramics" */ }
#[test] fn materials_reward_category_normalized() { /* Category "$MICRORESOURCE_CATEGORY_Manufactured;" → reward_category Some(Manufactured); sin Category → None */ }
#[test] fn category_from_journal_variants() { /* "Raw"→Raw, "$MICRORESOURCE_CATEGORY_Encoded;"→Encoded, "foo"→None */ }
#[test] fn synthesis_and_collected_parse() { /* Synthesis{Name:"FSD Basic",Materials:[{Name:"carbon",Count:1}]}; MaterialCollected{Category:"Raw",Name:"iron",Count:3} */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar; brazos nuevos en `match` exhaustivos sin cambiar comportamiento; nombres de evento en `diagnostics::EVENTS`. **Step 4:** PASS; Checks (si un snapshot `insta` cambia, revisar que solo añade lo nuevo).
- [ ] **Step 5: Commit** `feat(model): eventos de materiales tolerantes`

### Task 5: `tmc-model` — eventos de ingenieros, `Loadout` ampliado, taquilla y mochila

**Files:** Modify `crates/tmc-model/src/events.rs`, `crates/tmc-model/src/diagnostics.rs` (`EVENTS`).

**Interfaces:**
- Consumes: `Engineering`, `LoadoutModule`, `Loadout` (F1), unit variants `EventKind::ShipLocker`/`Backpack` y `CompanionKind::ShipLocker` (F0).
- Produces:
  - `Engineering` + `engineer: Option<String>`, `engineer_id: Option<u64> (rename "EngineerID")`, `blueprint_id: Option<u64> (rename "BlueprintID")`, `experimental_effect_localised: Option<String> (rename "ExperimentalEffect_Localised")` (aditivo; tests de F1 sin cambios).
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)] enum EngineerStatus { Known, Invited, Acquainted, Unlocked, Barred, Unknown }` con `fn from_journal(&str) -> Self`.
  - `EngineerEntry { engineer: String, engineer_id: u64, progress: EngineerStatus /* Unknown si falta Progress */, rank: Option<u8>, rank_progress: Option<u8> }`; `EngineerProgress { engineers: Vec<EngineerEntry> /* lista de arranque */, engineer: Option<String>, engineer_id: Option<u64>, progress: Option<String>, rank: Option<u8>, rank_progress: Option<u8> }` con `fn entries(&self) -> Vec<EngineerEntry>` (lista o la actualización suelta) y `fn is_startup_list(&self) -> bool`. Spike: en la actualización suelta **todos** los campos salvo `Engineer`/`EngineerID` son opcionales (puede traer solo `Rank` o solo `Progress`); sin `Progress` sale `Unknown` y el módulo conserva el estado previo de ese ingeniero.
  - `EngineerCraft { engineer, engineer_id, blueprint_name: String, blueprint_id: Option<u64>, level: u8, quality: Option<f64>, apply_experimental_effect: Option<String>, experimental_effect: Option<String>, ingredients: Vec<MaterialUse>, slot: Option<String>, module: Option<String> }`.
  - `EngineerContribution { engineer, engineer_id, type_: String (rename "Type"), commodity: Option<String>, material: Option<String>, quantity: u32, total_quantity: u32 }`.
  - `TechnologyBroker { broker_type: String, market_id: Option<u64>, commodities: Vec<MaterialUse>, materials: Vec<MaterialUse> }`.
  - `LockerItem { name: String, name_localised: Option<String>, count: u32, mission_id: Option<u64>, owner_id: Option<u64> }`; `ShipLockerContents { items, components, consumables, data: Vec<LockerItem> }` con `fn is_stub(&self) -> bool` y `fn from_value(v: &Value) -> Option<Self>` (para `ShipLocker.json`); `BackpackChange { added: Vec<LockerChange>, removed: Vec<LockerChange> }`, `LockerChange { name, name_localised, count: u32, type_: String /* Item|Component|Consumable|Data */ }`.
  - `EventKind::ShipLocker` pasa de unidad a `ShipLocker(Box<ShipLockerContents>)`. Stub (solo `timestamp` y `event`; 8 % en el spike): `is_stub() == true` y el consumidor **relee `ShipLocker.json`**; un stub nunca vacía la taquilla. `ExperimentalEffect`/`ApplyExperimentalEffect` no aparecen en los diarios del propietario: sus fixtures salen del manual del diario; + `EngineerProgress(Box<..>)`, `EngineerCraft(Box<..>)`, `EngineerContribution(..)`, `TechnologyBroker(..)`, `BackpackChange(..)`. Brazos que usaban `EventKind::ShipLocker` se actualizan (verificar en código al ejecutar: grep solo encontró `classify`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn engineer_progress_startup_list() { /* Engineers [{Engineer:"Felicity Farseer",EngineerID:300100,Progress:"Unlocked",Rank:5,RankProgress:0},{...,"Invited"}] → entries().len()==2, is_startup_list true */ }
#[test] fn engineer_progress_single_update() { /* {Engineer:"Elvira Martuuk",EngineerID:300160,Progress:"Unlocked"} → entries()==[Unlocked, rank None], is_startup_list false */ }
#[test] fn engineer_progress_loose_rank_only() { /* {Engineer:"Felicity Farseer",EngineerID:300100,Rank:4} → entries()[0].progress Unknown, rank Some(4) */ }
#[test] fn engineer_craft_parses_ingredients_and_experimental() { /* BlueprintName "Engine_Dirty", Level 5, Quality 0.42, ApplyExperimentalEffect "special_engine_overloaded", Ingredients [{Name:"pharmaceuticalisolators",Count:1}] → level 5, ingredients[0].count 1 */ }
#[test] fn loadout_engineering_extra_fields_are_additive() { /* módulo con Engineer/EngineerID/BlueprintID/ExperimentalEffect_Localised → campos Some; Loadout de F1 sin ellos → igual que antes */ }
#[test] fn ship_locker_stub_and_full() { /* {timestamp, event:"ShipLocker"} → ShipLocker(is_stub true); con Items [{Name:"healthpack"... Count:2}] → is_stub false, items.len()==1 */ }
#[test] fn backpack_change_added_and_removed() { /* Added [{Name:"graphene",Count:3,Type:"Component"}] → added[0].count 3 */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(model): eventos de ingenieros, taquilla y mochila; Loadout con ingeniero`

### Task 6: `tmc-model` + `tmc-app` — servicios cercanos nuevos y tiradas por defecto

**Files:** Modify `crates/tmc-model/src/{net.rs, settings.rs, diagnostics.rs}`, `crates/tmc-app/src/{config.rs, clipboard.rs}` (validación en el `PUT`, junto a `validate_f2`/`validate_f3`).

**Interfaces:**
- Produces:
  - `NearbyService` + `MaterialTrader(Option<TraderKind>)`, `TechnologyBroker(Option<BrokerKind>)` (`None` = cualquier tipo → Ardent; `Some` = tipo concreto → Spansh, spike de red); `enum TraderKind { Raw, Manufactured, Encoded }` (con `From<MaterialCategory>`), `enum BrokerKind { Human, Guardian }` (en `tmc-model::net`, ambos `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]`). `NetQuery::NearestService` + `system_name: String` (Spansh busca por nombre; los usos de F2 pasan `state.location.system`). Los `match` exhaustivos sobre `NearbyService` (`ardent::nearest_url`, `cache_policy::cache_keys`, `client.rs`) reciben aquí brazos que responden `NetOutcome::NoData` (ningún módulo los pide todavía); Task 7 los sustituye por los adaptadores reales.
  - `ModuleSettings` + `engineering_rolls: [u8; 5]` (por defecto `[1, 1, 1, 1, 4]`, `serde(default)`); `Config` + `engineering_rolls` copiado a `ModuleSettings`; `Config::validate_f4(&self) -> Result<(), &'static str>` (cada valor `1..=10`, si no `"invalid-engineering-rolls"`); `diagnostic_snapshot` + `"engineering_rolls"`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn module_settings_f4_defaults() { /* engineering_rolls == [1,1,1,1,4] */ }
#[test] fn f3_config_without_f4_fields_loads_with_defaults() { /* TOML de F3 → engineering_rolls [1,1,1,1,4] */ }
#[test] fn put_config_rejects_rolls_out_of_range() { /* [1,1,1,1,0] y [1,1,1,1,11] → 400 "invalid-engineering-rolls" y sin cambios */ }
#[test] fn trader_kind_from_category() { /* MaterialCategory::Encoded → TraderKind::Encoded */ }
#[test] fn config_snapshot_includes_engineering_rolls() { /* diagnostic_snapshot()["engineering_rolls"] == [1,1,1,1,4] */ }
#[test] fn nearby_service_variants_have_closed_diag_labels() { /* redact NetQuery {NearestService:{service:{MaterialTrader:"Raw"}}} conserva "Raw"; "CANARY" → [redacted]; system_name → [redacted] salvo la política pública de `system` */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(model): servicios de comerciante y broker; tiradas por defecto en ajustes`

### Task 7: `tmc-net` — comerciantes de materiales y broker

**Files:** Modify `crates/tmc-net/src/{ardent.rs, spansh.rs, client.rs, cache_policy.rs}`, `crates/tmc-app/src/net_service.rs` (caché 1 h), `crates/tmc-model/src/diagnostics.rs` (lista cerrada de etiquetas `endpoint`, ≈ línea 655); usa los fixtures de Task 2 (`ardent_nearest_material_trader.json`, `ardent_nearest_tech_broker.json`, `spansh_stations_material_trader.json`).

**Interfaces (según el spike de red):**
- Consumes: `ardent::{nearest_url, parse_stations}`, `RemoteStation`, `NetOutcome::Stations` (F2), `tmc_model::market::is_fleet_carrier`.
- Produces:
  - **Sin tipo** (`MaterialTrader(None)`, `TechnologyBroker(None)`): Ardent `nearest/material-trader` y `nearest/technology-broker` (segmentos exactos; Ardent no distingue tipos) con `parse_stations(body, min_pad)` (máx. `MAX_STATIONS = 5`, carriers fuera). Etiquetas: `"ardent.nearest.material_trader"`, `"ardent.nearest.technology_broker"`.
  - **Con tipo** (`MaterialTrader(Some(k))`, `TechnologyBroker(Some(b))`): Spansh `POST https://spansh.co.uk/api/stations/search` con cuerpo `{"filters":{"material_trader":{"value":["Raw"|"Manufactured"|"Encoded"]}},"sort":[{"distance":{"direction":"asc"}}],"size":5,"page":0,"reference_system":"<system_name>"}` (o `"technology_broker":{"value":["Guardian"|"Human"]}`): `pub fn spansh::station_search_body(reference_system: &str, service: NearbyService) -> serde_json::Value`, `pub fn spansh::parse_station_search(body: &str, min_pad: PadSize) -> Result<Vec<RemoteStation>, NetFailure>` (ignora campos no usados; plataforma máxima desde `large_pads`/`medium_pads`/`small_pads`; carriers fuera; `min_pad` re-comprobado en local). Etiquetas: `"spansh.stations.material_trader"`, `"spansh.stations.technology_broker"`.
  - Fallo de Spansh con tipo → `NetOutcome::NoData` (nunca se infiere el tipo por la economía ni se cae a Ardent sin tipo).
  - `cache_keys` distinto por servicio, tipo y plataforma; política **1 h** para los servicios nuevos (IF de F2 sin cambios); `endpoint_of` devuelve las cuatro etiquetas nuevas, añadidas a la lista cerrada de `diagnostics.rs` junto a `ardent.nearest.service`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn untyped_trader_uses_ardent_material_trader_segment() { /* MaterialTrader(None), addr 10477373803, Medium → "https://api.ardent-insight.com/v2/system/address/10477373803/nearest/material-trader?minLandingPadSize=2" */ }
#[test] fn typed_trader_request_is_spansh_station_search() { /* MaterialTrader(Some(Raw)), "Sol" → POST /api/stations/search; filters.material_trader.value == ["Raw"], size 5, sort distance asc, reference_system "Sol" */ }
#[test] fn typed_broker_request_is_spansh_station_search() { /* TechnologyBroker(Some(Guardian)) → filters.technology_broker.value == ["Guardian"] */ }
#[test] fn parses_spansh_station_search_fixture() { /* spansh_stations_material_trader.json → distancia asc; large_pads>0 → max_pad Some(Large); carrier fuera */ }
#[test] fn parses_ardent_trader_and_broker_fixtures() { /* ardent_nearest_material_trader.json / ardent_nearest_tech_broker.json → station_name y max_pad del fixture, len ≤ 5 */ }
#[test] fn spansh_failure_for_typed_service_is_no_data() { /* HTTP 500 en la consulta con tipo → NetOutcome::NoData */ }
#[test] fn cache_key_differs_per_service_kind_and_pad() { /* Some(Raw)/Medium ≠ Some(Encoded)/Medium ≠ Some(Raw)/Large ≠ None/Medium */ }
#[test] fn material_trader_cached_one_hour() { /* política: 3600 s */ }
#[test] fn new_endpoint_labels_are_closed_set() { /* las 4 etiquetas nuevas se conservan en NetOutcome; "spansh.stations.CANARY" → [redacted] */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(net): comerciantes de materiales y broker (Spansh con tipo, Ardent sin tipo) con caché de 1 h`

### Task 8: Datos estáticos `materials.json` y `trader_rules.json`

**Files:** Create `crates/tmc-modules/data/{materials.json, trader_rules.json}`; Modify `crates/tmc-modules/src/data.rs`, `THIRD_PARTY_NOTICES.md` (solo si la fuente es A).

**Contenido (orden de Task 3):** código + conjunto **provisional mínimo** `verified:false` (materiales de `engine_dirty`, `special_engine_overloaded` y `fsd_longrange`, los del fixture y una familia completa por tipo de comerciante). **Step 6** (tras la decisión del propietario): contenido completo en un commit aparte `feat(modules): catálogo completo de materiales` con `materials_full_catalogue_size` (número exacto del dato, ≈ 170).

**Interfaces:**
- Produces: `MaterialDef { key: String /* material_key */, name_en: String, category: MaterialCategory, grade: u8 /* 1..=5 */, family: String /* p. ej. "raw-1", "chemical", "emission-data" */, tradeable: bool, hints: Vec<Hint>, source: String, verified: bool }`, `Hint { kind: HintKind, detail_key: Option<String> /* clave ftl */ }`, `enum HintKind { Mining, SurfaceProspecting, Hge, SignalSource, DataScan, MissionReward, Salvage, Broker, Site }`; `fn materials() -> &'static HashMap<String, MaterialDef>`; `fn grade_cap(grade: u8) -> u32` = `[300, 250, 200, 150, 100][grade-1]` (constante del juego `GRADE_CAPS`).
  - `TraderRules { same_family_up: u32 /* 6 */, same_family_down: u32 /* 3 */, cross_family_factor: u32 /* 6, del spike */, source, verified }`; `fn trader_rules() -> &'static TraderRules`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn materials_load_with_provenance_and_valid_grades() { /* todas con source no vacío y grade 1..=5; "iron" → Raw grado 1, name_en "Iron"; "conductiveceramics" → Manufactured grado 3 (<del dato>) */ }
#[test] fn grade_caps_are_game_values() { /* grade_cap(1)==300 … grade_cap(5)==100 */ }
#[test] fn families_have_one_material_per_grade_where_tradeable() { /* cada familia intercambiable: grados distintos sin repetir */ }
#[test] fn trader_rules_default_rates() { /* same_family_up 6, same_family_down 3 */ }
#[test] fn corrupt_material_row_skipped_with_warning() { /* fila con grade 9 → omitida, resto cargado, sin pánico */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(modules): catálogo propio de materiales y reglas del comerciante con procedencia`

### Task 9: Datos estáticos `blueprints.json`

**Files:** Create `crates/tmc-modules/data/blueprints.json`; Modify `crates/tmc-modules/src/data.rs`, `THIRD_PARTY_NOTICES.md` (solo si A).

**Contenido (orden de Task 3):** código + conjunto provisional `verified:false`: `engine_dirty` G1–G5, `special_engine_overloaded` (Drag Drives), `fsd_longrange` G1–G5, tipos `engine` y `fsd`. **Step 6** (tras la decisión): todos los blueprints y experimentales de módulos de nave en un commit aparte `feat(modules): catálogo completo de blueprints`.

**Interfaces:**
- Produces: `ModuleTypeDef { id: String /* "engine", "fsd", "shield_generator"… */, item_prefixes: Vec<String> /* "int_engine_" */ }`; `Blueprint { key: String /* BlueprintName del diario, minúsculas */, module_types: Vec<String>, grades: BTreeMap<u8, Vec<Ingredient>> /* receta por tirada */, source, verified }`; `Ingredient { material: String, count: u32 }`; `Experimental { key: String /* ExperimentalEffect del diario, minúsculas */, module_types: Vec<String>, ingredients: Vec<Ingredient>, source, verified }`; `fn blueprints() -> &'static BlueprintCatalog` con `fn module_type_of(&self, item: &str) -> Option<&str>`, `fn blueprints_for(&self, module_type: &str) -> Vec<&Blueprint>`, `fn experimentals_for(&self, module_type: &str) -> Vec<&Experimental>`, `fn blueprint(&self, key: &str) -> Option<&Blueprint>`, `fn experimental(&self, key: &str) -> Option<&Experimental>`. **Todos** los blueprints y experimentales de módulos de nave (spec §10.1.2), `verified:false` salvo comprobación.

- [ ] **Step 1: Tests:**

```rust
#[test] fn every_ingredient_is_a_catalogued_material() { /* toda Ingredient.material ∈ materials() */ }
#[test] fn every_blueprint_has_contiguous_grades_from_1() { /* grados 1..=max sin huecos, max <= 5 */ }
#[test] fn thrusters_have_dirty_drive_and_drag_drives() { /* module_type_of("int_engine_size3_class5")=="engine"; blueprint("engine_dirty") con grados 1..=5; experimental("special_engine_overloaded") (clave del spike) para "engine" */ }
#[test] fn corrupt_blueprint_row_skipped_with_warning() { /* ingrediente con count 0 o grado 0 → fila omitida */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(modules): blueprints y experimentales de módulos de nave con procedencia`
- [ ] **Step 6 (tras Task 3):** contenido completo + `#[test] fn catalogue_covers_all_ship_module_types() { /* lista cerrada de tipos ⊆ tipos con algún blueprint; recuento == <n del dato> */ }`.

### Task 10: Datos estáticos `engineers.json`

**Files:** Create `crates/tmc-modules/data/engineers.json`; Modify `crates/tmc-modules/src/data.rs`, `THIRD_PARTY_NOTICES.md` (solo si A).

**Contenido (orden de Task 3):** código + conjunto provisional `verified:false` (Felicity Farseer 300100, Elvira Martuuk 300160, uno que aplique `engine_dirty` solo hasta G3 y uno a pie). **Step 6** (tras la decisión): todos, en un commit aparte, con `engineers_full_catalogue` (número exacto del dato).

**Interfaces:**
- Produces: `EngineerDef { id: u64 /* EngineerID */, name: String, system: String, star_pos: [f64; 3], workshop: String, market_id: Option<u64>, on_foot: bool, blueprints: Vec<EngineerBlueprint { blueprint: String, max_grade: u8 }>, unlock: UnlockSteps { requirement_key: String, invite_key: Option<String>, contribution: Option<Contribution { item: String, kind: String /* "commodity"|"material"|"credits" */, amount: u32 }> }, source, verified }`; `fn engineers() -> &'static Vec<EngineerDef>`; `fn engineer(id: u64) -> Option<&'static EngineerDef>`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn engineers_load_with_coords_and_provenance() { /* ≥ 1 a pie; todos con star_pos finito y source; 300100 → "Felicity Farseer", sistema "Deciat" */ }
#[test] fn engineer_blueprints_exist_in_catalogue() { /* todo blueprint de no a pie ∈ blueprints(); max_grade 1..=5 */ }
#[test] fn corrupt_engineer_row_skipped_with_warning() { /* star_pos no numérico → omitido */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(modules): ingenieros de nave y a pie con coordenadas y desbloqueo`

### Task 11: i18n — `engineering.ftl` y `engineering-data.ftl`

**Files:** Create `locales/{es,en,fr,de,pt}/{engineering.ftl, engineering-data.ftl}`; Modify `locales/*/main.ftl` (`layout-engineering`, ajuste `engineering-rolls`), `crates/tmc-i18n/src/lib.rs` (lista de argumentos y test de nombres).

**Interfaces:** `engineering.ftl`: títulos de paneles (`panel-engineering-materials`, `-locker`, `-engineers`, `-ship`, `-flow`, `-shopping`, `-trades`, `-sources`, `-alerts`), `nav-engineering`, columnas, estados (OK / con intercambio / falta N), «al máximo: lo que recojas se pierde», «de serie», «se sustituye el blueprint», «sin datos», «sin verificar», origenes de cambio, estados de ingeniero, avisos. `engineering-data.ftl`: `bp-<clave>`, `exp-<clave>`, `modtype-<tipo>`, `unlock-<id>`, `hint-<kind>` con nombre oficial del juego por idioma; si falta, inglés del juego con `.pending = 1`. Base para las tareas de UI (pueden añadir claves en los 5 idiomas en el mismo commit).

- [ ] **Step 1: Tests:** los existentes `all_languages_have_same_keys_as_es`, `keys_include_all_ftl_files` y el de formato con argumentos ampliados; nuevos:

```rust
#[test] fn every_catalogued_blueprint_and_experimental_has_a_name_in_all_languages() { /* ∀ clave de blueprints()/experimentals → bp-/exp- existe en es,en,fr,de,pt */ }
#[test] fn pending_names_are_marked_and_english() { /* en fr/de/pt/es, mensaje con .pending == valor en en */ }
#[test] fn unlock_keys_exist_in_engineering_data_ftl() { /* ∀ EngineerDef: unlock.requirement_key (e invite_key si hay) existen en los 5 idiomas */ }
#[test] fn every_module_type_has_modtype_name() { /* ∀ ModuleTypeDef.id → modtype-<id> en los 5 idiomas */ }
#[test] fn dirty_drive_spanish_name_is_official() { /* es bp-engine_dirty == <nombre oficial del spike, p. ej. «Motor sucio»> sin .pending */ }
```

- [ ] **Step 2–4:** FAIL → añadir → PASS; Checks. **Step 5: Commit** `feat(i18n): textos de ingeniería y nombres oficiales de blueprints en cinco idiomas`

---

## Bloque 2 — Inventario, taquilla, ingenieros y nave

### Task 12: `engineering` — inventario de materiales y cambios de la sesión

**Files:** Create `crates/tmc-modules/src/engineering/{mod.rs (esqueleto con mod inventory;), inventory.rs}` (tests unitarios en `#[cfg(test)] mod tests` de cada archivo; los del módulo en `engineering/tests.rs`, Task 20), `tests/fixtures/journal/engineering_session.log`; Modify `crates/tmc-modules/src/lib.rs` (`pub mod engineering;`).

**Fixture** (sintético, `CMDR TESTER`, sin FID real), en este orden: `Commander`, `Materials` (iron 120; un material G5 al máximo; uno no catalogado `foo_new`; codificados con excedente para un 6→1), `EngineerProgress` lista (Felicity 300100 Unlocked rango 5; Elvira 300160 Invited; uno Known), `LoadGame`, `Location` en un sistema sintético con `StarPos` = Deciat + [12.3, 0, 0], `Loadout` (ShipID 7; `MainEngines` de serie `int_engine_size5_class5`; `FrameShiftDrive` con `FSD_LongRange` L3 q 1.0), `ShipLocker` completo (graphene 5) y un stub, `BackpackChange` (+3 graphene), `MaterialCollected` ×3 (codificado +2; iron +3; uno por encima del máximo), `MaterialTrade` (paga iron 6, recibe 1), `Synthesis` (−1), `MissionCompleted` con `MaterialsReward` (`Category` `$MICRORESOURCE_CATEGORY_Manufactured;`), `FSDJump`, `Docked` en el taller de Felicity (`market_id` de `engineers.json`), `EngineerCraft` `FSD_LongRange` L4 q 1.0 en `FrameShiftDrive`, `EngineerProgress` suelto Elvira Invited→Unlocked, `Undocked`. Los valores finales (iron 117, graphene 8…) son los que afirman Tasks 12–24.

**Interfaces:**
- Consumes: eventos de Tasks 4–5, `MissionCompleted.materials_reward` (F3), `data::{materials, grade_cap}`.
- Produces: `MaterialInventory { fn apply_event(&mut self, ev: &EventKind, at: DateTime<Utc>) -> Vec<MaterialChange>; fn count(&self, key: &str) -> u32; fn at_max(&self) -> Vec<String>; fn by_category(&self) -> BTreeMap<MaterialCategory, BTreeMap<u8, Vec<MaterialLine>>>; fn has_snapshot(&self) -> bool }`. `Materials` sustituye todo (resincroniza); `MaterialCollected` suma con tope `grade_cap` (lo que excede se cuenta como `lost`); `MaterialDiscarded`, `Synthesis`, `EngineerCraft.ingredients`, `EngineerContribution` (tipo material), `TechnologyBroker.materials` restan (nunca bajo 0); `MaterialTrade` resta y suma; `MissionCompleted.materials_reward` suma (categoría de `reward_category`; si no, de `materials.json`). `MaterialLine { key, name_localised: Option<String>, name_en: Option<String> /* respaldo de materials.json, spec §7 */, grade: u8 /* 0 si no catalogado */, count: u32, max: u32, at_max: bool, known: bool }`; nombre mostrado: `name_localised` → `name_en` → clave del diario. `ChangeOrigin` deriva `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize`. `MaterialChange { key, delta: i32, origin: ChangeOrigin, lost: u32, at }`, `enum ChangeOrigin { Mission, Scan, Mining, Collected, Trade, Synthesis, Engineering, Discarded }` (`MaterialCollected`: Encoded→Scan, Raw→Mining, Manufactured→Collected). `SessionChanges { fn push(&mut self, c: &[MaterialChange]); fn totals(&self) -> BTreeMap<ChangeOrigin, (u32 /*in*/, u32 /*out*/)>; fn recent(&self) -> &[MaterialChange] /* últimos 50 */; fn reset(&mut self, at) }`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn snapshot_then_deltas() { /* Materials iron 120 → Collected iron 3 → Trade paid iron 6 → count("iron") == 117 (criterio 2) */ }
#[test] fn snapshot_resyncs_after_drift() { /* tras deltas a 117, Materials iron 50 → 50 */ }
#[test] fn collected_beyond_cap_is_clamped_and_flagged() { /* iron (G1, 300) en 299 + Collected 3 → 300, change.lost 2, at_max contiene "iron" */ }
#[test] fn mission_reward_category_from_catalog() { /* MissionCompleted MaterialsReward [{Name:"shieldpatternanalysis",Count:2}] sin Category → +2, origin Mission */ }
#[test] fn engineer_craft_consumes_ingredients_not_below_zero() { /* ingrediente con 1 y receta 3 → 0 */ }
#[test] fn unknown_material_kept_with_journal_name() { /* Materials con "foo_new" → línea known false, grade 0, count conservado, sin pánico */ }
#[test] fn name_en_fallback_when_no_localised() { /* Raw iron sin Name_Localised → name_localised None, name_en Some("Iron") */ }
#[test] fn session_totals_by_origin() { /* Collected encoded 2 + Trade + Synthesis → totals Scan (2,0), Trade (1,6), Synthesis (0,1) */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): inventario de materiales con instantánea, cambios y máximos`

### Task 13: `engineering` — taquilla de a pie

**Files:** Create `crates/tmc-modules/src/engineering/locker.rs`; Modify `engineering/mod.rs` (`mod locker;`).

**Interfaces:**
- Consumes: `ShipLockerContents`, `BackpackChange`, `CompanionFile::Raw(CompanionKind::ShipLocker, Value)`.
- Produces: `LockerInventory { fn apply_event(&mut self, ev: &EventKind); fn apply_file(&mut self, v: &Value); fn view(&self) -> LockerView }`: `ShipLocker` con contenido o `ShipLocker.json` sustituye; stub sin archivo no cambia nada; `BackpackChange` añade/resta en la sección de su `Type` (resincroniza la siguiente instantánea). `LockerView { items, components, consumables, data: Vec<LockerLine { key, name_localised, count }> }` ordenadas por nombre.

- [ ] **Step 1: Tests:**

```rust
#[test] fn locker_snapshot_from_event_or_file() { /* evento con Components graphene 5 → 5; stub + archivo con 7 → 7 */ }
#[test] fn backpack_change_adjusts_section() { /* Added graphene 3 (Component) → 8; Removed 10 → 0 */ }
#[test] fn stub_without_file_keeps_previous() { /* stub solo → sin cambios */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): taquilla de a pie desde ShipLocker y BackpackChange`

### Task 14: `engineering` — ingenieros

**Files:** Create `crates/tmc-modules/src/engineering/engineers.rs`; Modify `engineering/mod.rs`.

**Interfaces:**
- Consumes: `EngineerProgress::entries`, `data::{engineers, engineer}`, inventario (Task 12) y carga (`Cargo`) para el siguiente paso.
- Posición: `LocationState.star_pos` es `[f64; 3]` (no `Option`; por defecto `[0,0,0]` = Sol), así que el módulo guarda su `position: Option<[f64; 3]>`: `None` hasta el primer `Location`/`FSDJump`/`CarrierJump` y luego el `StarPos` de ese evento (`LocationInfo`; verificar en código al ejecutar el nombre del campo), mismo criterio de «posición conocida» que `missions::position`.
- Produces: `EngineerBook { fn apply(&mut self, p: &EngineerProgress) -> Vec<EngineerStatusChange>; fn status(&self, id: u64) -> Option<&EngineerEntry> }` (la lista de arranque sustituye sin generar cambios); `EngineerStatusChange { id: u64, from: Option<EngineerStatus>, to: EngineerStatus }`; `fn engineer_rows(book: &EngineerBook, pos: Option<[f64;3]>) -> Vec<EngineerRow>` con `EngineerRow { id, name, system, distance_ly: Option<f64>, status: EngineerStatus /* Unknown si no aparece */, rank: Option<u8>, rank_progress: Option<u8>, on_foot: bool, verified: bool }` ordenadas: nave antes que a pie, luego distancia (sin posición, por nombre); `fn next_step(def: &EngineerDef, entry: Option<&EngineerEntry>, have_material: &dyn Fn(&str) -> u32, have_commodity: &dyn Fn(&str) -> u32) -> NextStep` con `enum NextStep { Done, Requirement { key: String }, Invite { key: String }, Contribute { item: String, kind: String, have: u32, need: u32 } }`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn rows_from_fixture_progress() { /* engineering_session.log: Felicity Unlocked rank 5; Elvira Invited → status y rank (criterio 3) */ }
#[test] fn no_position_until_first_location() { /* sin Location/FSDJump → distance_ly None y orden por nombre; tras Location → Some */ }
#[test] fn distance_from_position() { /* pos [0,0,0], Deciat star_pos del json → distance_ly == norma (±0.01) */ }
#[test] fn startup_list_creates_no_changes() { /* apply(lista) → vec![] ; apply(suelta Invited→Unlocked) → 1 cambio from Some(Invited) */ }
#[test] fn next_step_contribution_uses_inventory() { /* contribución 10 de un material con 4 → Contribute{have:4, need:10} */ }
#[test] fn unknown_engineer_in_journal_ignored_without_panic() { /* EngineerID 999 → no aparece en rows, sin pánico */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): estado, rango, distancia y siguiente paso de los ingenieros`

### Task 15: `engineering` — mi nave modificada

**Files:** Create `crates/tmc-modules/src/engineering/ship.rs`; Modify `engineering/mod.rs`.

**Interfaces:**
- Consumes: `Loadout`/`LoadoutModule`/`Engineering` (F1 + Task 5), `EngineerCraft` (Task 5), `BlueprintCatalog::module_type_of`.
- Produces: `CurrentEng { blueprint: String, level: u8, quality: f64, experimental: Option<String> }`; `ShipModuleRow { slot: String, item: String, module_type: Option<String>, current: Option<CurrentEng> /* None = de serie */, engineerable: bool }`; `ShipView { ship_id: Option<u64>, ship: Option<String>, modules: Vec<ShipModuleRow> }` con `fn from_loadout(l: &Loadout) -> ShipView` y `fn apply_craft(&mut self, c: &EngineerCraft)` (actualiza el módulo de `slot` hasta el siguiente `Loadout`). Orden: armas, internos núcleo, opcionales (orden del `Loadout`). `ship_name`/`ship_ident` **no** se publican (privacidad/diagnóstico).

- [ ] **Step 1: Tests:**

```rust
#[test] fn stock_and_engineered_modules() { /* MainEngines int_engine_size5_class5 sin Engineering → current None, module_type "engine"; FSD con Engine.. Level 3 Quality 1.0 → current Some(level 3) */ }
#[test] fn craft_updates_slot_until_next_loadout() { /* EngineerCraft slot MainEngines Engine_Dirty L1 → current level 1 */ }
#[test] fn cosmetic_and_unknown_items_not_engineerable() { /* "paintjob_…", "decal_…" → engineerable false */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): vista de la nave con ingeniería actual por módulo`

---

## Bloque 3 — Ingenierizar, intercambios, lista de la compra, dónde conseguir, módulo

### Task 16: `engineering` — cálculo de ingenierizar

**Files:** Create `crates/tmc-modules/src/engineering/plan.rs`; Modify `engineering/mod.rs`.

**Interfaces:**
- Consumes: `Blueprint`, `Experimental`, `CurrentEng`, `ModuleSettings.engineering_rolls`.
- Produces: `PlanRequest { module_type: String, blueprint: String, experimental: Option<String>, target_grade: u8 /* por defecto 5 o el máximo del blueprint */, rolls: [u8; 5] }`; `fn start_grade(current: Option<&CurrentEng>, blueprint: &str, rolls: &[u8; 5]) -> StartPoint` con `StartPoint { grade: u8, partial_rolls: Option<u8>, replaces: bool }`; `fn plan_cost(bp: &Blueprint, exp: Option<&Experimental>, current: Option<&CurrentEng>, req: &PlanRequest) -> PlanCost`; `PlanCost { per_grade: BTreeMap<u8, (u8 /* tiradas */, BTreeMap<String, u32>)>, experimental: Option<BTreeMap<String, u32>>, total: BTreeMap<String, u32>, replaces_blueprint: bool, verified: bool }`.
- Semántica del diario: `Level` = grado actual (`Option<u32>` → `u8` acotado a 0..=5 en `CurrentEng`), `Quality` 0..1 = progreso dentro de ese grado; `quality >= 0.999` → grado completo.
- Reglas (spec §3.4.3 y §10.2.2–4): de serie → desde G1; mismo blueprint en G*n* con `quality >= 0.999` → desde G*n+1*; mismo blueprint en G*n* a medias → G*n* cuenta `max(1, ceil(rolls[n] × (1 − quality)))` tiradas y luego G*n+1*…; otro blueprint → desde G1 y `replaces_blueprint = true`; cada tirada consume la receta de su grado una vez; experimental una vez si difiere del actual; objetivo ≤ grado completo actual → sin grados; `verified` = AND de blueprint y experimental.

- [ ] **Step 1: Tests:**

```rust
#[test] fn stock_thrusters_dirty_g5_drag_drives_matches_manual() { /* criterio 4: total == oráculo calculado en el test desde el JSON crudo de blueprints.json (Σ g1..g4 receta×1 + g5 receta×4 + experimental), y == <tabla concreta del dato> */ }
#[test] fn same_blueprint_g3_complete_counts_from_g4() { /* criterio 4: current Engine_Dirty L3 q 1.0 → per_grade claves [4,5] */ }
#[test] fn partial_grade_counts_remaining_rolls() { /* L5 q 0.5, rolls g5 = 4 → per_grade {5: (2, …)} */ }
#[test] fn other_blueprint_restarts_at_g1_with_warning() { /* current Engine_Tuned L5 → per_grade claves [1..=5], replaces_blueprint true */ }
#[test] fn target_below_current_costs_nothing() { /* L5 q 1.0, objetivo 3 → per_grade vacío, total vacío */ }
#[test] fn same_experimental_not_counted_twice() { /* current experimental == pedido → experimental None */ }
#[test] fn custom_rolls_change_totals() { /* rolls [2,1,1,1,6] → g1 receta×2, g5 receta×6 */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): cálculo de materiales por grado y tiradas desde el grado actual`

### Task 17: `engineering` — planificador de intercambios

**Files:** Create `crates/tmc-modules/src/engineering/trades.rs`; Modify `engineering/mod.rs`.

**Interfaces:**
- Consumes: `data::{materials, trader_rules, grade_cap}`, `MaterialInventory::count`.
- Produces: `Rate { give: u32, get: u32 }`; `fn rate(from: &MaterialDef, to: &MaterialDef, rules: &TraderRules) -> Option<Rate>` (distinta categoría o no intercambiable → `None`; misma familia: subir *n* grados `6^n:1`, bajar *n* `1:3^n`; otra familia del mismo tipo: `cross_family_factor` aplicado sobre la regla de grado y fracción reducida; tasas confirmadas en el spike); `TradeProposal { give_key: String, give: u32, get_key: String, get: u32, trader: TraderKind, same_family: bool }`; `fn plan_trades(missing: &BTreeMap<String, u32>, surplus: &BTreeMap<String, u32>, have: &dyn Fn(&str) -> u32, rules: &TraderRules) -> TradePlan`; `TradePlan { proposals: Vec<TradeProposal>, uncovered: BTreeMap<String, u32> }`.
- Algoritmo (spec §10.2.5): faltantes en orden determinista (grado desc, luego clave); por cada uno, candidatos del mismo tipo con excedente suficiente para `trades = ceil(need / get)` (`give_total = trades × give`); primero misma familia; entre ellos el de menor `give_total`; empate → mayor excedente restante, luego clave; se descuenta del excedente; `have + trades × get` nunca supera `grade_cap` (si lo supera, se reduce `trades` y lo que falte queda en `uncovered`). Una sola fuente por faltante.

- [ ] **Step 1: Tests:**

```rust
#[test] fn same_family_up_six_to_one() { /* falta 1 de G3; excedente 12 del G2 de la familia → give 6, get 1 (criterio 5) */ }
#[test] fn same_family_down_one_to_three() { /* faltan 3 de G1; excedente G2 → give 1, get 3 */ }
#[test] fn cross_family_uses_penalty_from_rules() { /* misma categoría, otra familia, mismo grado → give 6 get 1 (valor de trader_rules) */ }
#[test] fn prefers_same_family_then_least_spent() { /* dos candidatos → misma familia aunque gaste más que otra familia; entre dos de la misma familia, el de menor give */ }
#[test] fn never_crosses_trader_type() { /* falta Encoded, solo excedente Raw → uncovered */ }
#[test] fn non_tradeable_materials_skipped() { /* tradeable false → nunca give ni get */ }
#[test] fn trade_never_exceeds_grade_cap() { /* G5 con 99/100 y faltan 3 → propuesta limitada, uncovered conserva el resto */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): planificador de intercambios con las tasas del comerciante`

### Task 18: `engineering` — lista de la compra, reservas y persistencia

**Files:** Create `crates/tmc-modules/src/engineering/goals.rs`; Modify `engineering/mod.rs`.

**Interfaces:**
- Consumes: `PlanRequest`, `plan_cost`, `plan_trades`, `Migration` (`tmc_core`), `StoreTask`, `Store::exec_json`/consultas JSON (verificar en código al ejecutar como en `missions::tourism`).
- Produces: `Goal { id: String /* "{ship_id}:{slot}:{blueprint}:{experimental|-}:{grade}" */, ship_id: Option<u64>, slot: String, item: String, request: PlanRequest, base: Option<CurrentEng> }`; `MAX_GOALS = 20`; `fn reserve(goals_needs: &[(String, BTreeMap<String, u32>)], have: &dyn Fn(&str) -> u32) -> Reservations` (en el orden de la lista: cada objetivo reserva `min(lo que queda, lo que necesita)`); `Reservations { by_goal: BTreeMap<String, BTreeMap<String, u32>>, fn surplus(&self, key: &str, have: u32) -> u32 /* have − Σ reservas de todos */ }`; `GoalView { id, percent: u8 /* Σ min(have_reservado, need) / Σ need */, missing: BTreeMap<String, u32>, trades: Vec<TradeProposal>, completable: bool /* missing vacío tras intercambios */ }`; `ShoppingView { goals: Vec<GoalView>, needs: BTreeMap<String, u32>, missing: BTreeMap<String, u32>, surplus: BTreeMap<String, u32> }`; `pub const ENGINEERING_MIGRATION: Migration` con id `"engineering-001"` (tabla `engineering_goals(cmdr TEXT, position INTEGER, id TEXT, json TEXT, PRIMARY KEY(cmdr, id))`); `fn goals_read_task(cmdr: String) -> StoreTask` (tag `engineering.goals`, lectura, devuelve `{cmdr, goals}`); `fn goals_write_task(cmdr: String, goals: Vec<Goal>) -> StoreTask` (tag `engineering.goals.write`, reemplaza todas las del comandante en una transacción).
- El flujo abierto (sin fijar) se evalúa como un objetivo más **al final** (no reserva antes que los fijados).
- `pin_goal` con un `id` ya presente **sustituye** ese objetivo en su posición; un 21.º distinto se rechaza y `ShoppingView.status = "too_many"` (si no, `"ok"`). `Goal`, `PlanRequest` y `CurrentEng` derivan `Serialize, Deserialize` (columna `json`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn reservations_follow_pinned_order() { /* 10 de X; A necesita 6, B 8 → A reserva 6, B 4; B missing X 4 */ }
#[test] fn reserved_by_other_goal_never_offered() { /* criterio 5: X reservado por A no aparece como give en las propuestas de B */ }
#[test] fn surplus_excludes_all_reservations() { /* have 30, reservas 12 → surplus 18 */ }
#[test] fn completable_when_trades_cover_missing() { /* missing cubierto por propuestas → completable true, percent < 100 */ }
#[test] fn goals_persist_across_restart() { /* criterio 7: Store en memoria + migración; write_task con 2 objetivos → read_task devuelve los 2 en orden */ }
#[test] fn reorder_changes_reservations() { /* mover B antes de A → B reserva primero */ }
#[test] fn max_goals_enforced() { /* el 21.º se rechaza con status "too_many" */ }
#[test] fn pin_same_id_replaces() { /* fijar dos veces "7:MainEngines:engine_dirty:special_engine_overloaded:5" → 1 objetivo en la misma posición; con grado 4 → 2 objetivos */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): lista de la compra con reservas por orden y persistencia`

### Task 19: `engineering` — dónde conseguirlo, ingeniero recomendado y avisos

**Files:** Create `crates/tmc-modules/src/engineering/{sources.rs, alerts.rs}`; Modify `engineering/mod.rs`.

**Interfaces:**
- Consumes: `RemoteStation`, `TraderKind`, `BrokerKind`, `MaterialDef.hints`, `EngineerDef`, `EngineerBook`, `ShoppingView`, `MaterialChange`, `EngineerStatusChange`.
- Produces:
  - `SourcesState { current_system: Option<u64>, fn set_current_system(&mut self, addr: Option<u64>) /* limpia resultados si cambia */; fn needed_traders(missing: &BTreeMap<String,u32>, proposals: &[TradeProposal]) -> BTreeSet<TraderKind>; fn on_stations(&mut self, system_address: u64, service: NearbyService, rows: Vec<RemoteStation>) -> bool /* false y sin cambios si system_address != current_system */; fn view(&self) -> SourcesView }`; `SourcesView { traders: BTreeMap<TraderKind, SourceStatus>, brokers: BTreeMap<BrokerKind, SourceStatus>, hints: BTreeMap<String, Vec<Hint>> }`, `enum SourceStatus { Loading, Ok(RemoteStation), NoData, Disabled }`. Peticiones: `MaterialTrader(Some(kind))` por tipo necesario; brokers con `TechnologyBroker(Some(..))`. Brokers solo si algún faltante tiene `HintKind::Broker`.
  - `fn pick_engineer(blueprint: &str, grade: u8, book: &EngineerBook, pos: Option<[f64;3]>) -> EngineerPick` con `EngineerPick { nearest_unlocked: Option<EngineerRow>, nearest_any: Option<EngineerRow>, next_step: Option<NextStep> /* del más cercano si no está desbloqueado */ }`.
  - `enum EngAlertKind { MaterialAtMax { key }, GoalCompletable { goal, engineer: Option<String> }, EngineerUnlocked { id }, EngineerInvited { id }, ScarceObtained { key, goal } }`; `fn alerts_from(changes: &[MaterialChange], at_max_before: &BTreeSet<String>, at_max_now: &BTreeSet<String>, completable_before: &BTreeSet<String>, view: &ShoppingView, status: &[EngineerStatusChange], missing_before: &BTreeMap<String,u32>) -> Vec<EngAlertKind>` (solo transiciones).

- [ ] **Step 1: Tests:**

```rust
#[test] fn traders_requested_only_for_needed_kinds() { /* missing solo Encoded → {Encoded} */ }
#[test] fn late_station_result_for_previous_system_rejected() { /* on_stations(addr viejo) → false y sin cambios */ }
#[test] fn pick_prefers_unlocked_and_reports_next_step_for_nearest_locked() { /* A desbloqueado a 50 al, B bloqueado a 10 al → nearest_unlocked A, nearest_any B, next_step Some(..) */ }
#[test] fn pick_respects_max_grade() { /* B solo hasta G3 y objetivo G5 → B excluido */ }
#[test] fn alert_only_on_transition() { /* iron ya al máximo antes → sin MaterialAtMax; pasa a máximo → 1 */ }
#[test] fn goal_completable_alert_names_nearby_engineer() { /* completable false→true → GoalCompletable{engineer: Some("Felicity Farseer")} */ }
#[test] fn scarce_material_obtained_alert() { /* faltaban 2 de X; MaterialCollected X 1 → ScarceObtained */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): comerciantes cercanos, ingeniero recomendado y avisos por transición`

### Task 20: `engineering` — módulo, topics, relevancia, red, almacén y registro

**Files:** Create `crates/tmc-modules/src/engineering/tests.rs`; Modify `engineering/mod.rs`, `crates/tmc-modules/src/registry.rs` (`Box::new(Engineering::new())`), `crates/tmc-app/src/clipboard.rs` (`MODULE_PREFIXES` + `"engineering."`), `crates/tmc-model/src/diagnostics.rs` (`TOPICS`, `MODULES`, `ACTIONS`, claves públicas de los topics nuevos), generador del contrato y `ui/tests/fixtures/engineering-topics.json` (con `UPDATE_CONTRACTS=1`).

**Interfaces:**
- Produces: `pub struct Engineering` (`Engineering::new()`, `id() = "engineering"`, `migrations() = &[ENGINEERING_MIGRATION]`).
  - Topics (todos con `as_of`, sin reloj): `engineering.inventory` (`by_category` solo con count > 0 y no catalogados, `at_max`), `engineering.locker`, `engineering.session` (`totals` + `recent`), `engineering.engineers` (`Vec<EngineerRow>` + `selected: Option<{id, next_step}>`), `engineering.ship` (`ShipView`), `engineering.flow` (`{status: "idle"|"ok"|"no_data", slot, options: {blueprints, experimentals} , request, cost: PlanCost, lines: Vec<FlowLine { key, need, have, state: "ok"|"trade"|"missing", missing: u32, trade: Option<TradeProposal> }>, percent, engineer: EngineerPick}`), `engineering.goals` (`ShoppingView`), `engineering.sources` (`SourcesView`), `engineering.alerts` (≤ 20, descartables).
  - Acciones (`const ACTION_*`): `engineering.select_module { slot }`, `engineering.preview { blueprint, experimental, grade, rolls? }`, `engineering.close_flow`, `engineering.pin_goal {}` (fija lo del flujo), `engineering.unpin_goal { id }`, `engineering.move_goal { id, to }`, `engineering.select_engineer { id }`, `engineering.dismiss_alert { index }`, `engineering.reset_session`. Argumentos inválidos → sin cambios y `decide!` de descarte.
  - Red: `NearestService { system_address, service: MaterialTrader(k)|TechnologyBroker(b), min_pad }` con `min_pad` = plataforma de la nave (`data::ship_pads`, F2; desconocida → `Small`), clave `Inflight` `"trader:{k}:{addr}"`; se piden al cambiar de sistema o de faltantes, nunca en replay/Legacy/offline (estado `Disabled`).
  - Almacén: `goals_read_task` al primer comandante y tras cambio de comandante; `goals_write_task` tras `pin/unpin/move` (nunca en replay/Legacy).
  - Avisos: `ctx.alert` solo en vivo y solo con `alerts_enabled`; instantáneas (`Materials`, lista de `EngineerProgress`, `Loadout`) no generan avisos.
  - `relevance`: `Active` si contexto de taller (atracado con `market_id` o `(system, workshop)` de `engineers.json`; o el último evento fue `EngineerCraft`/`EngineerProgress` suelto/`EngineerContribution`), o el flujo está abierto; se limpia con `Undocked`/`FSDJump`/`close_flow`. Si no, `Normal`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn fixture_inventory_matches_snapshot_plus_changes() { /* criterio 2 vía módulo: engineering_session.log → engineering.inventory counts == <valores del fixture>, at_max contiene <clave> */ }
#[test] fn preview_flow_with_fixture_ship() { /* select_module MainEngines + preview engine_dirty G5 + experimental → engineering.flow.lines no vacías y percent == <valor> */ }
#[test] fn replay_emits_no_requests_or_store_writes() { /* replaying con pin_goal → 0 NetRequest, 0 tags "*.write" */ }
#[test] fn no_requests_in_legacy_or_offline() { /* Legacy → 0 y sources Disabled; offline → 0 */ }
#[test] fn fake_ardent_trader_shows_nearest() { /* criterio 6: Net Stations para Encoded del sistema actual → sources.traders.Encoded Ok(station) */ }
#[test] fn no_services_shows_no_data_and_rest_works() { /* online_services=false → traders Disabled, flow ok, engineers con distancia */ }
#[test] fn late_trader_for_previous_system_not_applied() { /* respuesta tras FSDJump → sin cambios */ }
#[test] fn commander_switch_clears_and_reloads_goals() { /* Commander "B" → goals vacío y store_task engineering.goals con cmdr "B" */ }
#[test] fn stored_goals_for_other_cmdr_ignored() { /* Stored{tag:"engineering.goals", cmdr:"OTRO"} → sin cambios */ }
#[test] fn goals_survive_restart_and_alert_completable() { /* criterio 7: pin, nuevo Engineering con la misma Store → goals igual; MaterialCollected que completa → alerta goal-completable */ }
#[test] fn startup_snapshots_raise_no_alerts() { /* LoadGame + Materials al máximo + lista EngineerProgress → 0 alertas */ }
#[test] fn startup_engineer_list_does_not_activate_layout() { /* lista de arranque → Normal; EngineerCraft → Active; Undocked → Normal */ }
#[test] fn relevance_active_docked_at_workshop() { /* Docked con market_id de engineers.json → Active */ }
#[test] fn engineering_topics_stay_under_16_kib() { /* catálogo entero al máximo con nombres localizados, 40 ingenieros, 60 módulos, 20 objetivos, flujo con 30 líneas → cada topic < 16384 */ }
#[test] fn topics_dirty_only_on_change() { /* mismo Materials dos veces → engineering.inventory sucio solo la primera */ }
#[test] fn unknown_blueprint_is_no_data() { /* preview con blueprint "nope" → engineering.flow.status "no_data", sin cost */ }
#[test] fn ui_contract_snapshot_is_up_to_date() { /* patrón de missions/trade: contract_snapshot() del escenario de abajo; con UPDATE_CONTRACTS=1 reescribe ui/tests/fixtures/engineering-topics.json; si no, compara (CRLF normalizado) */ }
```
  **Escenario del contrato** (`fn contract_snapshot() -> Value` en `engineering/tests.rs`): `engineering_session.log` en vivo (no replay, online) + `Net` falsa de Spansh para `MaterialTrader(Some(Encoded))` con «Fake Port» a 8,2 al y plataforma L + acciones `pin_goal` del FSD (`fsd_longrange`, G5, sin experimental, tiradas por defecto) y después `select_module MainEngines` + `preview { blueprint: "engine_dirty", experimental: "special_engine_overloaded", grade: 5 }`. Valores que fija y que afirman Tasks 21–24: `engineering.inventory` iron 117/300 en G1 y un material en `at_max`; `engineering.locker` graphene 8; `engineering.session` Scan +2, Trade +1/−6; `engineering.engineers` Felicity Unlocked rango 5 a 12,3 al; `engineering.goals` objetivo del FSD `completable: true` con `engineer: "Felicity Farseer"` y una propuesta 6→1; `engineering.flow` con al menos una línea `missing` 2 (el conjunto provisional de Tasks 8–9 y el fixture se ajustan para que así sea); `engineering.sources` Encoded `Ok(«Fake Port», 8.2, Large)`. Si un valor no sale con los datos, se ajusta el fixture (no los tests de UI) y se anota.
  más en `crates/tmc-app/src/clipboard.rs`: `engineering_actions_reach_core` (`engineering.reset_session` llega como `SourceEvent::Action`); y en `diagnostics.rs`: `engineering_topics_actions_and_module_registered_and_canary_redacted` (topics y acciones de arriba registrados; `engineering.CANARY` → `[redacted]`; `{"cmdr":"CMDR TESTER"}` no aparece). El test de `registry.rs` pasa con las acciones nuevas.

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(engineering): módulo de ingeniería con topics, relevancia, red, almacén y registro`

**Revisión de continuación (2026-10-21, Codex):** `9904133` es implementación parcial, no cierre. Correcciones con TDD: conservar operaciones de objetivos durante una lectura tardía; rechazar blueprints incompatibles y cerrar un flujo cuyo módulo se sustituye; invalidar estaciones y correlacionar respuestas al cambiar de plataforma; conservar reintentos del servicio común; diagnóstico con etiquetas finitas por ruta exacta y texto libre redactado. Contrato UI sin cambios. Revisión de spec del alcance corregido aprobada; el requisito de tamaño permanece bloqueado (véase «Desviaciones»).

Evidencia reproducible del bloqueo (ambos comandos devuelven 101, **no PASS**):

```text
cargo test -p tmc-modules worst_case_engine_goals_size -- --ignored --nocapture
engineering.goals: 41116 bytes; 20 objetivos, 215 intercambios
cargo test -p tmc-modules engineering_topics_stay_under_16_kib -- --ignored --nocapture
engineering.locker: 16832 B; 200 recursos localizados, 50 por sección
```

Estos dos probes se conservan ignorados con el incumplimiento explícito como motivo hasta resolver el contrato. El test de tamaño del escenario real del contrato sigue activo; pasar las demás pruebas no satisface el peor caso ni cierra esta tarea.

Verificación del commit parcial: `cargo fmt --all --check` y `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo test --workspace -- --test-threads=1` → **1.344 PASS / 4 ignoradas** (dos previas y los dos probes anteriores). Suite ejecutada con `TMP`/`TEMP` dentro de `.tmp/test-runtime` y permisos de sockets localhost fuera del sandbox; dentro del aislamiento ZeroMQ devuelve error 10013. El test del informe de arranque limpia su residuo global antes de preparar el caso, con RED→GREEN observado; la suite se serializa por el estado global compartido de esos tests. UI: `npm.cmd --prefix ui run check` (0 errores/avisos), `npm.cmd --prefix ui test -- --run` (**238 PASS**) y `npm.cmd --prefix ui run build` PASS. Revisiones independientes de spec y calidad aprueban el alcance parcial; **Task 20 no completa**. Sin nuevas dependencias, sin push y sin recompilar el release del propietario. La medición de consumo de F4 sigue en Task 28 tras resolver el contrato y completar la UI.

---

## Bloque 4 — UI

Todas las tareas de UI: tipos en `ui/src/modules/engineering/types.ts` derivados del contrato `engineering-topics.json`; tests con el contrato real; nombres de blueprint/experimental con `bp-`/`exp-` y marca visible si `.pending`; «sin verificar» si `verified:false`; formato con `format.ts`; claves nuevas en los 5 idiomas; `UI_PANELS`/`UI_SLOTS` de `diagnostics.rs` actualizados en la misma tarea (hay test de UI que los sincroniza).

### Task 21: UI — «Materiales» (inventario, taquilla, sesión)

**Files:** Create `ui/src/modules/engineering/{index.ts, types.ts, labels.ts, MaterialsPanel.svelte}`, `ui/tests/engineering.test.ts`; Modify `ui/src/modules/registry.ts` (`{ id: 'engineering', titleKey: 'nav-engineering', panels }`), `locales/*/engineering.ftl`, `crates/tmc-model/src/diagnostics.rs` (`UI_PANELS`).

**Interfaces:** pestañas Materias primas / Fabricados / Codificados / Taquilla; por grado (G1–G5) con cantidad/máximo (300/250/200/150/100) y aviso «al máximo»; cambios de la sesión por origen.

- [ ] **Step 1: Tests:**

```ts
test('materials grouped by grade with count over max', /* contrato: iron 117/300 en G1 */);
test('at max warning visible', /* at_max ["iron"] → engineering-at-max visible */);
test('unknown material shows journal name and no data', /* known false → nombre del diario + engineering-no-data */);
test('material name falls back to name_en', /* name_localised null, name_en "Iron" → "Iron" */);
test('locker tab lists components', /* graphene 8 */);
test('session totals by origin', /* Scan +2, Trade +1/−6 */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): panel de materiales con taquilla y cambios de la sesión`

### Task 22: UI — «Ingenieros»

**Files:** Create `ui/src/modules/engineering/EngineersPanel.svelte`; Modify `index.ts`, `locales/*/engineering.ftl`, `ui/tests/engineering.test.ts`, `diagnostics.rs` (`UI_PANELS`).

**Interfaces:** lista (nave y a pie) con estado, rango y progreso, sistema y distancia; al elegir → `engineering.select_engineer { id }` y siguiente paso con lo que tienes; blueprints y grado máximo del ingeniero.

- [ ] **Step 1: Tests:**

```ts
test('engineer row shows status rank and distance', /* Felicity Unlocked, rango 5, "12,3 al" (es) */);
test('select engineer posts json action', /* POST engineering.select_engineer {id:300100} */);
test('next step contribution shows have and need', /* 4/10 */);
test('on foot engineers in their own section', /* on_foot true → sección a pie */);
test('engineer blueprints with max grade', /* Felicity → nombre bp-engine_dirty y «G5» */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): panel de ingenieros con estado y siguiente paso`

### Task 23: UI — «Mi nave» e «Ingenierizar un módulo»

**Files:** Create `ui/src/modules/engineering/{ShipPanel.svelte, EngineerFlow.svelte}`; Modify `index.ts`, `locales/*/engineering.ftl`, `ui/tests/engineering.test.ts`, `diagnostics.rs`.

**Interfaces:** módulos con blueprint/grado/calidad/experimental o «de serie»; elegir → `engineering.select_module`; selector de blueprint y experimental (nombres oficiales), grado (G5 por defecto), tiradas por grado ajustables (por defecto de `engineering_rolls`) → `engineering.preview`; tabla por material (necesarios, tienes, estado OK / intercambio concreto / falta N), por grado y total, % listo; aviso de sustitución; ingeniero recomendado; «Añadir a la lista de la compra» → `engineering.pin_goal`; «Cerrar» → `engineering.close_flow`.

- [ ] **Step 1: Tests:**

```ts
test('stock module shows de serie', /* current null → engineering-stock */);
test('preview posts blueprint experimental grade and rolls', /* POST engineering.preview {blueprint:"engine_dirty",experimental:"special_engine_overloaded",grade:5,rolls:[1,1,1,1,4]} */);
test('lines show ok trade and missing states', /* contrato: 3 estados con "falta 2" */);
test('replace warning when other blueprint', /* replaces_blueprint → engineering-replaces */);
test('pending translation is marked', /* bp sin traducción → marca pending visible */);
test('pin goal posts action', /* POST engineering.pin_goal {} */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): mi nave y flujo de ingenierizar un módulo`

### Task 24: UI — «Lista de la compra», «Intercambios», «Dónde conseguirlo» y avisos

**Files:** Create `ui/src/modules/engineering/{ShoppingPanel.svelte, TradesPanel.svelte, SourcesPanel.svelte, EngineeringAlerts.svelte}`; Modify `index.ts`, `locales/*/engineering.ftl`, `ui/tests/engineering.test.ts`, `diagnostics.rs`.

**Interfaces:** objetivos con % y faltantes, subir/bajar (`engineering.move_goal`), quitar (`engineering.unpin_goal`), «ya puedes completar X» con ingeniero; intercambios das/recibes con comerciante cercano; dónde conseguir: comerciante por tipo con distancia y plataforma o «sin datos», broker si aplica, pistas por material; avisos descartables (`engineering.dismiss_alert`).

- [ ] **Step 1: Tests:**

```ts
test('goal shows percent and completable hint', /* completable → engineering-goal-completable con nombre de ingeniero */);
test('move goal up posts json', /* POST engineering.move_goal {id, to:0} */);
test('trade shows give and receive', /* "6 → 1" con nombres */);
test('no services shows no data per trader', /* Disabled → engineering-no-data */);
test('nearest trader shows distance and pad', /* "8,2 al", L */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): lista de la compra, intercambios, dónde conseguirlo y avisos`

---

## Bloque 5 — Disposición, ajustes, fixtures, eval y cierre

### Task 25: UI — disposición «Ingeniería», prioridad, Auto-companion y ajuste de tiradas

**Files:** Modify `ui/src/shell/layouts.ts`, `ui/src/settings/Settings.svelte`, `locales/*/main.ftl`, `ui/tests/layouts.test.ts`; Create `ui/tests/settings-f4.test.ts`; Modify `crates/tmc-model/src/diagnostics.rs` (`UI_LAYOUTS` + `"engineering"`, `UI_SLOTS`).

**Interfaces:** `layouts` + `{ id: 'engineering', titleKey: 'layout-engineering', icon: 'wrench', slots: [materials 2, engineers 1, eng-alerts 1, eng-ship 1, eng-flow 2 when flow.status != 'idle', shopping 2, trades 1, sources 1] }`; `AutoLayout`/`MANUAL` + `'engineering'`; `autoLayout`: manual > `combat` Active/Urgent > `mining` Active > **`engineering` Active/Urgent** > `missions` > `trade` > exploración; `nextPin` sin cambios; el cambio automático solo con `prefs.autoCompanion` (lógica existente de `App.svelte`, sin tocarla salvo el tipo). Ajustes: tiradas G1–G5 (1–10) → `PUT /api/config` JSON con `engineering_rolls`; error revierte.

- [ ] **Step 1: Tests:**

```ts
test('engineering beats missions', /* engineering active + missions active → 'engineering' */);
test('mining beats engineering', /* mining active + engineering active → 'mining' */);
test('combat beats engineering', /* combat urgent + engineering active → 'combat' */);
test('engineering beats trade and exploration', /* → 'engineering' */);
test('auto companion OFF keeps layout when engineering becomes active', /* OFF + engineering active → disposición sin cambio */);
test('engineering slots and flow visibility', /* flow idle → sin eng-flow; ok → visible */);
test('settings put rolls and reverts on 400', /* [1,1,1,1,11] → 400 → valor anterior */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): disposición de ingeniería con prioridad, Auto-companion y tiradas`

### Task 26: Fixtures sintéticos y pruebas de extremo a extremo

**Files:** usa `tests/fixtures/journal/engineering_session.log` (Task 12) y los fixtures de red (Task 2); Create `ui/tests/e2e/engineering.spec.ts`; Modify `crates/tmc-app/src/e2e_tests.rs`. Si hubiera que tocar el fixture, se actualizan en este commit los tests Rust, el contrato (`UPDATE_CONTRACTS=1`) y los tests de UI 21–24.

**Interfaces:** el fixture de Task 12 (`CMDR TESTER`, sin FID real) ya contiene: `LoadGame`, `Materials` (con un material al máximo y uno no catalogado), `EngineerProgress` lista (Unlocked/Invited/Known), `Loadout` con propulsores de serie y un FSD `FSD_LongRange` G3 q 1.0, `ShipLocker` completo + stub, `BackpackChange`, `MaterialCollected` ×3 (uno por encima del máximo), `MaterialTrade`, `Synthesis`, `MissionCompleted` con `MaterialsReward`, `FSDJump` a sistema con comerciante, `Docked` en el taller de un ingeniero, `EngineerCraft` sobre el FSD (G4), `EngineerProgress` suelto Invited→Unlocked.

- [ ] **Step 1: Tests:**

```rust
#[test] fn every_line_of_f4_fixture_is_typed() { /* 0 errores; eventos en Other == EXPECTED_OTHER explícito */ }
#[test] fn fixture_inventory_criterion_2() { /* counts por categoría/grado == Materials + cambios, a mano en el test */ }
#[test] fn fixture_engineers_criterion_3() { /* estado, rango y progreso == fixture; distancia desde el último FSDJump */ }
```
  más e2e Playwright (headless de depuración con Ardent falso vía la sustitución de F2): `engineering_inventory_and_at_max` (criterio 2), `engineering_engineers_status_distance` (3), `engineering_flow_dirty_drive_g5` (4), `engineering_trade_skips_reserved` (5), `engineering_nearest_trader_fake_ardent` y `engineering_no_services_no_data` (6), `engineering_goal_persists_after_restart` (7).
- [ ] **Step 2:** revisar fixtures (sin datos reales). **Step 3–4:** PASS; checks.
- [ ] **Step 5: Commit** `test: fixtures sintéticos de ingeniería con pruebas de extremo a extremo`

### Task 27: `tmc-headless eval` — conciliación del inventario de materiales

**Files:** Modify `apps/headless/src/eval.rs`, `crates/tmc-modules/src/eval.rs`; Create `tests/fixtures/journal/engineering_eval.log` (`Materials` → cambios → `Materials` coherente; no toca `engineering_session.log`).

**Interfaces:** informe agregado (sin nombres de comandante ni sistemas): para cada instantánea `Materials` posterior a otra en el historial, predicción = anterior + cambios con `MaterialInventory::apply_event`; métricas n, % de materiales exactos, diferencia absoluta media y máx., recuento de eventos entre medias no tipados (para «Desviaciones»). Veredicto `INSUFICIENTE` si n < 5 (como F2/F3).

- [ ] **Step 1: Tests:** `eval_inventory_reconciles_on_fixture` (`engineering_eval.log`: 100 % exactos, n == 1, `INSUFICIENTE`).
- [ ] **Step 2–4:** FAIL → implementar → PASS; checks; ejecutar sobre el historial del propietario y anotar solo agregados en «Verificación F4».
- [ ] **Step 5: Commit** `feat(headless): eval de conciliación del inventario de materiales`

### Task 28: Avisos de terceros, consumo, verificación de los 10 criterios y cierre

**Files:** Modify `THIRD_PARTY_NOTICES.md` (completar: Ardent/Spansh por red para comerciantes/broker; datos propios o importados de F4 con su licencia — lo importado ya se anotó en Tasks 8–10), este plan («Verificación F4», «Desviaciones»), `status.md`.

- [ ] **Step 1:** Checks completos de Rust (incluido `cargo clippy --workspace --all-targets --release -- -D warnings` con `CARGO_TARGET_DIR=target-f4`) y UI + e2e (criterio 1) con salida.
- [ ] **Step 2:** `CARGO_TARGET_DIR=target-f4 cargo build --release -p tmc-desktop` y `pwsh scripts/measure-footprint.ps1 -Exe target-f4/release/TMC.exe -JournalDir .tmp/footprint-f4 -FixturePath tests/fixtures/journal/engineering_session.log -ShowWindow -Seconds 120` con la disposición «Ingeniería» visible (criterio 10). **Nunca** `target/release/TMC.exe`. Si el TMC.exe del propietario ocupa la instancia, medir el núcleo con `tmc-headless` y dejar la medición con ventana como pendiente del propietario (como F3).
- [ ] **Step 3:** `git diff main -- Cargo.lock ui/package.json` y licencias de cada paquete o dato nuevo (criterio 8); procedencia y `verified` de los cuatro JSON; claves de los 5 idiomas y nombres oficiales (criterio 9); evidencia por criterio en «Verificación F4»; revisión completa de la rama contra la spec.
- [ ] **Step 4: Commit** `docs: verificación de criterios y cierre de F4`

---

## Auto-revisión (cobertura de la spec)

| Spec | Tareas |
|---|---|
| §3.1 disposición, activación y prioridad; Auto-companion | 20, 25 |
| §3.2 inventario por grado, máximos, taquilla, cambios de la sesión con origen | 4, 5, 8, 12, 13, 20, 21 |
| §3.3 ingenieros (estado, rango, distancia, blueprints, siguiente paso, a pie) | 5, 10, 14, 20, 22 |
| §3.4 mi nave e ingenierizar (pasos 1–8) | 5, 9, 15, 16, 17, 19, 20, 23 |
| §3.5 lista de la compra (persistente, reservas, completable) | 18, 19, 20, 24 |
| §3.6 intercambios | 8, 17, 24 |
| §3.7 dónde conseguirlo (pistas, comerciante, broker) | 2, 7, 8, 19, 24 |
| §3.8 avisos | 19, 20, 24 |
| §4 módulo único, tipado, datos propios, persistencia, red | 4–10, 12–20 |
| §5 servicios externos, opt-out, Legacy, «sin datos» | 2, 6, 7, 19, 20 |
| §6 rendimiento y robustez | 12, 20, 26, 28 |
| §7 i18n y nombres oficiales | 0, 11, 21–25 |
| §9 spikes | 0, 1, 2, 3 |
| §10.1 decisiones del propietario | 0 (fuente A→B), 9 (todos los blueprints, `verified:false`), 11 (nombres oficiales) |
| §10.2 decisiones del redactor 1–9 | 1→20; 2→6/16/25; 3→16; 4→16; 5→17; 6→18; 7→12/13; 8→18/6; 9→2/7 |

| Criterio §8 | Tareas que lo prueban |
|---|---|
| 1 checks + e2e + clippy release | 28 |
| 2 inventario = `Materials` + cambios; aviso al máximo | 12, 20, 26 |
| 3 ingenieros desde `EngineerProgress`; distancia | 14, 26 |
| 4 Motor sucio G5 + Drag Drives = cálculo manual; G3 → desde G4 | 16, 26 |
| 5 tasas correctas sin tocar reservados | 17, 18, 26 |
| 6 comerciante con Ardent falso; «sin datos» | 7, 20, 26 |
| 7 lista persiste y avisa completable | 18, 20, 26 |
| 8 procedencia, `verified`, nada GPL/AGPL, avisos de terceros | 0, 8, 9, 10, 28 |
| 9 5 idiomas y nombres oficiales con respaldo marcado | 11, 21–25, 28 |
| 10 footprint con «Ingeniería» | 28 |

**Consistencia de tipos revisada:** `MaterialCategory` (4) en 6 (`TraderKind::from`), 8, 12; `MaterialUse` (4) en 5, 12; `EngineerProgress`/`EngineerEntry`/`EngineerStatus` (5) en 14, 19, 20; `ShipLockerContents`/`BackpackChange` (5) en 13; `NearbyService::{MaterialTrader, TechnologyBroker}`/`TraderKind`/`BrokerKind` (6) en 7, 19, 20; `ModuleSettings.engineering_rolls` (6) en 16, 20, 23, 25; `MaterialDef`/`TraderRules`/`grade_cap` (8) en 12, 17, 19; `Blueprint`/`Experimental`/`BlueprintCatalog` (9) en 15, 16, 20; `EngineerDef`/`NextStep` (10, 14) en 19, 22; `CurrentEng` (15) en 16, 18; `PlanRequest`/`PlanCost` (16) en 18, 20; `TradeProposal`/`TradePlan` (17) en 18, 19, 20; `Goal`/`ShoppingView`/`ENGINEERING_MIGRATION` (18) en 19, 20; `SourcesView`/`EngineerPick`/`EngAlertKind` (19) en 20.

## Verificación F4

(Se completa en Task 28 con comando + salida por criterio.)

## Desviaciones

(Cambios respecto a este plan o a la spec, con fecha, motivo y tareas afectadas.)

- 2026-10-21 · **Task 20: revisión de cierre solicitada por el propietario; no completa.** El commit `9904133` no cerraba el seguimiento. La revisión encontró carreras al aplicar la lectura inicial de objetivos, blueprints conocidos incompatibles con el módulo, flujo obsoleto tras sustituir el módulo de una ranura, estaciones incompatibles tras cambiar de plataforma y cobertura insuficiente de tamaño/diagnóstico. Se corrigen con regresiones antes de avanzar a Task 21; las claves de red mantienen la interfaz del plan y se correlacionan además por ID de petición. Los reintentos de `Failed` ya los realiza `NetService::retry` mediante `NetClient::retry_at`; no se añade otro mecanismo.
- 2026-10-21 · **Bloqueo del contrato de Task 20, pendiente de decisión del propietario.** Una medición con el serializador y evaluador reales, 20 objetivos de propulsores de serie `engine_dirty` G5 + `special_engine_overloaded`, tiradas `[10; 5]` y excedentes únicamente de materiales ajenos a la receta produce `engineering.goals` de **41.116 bytes** con **215 propuestas**. Sin experimental son 30.986 bytes/155 propuestas; FSD, 35.084 bytes/164 propuestas. Además, `engineering.locker` ocupa **16.832 bytes** con 200 recursos localizados (50 por cada una de las cuatro secciones); la sesión publicada con 50 cambios ocupa 4.242 bytes, la nave con 60 módulos 11.879 y 20 avisos 2.803. Incumple `< 16.384` bytes: el escenario anterior con inventario lleno y objetivos FSD ya modificados no acreditaba el peor caso. No se recortan objetivos ni intercambios, ni se aumenta el límite sin aprobación. Propuesta a aprobar: resumen y revisión por WebSocket con detalles paginados por bytes vía REST, siguiendo arquitectura §7; alternativa de menor implementación: excepciones explícitas de tamaño para objetivos y taquilla, sujeta a comprobar consumo y latencia. La primera opción afecta Task 20, tipos de Task 21, paneles de Task 24 y e2e de Task 26; las interfaces aprobadas actuales permanecen vigentes hasta la decisión. El probe reproducible se conserva como prueba ignorada con motivo explícito y se ejecuta manualmente para evidenciar el incumplimiento; no equivale a un PASS ni al cierre de Task 20.

- 2026-10-21 · Resoluciones del controlador tras el preflight y los spikes (Tasks 0–2):
  1. **Orden de Task 3:** Tasks 8–10 escriben código con un conjunto provisional mínimo `verified:false` (incluye `engine_dirty` G1–G5 + Drag Drives del criterio 4 y `fsd_longrange`); solo el contenido completo espera la decisión de fuente (spike de datos: B en todos los archivos, nada importado). Tasks 12–15 siguen tras el código de 8–10.
  2. **Fixtures:** Task 12 crea `engineering_session.log` con todo lo que usan Tasks 12–20; quien lo amplíe lo lista en *Files*. Task 27 usa `engineering_eval.log` propio.
  3. **Contrato UI:** Task 20 fija el escenario y los valores de `ui/tests/fixtures/engineering-topics.json` (test `ui_contract_snapshot_is_up_to_date`, como `missions`/`trade`), que afirman Tasks 21–24.
  4. **Posición:** `LocationState.star_pos` no es `Option`; el módulo guarda `position: Option<[f64;3]>` desde el primer `Location`/`FSDJump`/`CarrierJump`.
  5. **Grados:** `start_grade` recibe `rolls`; `Level` = grado actual, `Quality` 0..1 = progreso en ese grado; a medias = `max(1, ceil(rolls_g×(1−quality)))`; `quality >= 0.999` = completo.
  6. **Intercambios solo dentro del tipo de comerciante** (§10.2.5): «entre categorías» de §3.6 significa «otra familia del mismo tipo de comerciante».
  7. **`Goal.id`** = `{ship_id}:{slot}:{blueprint}:{experimental|-}:{grade}`; repetir `pin_goal` con el mismo id lo sustituye.
  8. Task 11 añade `unlock_keys_exist_in_engineering_data_ftl` y `every_module_type_has_modtype_name`.
  9. `Ord/PartialOrd` en `MaterialCategory`, `ChangeOrigin`, `TraderKind`, `BrokerKind`, `EngineerStatus` (claves de `BTreeMap`/`BTreeSet`).
  10. `SourcesState.current_system` + `set_current_system`; `on_stations` rechaza otros sistemas.
  11. Tests de valor único: Task 4 entrada mal formada → omitida (`len()==1`); Task 7 sin disyunciones (con tipo → Spansh; sin tipo → Ardent; fallo de Spansh → `NoData`).
  12. **Red (spike):** con tipo → Spansh `POST /api/stations/search` (`material_trader` Raw/Manufactured/Encoded, `technology_broker` Guardian/Human, `size` 5, distancia asc., `reference_system`); sin tipo → Ardent `nearest/material-trader` y `nearest/technology-broker`; 4 etiquetas `endpoint` nuevas en la lista cerrada; `NearestService` gana `system_name` (cambio aditivo de la variante de F2). **Diario (spike):** stub de `ShipLocker` → releer `ShipLocker.json`, nunca vaciar; `EngineerProgress` suelto con campos opcionales; `MaterialsReward.Category` llega como `$MICRORESOURCE_CATEGORY_X;` → se normaliza, con `materials.json` de respaldo si falta o es desconocida; `ExperimentalEffect` no visto en los diarios del propietario → fixtures desde el manual del diario; `ScientificResearch` no se tipa: la siguiente instantánea `Materials` (en cada `LoadGame`) resincroniza.
  13. Respaldo `name_en` de materiales (spec §7) en Task 12, con test de UI en Task 21; test de blueprints y grado máximo del ingeniero en Task 22.

Ambigüedades de la spec resueltas por el redactor (2026-10-21; se someten al propietario en Task 3):

- **Grado a medias** (§10.2.4): grado completo = `quality >= 0.999`; si el grado actual *n* está a medias y es del mismo blueprint, cuenta `max(1, ceil(tiradas_n × (1 − quality)))` tiradas de G*n* y luego los grados siguientes; si está completo, desde G*n+1* (criterio 4).
- **Intercambios de varios grados:** se encadenan las tasas (`6^n:1` al subir, `1:3^n` al bajar); entre familias se aplica `cross_family_factor` de `trader_rules.json` (confirmado en el spike). Una sola fuente por faltante; nunca se recibe por encima del máximo del grado.
- **Reservas** (§3.5 + §10.2.6): el excedente de cualquier objetivo excluye lo reservado por **todos** los fijados; las reservas se asignan en el orden de la lista; el flujo abierto sin fijar se evalúa al final.
- **Origen de `MaterialCollected`:** codificado → escaneo; materia prima → minería/recolección; fabricado → recogido.
- **Taquilla:** última instantánea `ShipLocker` (evento o `ShipLocker.json`) + `BackpackChange`; otros eventos de micro-recursos no se tipan en F4 y la siguiente instantánea resincroniza.
- **Ingeniero recomendado:** se muestran el más cercano desbloqueado y el más cercano en general (si difiere), con el siguiente paso de este último.
- **Broker de tecnología «si aplica»:** solo cuando algún material faltante tiene pista `broker`; si no, no se consulta.
- **Relevancia:** taller = `Docked` con `market_id` o (sistema, taller) de `engineers.json`, o último evento `EngineerCraft`/`EngineerProgress` suelto/`EngineerContribution`, o flujo abierto; se limpia al despegar, saltar o cerrar el flujo. La lista de arranque de `EngineerProgress` no activa la disposición.
- **Tiradas ajustables:** rango 1–10 por grado (`config.toml` `engineering_rolls`).
- **Tamaño de topics:** `engineering.inventory` solo publica materiales con cantidad > 0 (y los no catalogados); la UI no lista los que tienes a cero.
- **Límite de la lista de la compra:** 20 objetivos (`MAX_GOALS`).
