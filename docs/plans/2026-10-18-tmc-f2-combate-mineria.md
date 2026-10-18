# TMC F2 «Cazarrecompensas/combate (incl. AX) y minería» — Plan de implementación

> **Para agentes:** SUB-SKILL OBLIGATORIA: usar superpowers:subagent-driven-development (recomendado) o superpowers:executing-plans para ejecutar este plan tarea a tarea. Los pasos usan casillas (`- [ ]`) para el seguimiento. Lo marcado «(verificar en código al ejecutar)» es una interfaz existente que el redactor no confirmó: se comprueba antes de escribir el test y, si difiere, se adapta el nombre sin cambiar el comportamiento y se anota en «Desviaciones».

**Goal:** Que la segunda pantalla diga, mientras se caza o se mina, qué hay delante (objetivo con LIMPIO/BUSCADO y subsistema), cómo está la nave (escudos, casco por escalones, calor de tres estados, pips exactos), cuánto se lleva ganado (vales sin cobrar por facción y tipo, créditos/hora, t/h) y qué hacer después (dónde cobrar, Interstellar Factors más cercano, dónde vender la carga).

**Architecture:** Se conserva la arquitectura de F0/F1 (actor único dueño del estado, módulos sin E/S, difusión a 10 Hz, red en `tmc-net` vía `ModuleCtx::request` → `SourceEvent::Net`, SQLite vía `ModuleCtx::store_task` → `SourceEvent::Stored`). Dos módulos nuevos: **`combat`** (objetivo, mi nave, sesión de caza con libro de vales, zona de conflicto, AX) y **`mining`** (prospector, sesión de minería, dónde vender). Adaptador nuevo **Ardent** en `tmc-net` con dos consultas (`CommodityImportsNearby`, `NearestService`). Datos estáticos propios `ship_pads.json` y `mining_yield.json` con procedencia y `verified`.

**Tech Stack:** el de F1 (Rust 2024, tokio, serde, rusqlite, axum, reqwest+rustls, Fluent, Tauri 2, Svelte 5 + TS + Vite, Vitest, Playwright). **Sin dependencias nuevas** previstas.

**Spec:** [`docs/specs/2026-10-18-tmc-f2-combate-mineria-design.md`](../specs/2026-10-18-tmc-f2-combate-mineria-design.md) (aprobada 2026-10-18; manda sobre este plan). Base: spec de arquitectura F0 y spec F1. Dominio: `docs/research/notes/player_roles_domain.md` §7 (combate), §8 (AX), §9 (minería); `docs/research/notes/community_apis_core.md` (sección Ardent).

## Restricciones globales

- Código e identificadores en inglés; documentación, commits y textos de referencia (`es`) en español.
- Presupuesto (requisito): **< 150 MB** RAM del árbol de procesos, **< 1 %** CPU en reposo, difusión **máx. 10 Hz**, latencia evento→UI **p95 < 500 ms**. Sin animaciones infinitas ni `requestAnimationFrame` continuo; los cronómetros de la UI usan un `setInterval` de 1 s **solo mientras hay algo que contar** (sobrecalentamiento) o un `setTimeout` único (vaciado del objetivo a los 10 s).
- **Todo test afirma un valor concreto; no hay tests vacíos** (ningún cuerpo `{}` ni solo comentario). Los `/* … */` de este plan describen entrada y valor esperado; el test real los escribe como datos y `assert_eq!`. Los marcadores `<valor>`/`<n>` de los tests con fixture se sustituyen por el número concreto al escribir el fixture, en el mismo commit.
- Tipos: variantes grandes de enums van en `Box` (clippy `large_enum_variant`), como `EventKind::Scan(Box<Scan>)`. Las variantes nuevas de `EventKind`, `NetQuery`, `NetOutcome`, `NetSource` siguen el patrón existente (`de(name, raw).map(...)`; un struct que no deserializa cae en `Other`, nunca error).
- **Módulos** (lecciones de F1, obligatorias):
  - nunca emiten `NetQuery` ni `StoreTask` de **escritura** durante el replay (`ModuleCtx` ya suprime peticiones en replay: comprobarlo con test por módulo);
  - todo resultado `SourceEvent::Stored` comprueba el comandante (y generación) antes de aplicarse; si no coincide, se descarta;
  - peticiones en vuelo caducan a los **60 s** (`INFLIGHT_TTL_S`, como `navigation`); un `Failed` se reintenta solo tras el enfriamiento que marca `NetClient::retry_at`; peticiones idénticas en vuelo no se duplican;
  - un topic se marca sucio **solo si su valor cambió**; cada topic tiene tamaño acotado con test (< 16 KiB);
  - respuestas tardías (de otro sistema o de una consulta anterior) se guardan en caché pero no se aplican ni pisan datos más nuevos;
  - `Module::relevance(&self, &GameState)` no recibe reloj: cada módulo guarda `last_ts` (marca del último evento/`Status`) y la caducidad por tiempo (10 s, 30 s, 10 min) se evalúa en el siguiente evento; los tests avanzan el tiempo con un evento posterior;
  - el control de peticiones en vuelo (TTL 60 s, deduplicación, descarte de tardías) vive en un único helper `crate::inflight::Inflight` (reutilizar el de `navigation` si ya existe; verificar) que usan `combat` y `mining`.
- Red: nunca se consulta con `Galaxy::Legacy` ni con «Consultar servicios online» desactivado. Ardent: **1 petición/s**, timeout 5 s, reintentos y salud como F1 (3 fallos = caído 10 min), `User-Agent: TMC/<versión>`. Caché `api_cache`: mercados cercanos **15 min** por (sistema, mineral, radio), servicio más cercano **1 h** por (sistema, servicio). Solo se envía `SystemAddress` y nombre del mineral; nada del comandante.
- Licencias: Ardent es AGPL-3.0, **solo uso por red**; sus respuestas van a `api_cache`, nunca al repo. Los fixtures de red de F2 son **sintéticos con la forma real** observada en el spike (valores inventados). Nada GPL/AGPL incorporado; toda fuente nueva en `THIRD_PARTY_NOTICES.md`.
- **Datos estáticos propios** (`crates/tmc-modules/data/*.json`): hechos del juego o estimaciones propias, cada entrada con `source` (procedencia) y `verified: bool`; carga **tolerante** (JSON inválido o entrada mala → se registra con `tracing::warn!` y se ignora; nunca `panic!`/`unwrap` en producción), con test de carga real y de carga corrupta.
- i18n: ninguna cadena visible en el código; claves en `locales/{es,en,fr,de,pt}/{combat,mining}.ftl` (y `main.ftl` para ajustes/disposiciones), **traducciones reales** en los 5 idiomas y mismas claves (test existente). El test `embedded_resources_parse_and_format_all_messages_without_errors` de `crates/tmc-i18n/src/lib.rs` usa una **lista fija de nombres de argumento**: toda tarea que añada una variable Fluent nueva (`reward`, `faction`, `tons`, `rate`, `price`, `fee`, `ship`, `material`, `remaining`, `days`, `distance`…) la añade a esa lista en el mismo commit.
- UI: números, créditos, porcentajes, distancias y fechas **solo** con los helpers `Intl` de `ui/src/lib/format.ts` (sin `toFixed`); lo estimado se marca «≈»; las peticiones que cambian estado (`PUT /api/config`, acciones) envían JSON con `Content-Type: application/json` (protección CSRF del servidor).
- **Checks** antes de cada commit: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; si se toca `ui/`: `npm --prefix ui run check`, `npm --prefix ui test`, `npm --prefix ui run build`.
- Commits pequeños (uno por tarea), en español con prefijo convencional y línea final `Co-Authored-By: <modelo que ejecuta> <noreply@anthropic.com>`. **Sin push** sin orden del propietario. `status.md` al empezar y terminar cada tarea (AGENTS.md §4).
- Una sola fuente por concepto: el libro de vales se actualiza **solo** con `VoucherLedger::apply_event` (en vivo, al reconstruir y en `eval`); el precio de un mineral se resuelve **solo** con `PriceBook::best_price`; el estado de calor **solo** con `HeatTracker`; el tamaño de plataforma **solo** con `data::ship_pads`.

## Foco de revisión

Entradas que la spec implica, que ningún test de F0/F1 cubre y que más pueden morder al usuario. Cada una tiene su test en la tarea dueña:

1. **Vales mal conciliados** (canje parcial, canje sin facción detallada, canje en IF con `BrokerPercentage`, muerte entre medias, reinicio de la app): la suma sin cobrar debe cuadrar ±0 → `redeem_without_faction_subtracts_oldest_first`, `broker_redeem_clears_face_value`, `died_zeroes_all_unredeemed_vouchers` (Task 13) y `ledger_rebuilt_after_restart_matches_live_ledger` (Task 14).
2. **Ráfagas de combate** (cientos de `Status`/`ShipTargeted`/`UnderAttack` por segundo): los topics solo se ensucian si cambian y el difusor no supera 10 Hz → `identical_status_burst_marks_ship_dirty_once` (Task 12), `burst_of_1000_combat_events_stays_under_10hz_and_500ms` (Task 26).
3. **Objetivo perdido, sin escanear o con valores raros** (`TargetLocked:false`, `ScanStage` < 3, `LegalStatus` desconocido, nave thargoide): «escaneando…», vaciado a los 10 s, nunca pánico → `target_lost_sets_lost_at_and_keeps_data`, `scan_stage_below_3_is_scanning`, `unknown_legal_status_is_unknown_badge` (Task 11).
4. **Datos de mercado viejos o absurdos** (obras de construcción, carriers, `demandBracket` texto/número, plataforma incompatible, respuesta de otro sistema): se filtran o se marcan con su antigüedad → `construction_sites_and_carriers_are_excluded`, `demand_bracket_accepts_string_and_number` (Task 7), `large_pad_market_hidden_for_large_ship_only_when_incompatible` (Task 21) y `late_markets_for_previous_system_not_applied` (Task 22).
5. **Servicios desactivados, caídos o Legacy**: paneles locales funcionan, «Dónde vender» muestra «sin datos», el valor del prospector cae al precio local → `disabled_online_services_never_reach_ardent` (Task 7), `no_requests_in_legacy_or_offline` (Tasks 16 y 22), `price_falls_back_to_last_market_sell` (Task 21).
6. **Replay e historial**: ninguna petición ni escritura durante el replay; resultados `Stored` de otro comandante descartados → `replay_emits_no_requests_or_store_writes` y `stored_result_for_other_cmdr_is_ignored` (Tasks 16 y 22).

## Resumen de tareas

| # | Tarea | Bloque |
|---|---|---|
| 0 | Spike AX — diarios del propietario (agregados) y notas de actualización | 0 |
| 1 | Spike Ardent — `nearby/imports` y `nearest/interstellar-factors` | 0 |
| 2 | Aprobaciones del propietario tras los spikes | 0 |
| 3 | `tmc-model` — eventos de combate | 1 |
| 4 | `tmc-model` — eventos de minería y mercado, `Cargo` con inventario, reparaciones | 1 |
| 5 | `tmc-model` — bloque `ThargoidWar` y facciones en `LocationInfo` | 1 |
| 6 | `tmc-model` — `NetQuery`/`NetOutcome` de Ardent, `PadSize`, `ModuleSettings` de minería | 1 |
| 7 | `tmc-net` — adaptador Ardent, límites, caché y `NetClient` | 1 |
| 8 | `tmc-app`/`tmc-server` — configuración nueva en `config.toml` y `PUT /api/config` | 1 |
| 9 | `tmc-modules` — datos `ship_pads.json` y `mining_yield.json` | 1 |
| 10 | i18n — `combat.ftl` y `mining.ftl` en 5 idiomas | 1 |
| 11 | `combat` — objetivo actual (`TargetTracker`) | 2 |
| 12 | `combat` — mi nave (escudos, casco por escalones, calor, pips, alertas) | 2 |
| 13 | `combat` — libro de vales (`VoucherLedger`) | 2 |
| 14 | `combat` — sesión de caza y reconstrucción desde el historial | 2 |
| 15 | `combat` — zona de conflicto y AX | 2 |
| 16 | `combat` — módulo, topics, relevancia, red (IF) y registro | 2 |
| 17 | UI — paneles «Objetivo» y «Mi nave» | 2 |
| 18 | UI — paneles «Sesión de caza», «Zona de conflicto» y «AX» | 2 |
| 19 | `mining` — prospector (composición, valor, veredicto, histórico) | 3 |
| 20 | `mining` — sesión de minería (refinados, t/h, refinería, limpets, carga) | 3 |
| 21 | `mining` — `PriceBook` y «Dónde vender» | 3 |
| 22 | `mining` — módulo, topics, relevancia, red y registro | 3 |
| 23 | UI — panel «Prospector» | 3 |
| 24 | UI — paneles «Sesión de minería» y «Dónde vender» | 3 |
| 25 | UI — disposiciones «Combate» y «Minería» con relevancia, y ajustes | 4 |
| 26 | Fixtures sintéticos y pruebas de extremo a extremo (criterios 2, 3, 5, 6, 7) | 4 |
| 27 | `tmc-headless eval` — vales y rendimiento de asteroides | 4 |
| 28 | Avisos de terceros, consumo, verificación de los 10 criterios y cierre de F2 | 4 |

## Mapa de archivos

```
crates/tmc-model/src/{events.rs*, net.rs*, settings.rs*}
crates/tmc-net/src/{ardent.rs+, client.rs*, health.rs*, limiter.rs*, cache_policy.rs*, lib.rs*}
crates/tmc-modules/data/{ship_pads.json+, mining_yield.json+}
crates/tmc-modules/src/{lib.rs*, registry.rs*, data.rs*, eval.rs*, inflight.rs+}
crates/tmc-modules/src/combat/{mod.rs+, target.rs+, ship.rs+, vouchers.rs+, session.rs+, cz.rs+, ax.rs+, tests.rs+}
crates/tmc-modules/src/mining/{mod.rs+, prospector.rs+, session.rs+, prices.rs+, sell.rs+, tests.rs+}
crates/tmc-i18n/src/lib.rs*                      (lista de argumentos del test de formato)
crates/tmc-app/src/{config.rs*, wiring.rs*}      crates/tmc-server/src/rest.rs*
apps/headless/src/eval.rs*
locales/{es,en,fr,de,pt}/{main.ftl*, combat.ftl+, mining.ftl+}
ui/src/modules/registry.ts*   ui/src/shell/layouts.ts*   ui/src/settings/Settings.svelte*
ui/src/modules/combat/{index.ts+, types.ts+, TargetPanel.svelte+, ShipPanel.svelte+, PipsGauge.svelte+,
                       HuntingPanel.svelte+, ConflictZonePanel.svelte+, AxPanel.svelte+}
ui/src/modules/mining/{index.ts+, types.ts+, ProspectorPanel.svelte+, MiningSessionPanel.svelte+, SellPanel.svelte+}
ui/tests/{combat.test.ts+, mining.test.ts+, layouts.test.ts*}
tests/fixtures/journal/{combat_session.log+, mining_session.log+, ax_session.log+}
tests/fixtures/status/{combat_*.json+}
tests/fixtures/net/{ardent_nearby_imports_platinum.json+, ardent_nearby_imports_empty.json+, ardent_nearest_if.json+}
docs/research/notes/{spike_ax_f2.md+, spike_ardent_f2.md+}
THIRD_PARTY_NOTICES.md*
```
(`+` crear, `*` modificar.)

---

## Bloque 0 — Spikes y aprobaciones

Sin código de producción. Peticiones reales a mano (curl o script desechable en el scratchpad, nunca en el repo), ≤ 1 petición/s.

### Task 0: Spike AX

**Files:** Create `docs/research/notes/spike_ax_f2.md`.

**Interfaces:** Produces la lista de eventos/campos AX confirmados que consumen Tasks 5, 15 y 26 (`THARGOID_SHIPS`, detección del bono AX, presencia thargoide).

- [ ] **Step 1:** Recorrer los diarios del propietario con un script desechable que **solo emite agregados** (recuentos por evento y por valor de campo; nunca nombres de comandante, FID, chats ni nombres de pilotos): `ShipTargeted.Ship` con valores thargoides, `FactionKillBond.VictimFaction` thargoide (`$faction_Thargoid;`?), `Bounty`/`FactionKillBond` con `SharedWithOthers`, `RedeemVoucher` (tipos, presencia de `Factions[]`, `Faction`, `BrokerPercentage` y si `Amount` es neto o nominal), bloque `ThargoidWar` en `FSDJump`/`Location`, estados de facción `Thargoid_*`, `Status.LegalState` = `Thargoid`.
- [ ] **Step 2:** Revisar notas de actualización 4.1–4.4.x por cambios AX (interceptores, ZC AX, recompensas) tras la guerra (fin 2024-12-13).
- [ ] **Step 3:** Documentar: tabla «evento/campo → confirmado (n.º de apariciones) / no visto / fuente secundaria», valores de `Ship` thargoides (scout, interceptor, swarm…), forma de `ThargoidWar`, regla para `BrokerPercentage` y asistencias, fecha.
- [ ] **Criterio de decisión:** lo no confirmado se **omite** (se anota la omisión en «Desviaciones» de este plan, no bloquea). Si `Amount` con broker es neto, la regla de Task 13 usa `Amount / (1 − BrokerPercentage/100)` para el valor nominal; si es nominal, `Amount` tal cual. Sin ningún dato AX real, el fixture AX es sintético (criterio 7 lo permite).
- [ ] **Step 4: Commit** `docs: spike AX para F2 (agregados de diarios y notas de actualización)`

### Task 1: Spike Ardent

**Files:** Create `docs/research/notes/spike_ardent_f2.md`, `tests/fixtures/net/ardent_nearby_imports_platinum.json`, `ardent_nearby_imports_empty.json`, `ardent_nearest_if.json`.

**Interfaces:** Produces el mapeo «campo Ardent → `RemoteMarket`/`RemoteStation`» (Task 6) y las URL de `tmc_net::ardent` (Task 7).

- [ ] **Step 1:** Peticiones reales: (a) `GET /v2/system/address/{id64}/commodity/name/{c}/nearby/imports` con `maxDistance`, `fleetCarriers=false`, `minVolume`, `maxDaysAgo` (¿acepta `minLandingPadSize`?); (b) `GET /v2/system/address/{id64}/nearest/interstellar-factors?minLandingPadSize=`; (c) nombre de mineral esperado por Ardent frente al `Type` del diario (`platinum`, `lowtemperaturediamond`, `opal`/Void Opal, `painite`…); (d) campos de plataforma (`maxLandingPadSize` 1/2/3), fecha (`updatedAt`), demanda (`demand`, `demandBracket` texto/número), tipo de estación y cómo reconocer obras de construcción/colonización; (e) latencia y cabeceras.
- [ ] **Step 2:** Documentar endpoint, parámetros, respuesta recortada, mapeo, valores por defecto elegidos (`maxDaysAgo`, `minVolume`), errores (404 sistema desconocido, `[]`), fecha.
- [ ] **Step 3:** Escribir los 3 fixtures **sintéticos con la forma real** (estaciones y sistemas inventados, precios inventados; incluir a propósito una obra de construcción, un carrier, un `demandBracket` texto y otro número, una estación solo de plataforma M).
- [ ] **Criterio de decisión:** si `nearby/imports` no filtra por plataforma, el filtro es local con `data::ship_pads` (el parámetro `min_pad` de `NetQuery` se conserva y el adaptador lo ignora); si la ruta por `address` no existe, se usa `name/{nombre}`. La interfaz interna (Task 6) **no cambia**; solo el adaptador.
- [ ] **Step 4: Commit** `docs: spike de Ardent para F2 y fixtures sintéticos de red`

### Task 2: Aprobaciones del propietario tras los spikes

**Files:** Modify `status.md` («Bloqueos»).

- [ ] **Step 1:** Registrar y pedir aprobación solo de lo que cambie alcance: omisiones AX (Task 0), valores por defecto de Ardent (`maxDaysAgo`), y la decisión del redactor sobre asistencias (ver Task 14).
- [ ] **Step 2:** Las Tasks 3–24 avanzan mientras tanto (las interfaces no dependen de la respuesta).
- [ ] **Step 3: Commit** `docs(status): aprobaciones pendientes tras los spikes de F2`

---

## Bloque 1 — Eventos, red, datos y configuración

### Task 3: `tmc-model` — eventos de combate

**Files:** Modify `crates/tmc-model/src/events.rs`.

**Interfaces:**
- Produces (structs `#[serde(default, rename_all = "PascalCase")]`, `Debug + Clone + PartialEq + Serialize + Deserialize + Default`, campos `Option` salvo imprescindibles; `_Localised` con `rename` explícito):
  - `ShipTargeted { target_locked: bool, ship, ship_localised, scan_stage: Option<u8>, pilot_name, pilot_name_localised, pilot_rank, shield_health: Option<f64>, hull_health: Option<f64>, faction, legal_status, bounty: Option<u64>, subsystem, subsystem_localised, subsystem_health: Option<f64>, power }` (en el diario `Subsystem`; `ShieldHealth`/`HullHealth`/`SubsystemHealth` en 0–100). Se clasifica para `"ShipTargeted"` y la variante histórica `"ShipTargetted"` si aparece (alias en `classify`).
  - `BountyReward { faction: String, reward: u64 }`; `Bounty { rewards: Vec<BountyReward>, total_reward: Option<u64>, reward: Option<u64>, faction: Option<String>, victim_faction, victim_faction_localised, target, target_localised, shared_with_others: Option<u32> }` (formato antiguo de patines: `Reward` + `Faction` sin `Rewards`; `fn normalized_rewards(&self) -> Vec<BountyReward>` unifica ambos).
  - `FactionKillBond { reward: u64, awarding_faction, awarding_faction_localised, victim_faction, victim_faction_localised }`; `CapShipBond` con la misma forma (`type CapShipBond = FactionKillBond` si deserializa igual).
  - `RedeemVoucher { type_: String (rename "Type"), amount: u64, faction: Option<String>, factions: Vec<VoucherFaction { faction: String, amount: u64 }>, broker_percentage: Option<f64> }`.
  - `ShieldState { shields_up: bool }`, `EscapeInterdiction { interdictor, is_player: Option<bool> }`, `PvpKill { victim, combat_rank: Option<u8> }`, `Promotion { combat: Option<u8>, trade, explore, cqc, soldier, exobiologist, federation, empire }` (todos `Option<u8>`).
  - `EventKind` + `ShipTargeted(Box<ShipTargeted>)`, `Bounty(Box<Bounty>)`, `FactionKillBond(FactionKillBond)`, `CapShipBond(FactionKillBond)`, `RedeemVoucher(RedeemVoucher)`, `ShieldState(ShieldState)`, `HeatWarning`, `HeatDamage`, `EscapeInterdiction(EscapeInterdiction)`, `PvpKill(PvpKill)` (`"PVPKill"`), `FighterDestroyed`, `Promotion(Promotion)`.

  - `HullDamage` (ya tipado): verificar que tiene `fighter: Option<bool>`; si falta, añadirlo aquí (lo consume Task 12).

- [ ] **Step 1: Tests** (en `events.rs`):

```rust
#[test] fn ship_targeted_full_scan_parses() { /* TargetLocked true, Ship "viper_mkiv", Ship_Localised "Viper Mk IV", ScanStage 3, PilotRank "Dangerous", ShieldHealth 45.5, HullHealth 80.0, LegalStatus "Wanted", Bounty 125000, Subsystem "$int_powerplant_size3_class5_name;", SubsystemHealth 62.0 → bounty Some(125000), scan_stage Some(3), subsystem_health Some(62.0) */ }
#[test] fn ship_targeted_lost_target_parses_minimal() { /* {"event":"ShipTargeted","TargetLocked":false} → target_locked false, ship None */ }
#[test] fn bounty_rewards_and_legacy_skimmer_format_normalize() { /* Rewards [{A,50000},{B,10000}] → normalized 2 entradas, suma 60000; {Reward:5000, Faction:"C"} → normalized [{C,5000}] */ }
#[test] fn redeem_voucher_with_factions_and_broker() { /* Type "bounty", Amount 37500, Factions [{Faction:"A",Amount:50000}], BrokerPercentage 25.0 → factions[0].amount 50000, broker Some(25.0) */ }
#[test] fn faction_kill_bond_and_cap_ship_bond_parse() { /* Reward 20000, AwardingFaction "A", VictimFaction "B" → kind FactionKillBond; idem "CapShipBond" → CapShipBond, reward 20000 */ }
#[test] fn heat_and_shield_events_parse() { /* "HeatWarning"→HeatWarning, "HeatDamage"→HeatDamage, ShieldState{ShieldsUp:false} → shields_up false */ }
#[test] fn promotion_combat_rank_parses() { /* {"event":"Promotion","Combat":5} → combat Some(5) */ }
```

- [ ] **Step 2:** `cargo test -p tmc-model` → FAIL. **Step 3:** implementar y clasificar en `classify`; añadir los brazos nuevos a `match` exhaustivos (`state.rs`, `actor.rs`) sin cambiar comportamiento. **Step 4:** PASS; snapshots `insta` de F0/F1 sin cambios; Checks.
- [ ] **Step 5: Commit** `feat(model): eventos de combate tipados`

### Task 4: `tmc-model` — eventos de minería y mercado, `Cargo` con inventario, reparaciones

**Files:** Modify `crates/tmc-model/src/events.rs`.

**Interfaces:**
- Consumes: `Cargo { vessel, count }` existente (se amplía), `EventKind::Market` (unidad; no cambia).
- Produces:
  - `ProspectedMaterial { name: String, name_localised: Option<String>, proportion: f64 }`; `AsteroidContent { Low, Medium, High, Unknown }` con `fn from_journal(s: &str) -> Self` (`"$AsteroidMaterialContent_High;"` → `High`; desconocido → `Unknown`); `ProspectedAsteroid { materials: Vec<ProspectedMaterial>, content: String, content_localised, motherlode_material, motherlode_material_localised, remaining: Option<f64> }` + `fn content_level(&self) -> AsteroidContent`.
  - `AsteroidCracked { body }`, `MiningRefined { type_ (rename "Type"), type_localised }`, `LaunchDrone { type_ (rename "Type") }`, `EjectCargo { type_, type_localised, count: u32, abandoned: Option<bool> }`, `MarketSell { market_id (rename "MarketID"), type_, type_localised, count: u32, sell_price: u64, total_sale: u64, avg_price_paid: Option<u64> }`, `RepairAll { cost: Option<u64> }`, `Repair { items: Vec<String>, item: Option<String>, cost: Option<u64> }`.
  - `Cargo` + `inventory: Vec<CargoItem { name: String, name_localised: Option<String>, count: u32, stolen: Option<u32> }>`.
  - `EventKind` + `ProspectedAsteroid(Box<ProspectedAsteroid>)`, `AsteroidCracked(..)`, `MiningRefined(..)`, `LaunchDrone(..)`, `EjectCargo(..)`, `MarketSell(..)`, `RepairAll(..)`, `Repair(..)`.
  - Normalización de nombres de mineral: `pub fn commodity_key(raw: &str) -> String` (minúsculas, sin `$`, `_name;` ni espacios: `"$platinum_name;"`, `"Platinum"` → `"platinum"`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn prospected_asteroid_parses_materials_content_and_motherlode() { /* Materials [{Name:"Platinum",Proportion:31.5},{Name:"Painite",Proportion:12.0}], Content "$AsteroidMaterialContent_High;", MotherlodeMaterial "Monazite", Remaining 100.0 → 2 materiales, content_level High, motherlode Some("Monazite") */ }
#[test] fn unknown_asteroid_content_is_unknown() { /* Content "$Foo;" → Unknown */ }
#[test] fn mining_refined_and_launch_drone_parse() { /* MiningRefined Type "$platinum_name;" → commodity_key "platinum"; LaunchDrone Type "Prospector" */ }
#[test] fn market_sell_parses_prices() { /* Type "platinum", Count 20, SellPrice 220000, TotalSale 4400000 */ }
#[test] fn cargo_event_with_inventory_parses_limpets() { /* Inventory [{Name:"drones",Count:18},{Name:"platinum",Count:42}] → inventory.len()==2, drones 18 */ }
#[test] fn commodity_key_normalizes_variants() { /* "$lowtemperaturediamond_name;"→"lowtemperaturediamond", "Platinum"→"platinum", "Low Temperature Diamond"→"lowtemperaturediamond" */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks (snapshots sin cambios).
- [ ] **Step 5: Commit** `feat(model): eventos de minería, venta en mercado, inventario de carga y reparaciones`

### Task 5: `tmc-model` — bloque `ThargoidWar` y facciones en `LocationInfo`

**Files:** Modify `crates/tmc-model/src/events.rs`.

**Interfaces:**
- Consumes: `LocationInfo` (ya en `Box`), `SystemFaction`.
- Produces: `ThargoidWar { current_state: Option<String>, next_state_success, next_state_failure, success_state_reached: Option<bool>, war_progress: Option<f64>, remaining_ports: Option<u32>, estimated_remaining_time: Option<String> }`; `LocationInfo` + `thargoid_war: Option<ThargoidWar>`, `factions: Vec<FactionInfo { name: String, faction_state: Option<String>, active_states: Vec<StateRef { state: String }> }>` (si `factions` ya existe con otro nombre, reutilizarlo — verificar en código al ejecutar); `impl LocationInfo { pub fn thargoid_presence(&self) -> bool }` = `thargoid_war.current_state` no vacío/≠ `""` **o** algún estado de facción/sistema que empiece por `Thargoid_`. (Campos exactos según Task 0; lo no confirmado se omite.)

- [ ] **Step 1: Tests:**

```rust
#[test] fn fsdjump_with_thargoid_war_block_parses() { /* ThargoidWar{CurrentState:"Thargoid_Controlled", WarProgress:0.25, RemainingPorts:2} → war_progress Some(0.25), thargoid_presence true */ }
#[test] fn faction_state_thargoid_marks_presence() { /* sin ThargoidWar, Factions[{Name:"X",FactionState:"Thargoid_Probing"}] → true */ }
#[test] fn empty_thargoid_war_state_is_no_presence() { /* ThargoidWar{CurrentState:""} → false */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(model): bloque ThargoidWar y estados de facción en la ubicación`

### Task 6: `tmc-model` — consultas de Ardent, `PadSize` y ajustes de minería

**Files:** Modify `crates/tmc-model/src/net.rs`, `crates/tmc-model/src/settings.rs`.

**Interfaces:**
- Produces:
  - `pub enum PadSize { Small, Medium, Large }` (`Ord`; `fn from_ardent(n: u8) -> Option<Self>` 1/2/3; `fn fits(self, ship: PadSize) -> bool` = `self >= ship`).
  - `pub enum NearbyService { InterstellarFactors }` (ampliable).
  - `NetQuery` + `CommodityImportsNearby { system_address: u64, commodity: String, max_distance_ly: u32, min_pad: PadSize }`, `NearestService { system_address: u64, service: NearbyService }`.
  - `NetSource` + `Ardent`.
  - `RemoteMarket { station_name: String, system_name: String, market_id: Option<u64>, distance_ly: f64, distance_to_arrival_ls: Option<f64>, sell_price: u64, demand: Option<u64>, max_pad: Option<PadSize>, station_type: Option<String>, updated_at: Option<DateTime<Utc>> }`; `RemoteStation { station_name, system_name, distance_ly: f64, distance_to_arrival_ls: Option<f64>, max_pad: Option<PadSize>, updated_at: Option<DateTime<Utc>> }`.
  - `NetOutcome` + `Markets(Vec<RemoteMarket>)`, `Stations(Vec<RemoteStation>)`.
  - `ModuleSettings` + `mining_interest: Vec<String>` (claves `commodity_key`), `mining_threshold_cr: u64`, `sell_search_radius_ly: u32`; `Default`: lista `DEFAULT_MINING_INTEREST` = `["platinum","painite","lowtemperaturediamond","opal","alexandrite","grandidierite","monazite","musgravite","rhodplumsite","serendibite","benitoite"]` (claves verificadas en Task 1), `1_000_000`, `100`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn commodity_imports_query_roundtrips_json() { /* CommodityImportsNearby{system_address:10477373803, commodity:"platinum", max_distance_ly:100, min_pad:Large} → from_str(to_string(x))==x */ }
#[test] fn markets_outcome_roundtrips_with_ardent_source() { /* NetResponse{source:Ardent, outcome:Markets(vec![RemoteMarket{sell_price:220000, max_pad:Some(Medium),..}])} roundtrip */ }
#[test] fn pad_size_fits() { /* Large.fits(Medium)==true, Medium.fits(Large)==false, from_ardent(3)==Some(Large), from_ardent(9)==None */ }
#[test] fn module_settings_mining_defaults() { /* mining_threshold_cr 1_000_000, sell_search_radius_ly 100, mining_interest contiene "platinum" y len 11; los valores de F1 no cambian */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar; brazos nuevos en `match` exhaustivos de `tmc-net` (`retry_at`, `is_down_for`, `execute_inner` → `Disabled` provisional hasta Task 7). **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(model): consultas de Ardent, tamaño de plataforma y ajustes de minería`

### Task 7: `tmc-net` — adaptador Ardent

**Files:** Create `crates/tmc-net/src/ardent.rs`; Modify `client.rs`, `health.rs`, `limiter.rs`, `cache_policy.rs`, `lib.rs`.

**Interfaces:**
- Consumes: `Transport` (`get`), `Service` (verificar en código al ejecutar), fixtures de Task 1.
- Produces:
  - `Service::Ardent`; límite 1 petición/s; salud como F1.
  - `ardent::imports_nearby_url(system_address: u64, commodity: &str, max_distance_ly: u32, min_pad: PadSize) -> String` (con `fleetCarriers=false`, `minVolume` y `maxDaysAgo` del spike; añade `minLandingPadSize` solo si el spike confirma que Ardent lo acepta, si no `min_pad` se ignora); `ardent::nearest_url(system_address: u64, service: NearbyService) -> String`.
  - `ardent::parse_markets(body: &str) -> Result<Vec<RemoteMarket>, NetFailure>` (excluye obras de construcción/colonización y carriers; tolera `demandBracket` texto o número; ordena por `sell_price` desc; máximo 50 entradas); `ardent::parse_stations(body: &str) -> Result<Vec<RemoteStation>, NetFailure>` (máx. 5, por distancia).
  - `cache_policy`: clave `ardent:imports:<addr>:<commodity>:<radio>:<pad>` TTL 15 min; `ardent:nearest:<addr>:<servicio>` TTL 1 h.
  - `NetClient::execute_inner`: brazos nuevos → `(NetSource::Ardent, …)`; `retry_at`/`is_down_for` con `Service::Ardent`; respeta `online_allowed` (como F1, revalidado tras esperar turno); la URL base de Ardent es sustituible por la misma vía que usan los servicios falsos de F1 en e2e (verificar en código al ejecutar; la usa Task 26).

- [ ] **Step 1: Tests** (en `ardent.rs` y `client.rs`, con `FakeTransport` existente):

```rust
#[test] fn imports_url_has_filters() { /* addr 10477373803, "platinum", 100 → contiene "/v2/system/address/10477373803/commodity/name/platinum/nearby/imports", "maxDistance=100", "fleetCarriers=false" */ }
#[test] fn parses_markets_fixture_sorted_by_price() { /* ardent_nearby_imports_platinum.json → primer sell_price == <valor del fixture>, max_pad == Some(Large) */ }
#[test] fn construction_sites_and_carriers_are_excluded() { /* el fixture incluye 1 obra y 1 carrier → ninguno en el resultado; len == <n esperado> */ }
#[test] fn demand_bracket_accepts_string_and_number() { /* entradas con "demandBracket":"3" y 3 → ambas parsean */ }
#[test] fn empty_imports_is_empty_markets() { /* ardent_nearby_imports_empty.json → Ok(vec![]) */ }
#[test] fn parses_nearest_interstellar_factors() { /* ardent_nearest_if.json → [0].station_name == <valor>, distance_ly == <valor> */ }
#[tokio::test] async fn disabled_online_services_never_reach_ardent() { /* online=false → (Cache, Disabled), FakeTransport.total()==0 */ }
#[tokio::test] async fn ardent_respects_one_request_per_second() { /* 3 consultas distintas con tiempo pausado → la 3.ª sale ≥ 2 s después de la 1.ª */ }
#[tokio::test] async fn three_ardent_failures_mark_down() { /* 3×HTTP 503 → is_down_for(q)==true; 4.ª → Failed(Down) sin tocar transporte */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar. **Step 4:** PASS; Checks.
- [ ] **Step 5: Commit** `feat(net): adaptador de Ardent para mercados cercanos e Interstellar Factors`

### Task 8: configuración nueva en `config.toml` y `PUT /api/config`

**Files:** Modify `crates/tmc-app/src/config.rs`, `crates/tmc-app/src/wiring.rs`, `crates/tmc-server/src/rest.rs` (verificar en código al ejecutar dónde se valida el `PUT`).

**Interfaces:**
- Produces: `Config` + `mining_interest: Vec<String>`, `mining_threshold_cr: u64`, `sell_search_radius_ly: u32` con `#[serde(default)]` (un `config.toml` de F1 carga sin error); validación del `PUT`: lista ≤ 30 entradas, cada una normalizada con `commodity_key` y ≤ 40 caracteres; umbral ≤ 1 000 000 000; radio 10–500. Mapeo a `ModuleSettings` y difusión en caliente (`SourceEvent::Settings`) como los ajustes de F1.

- [ ] **Step 1: Tests:**

```rust
#[test] fn f1_config_without_mining_fields_loads_with_defaults() { /* TOML de F1 → mining_threshold_cr 1_000_000, radio 100, interés len 11 */ }
#[test] fn put_config_rejects_radius_out_of_range() { /* {"sell_search_radius_ly":600} → 400 y config sin cambios */ }
#[test] fn put_config_normalizes_interest_list() { /* {"mining_interest":["Platinum","$painite_name;"]} → ["platinum","painite"] guardado y emitido en ModuleSettings */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(app): ajustes de minería en config.toml y en caliente`

### Task 9: datos estáticos `ship_pads.json` y `mining_yield.json`

**Files:** Create `crates/tmc-modules/data/ship_pads.json`, `mining_yield.json`; Modify `crates/tmc-modules/src/data.rs`.

**Interfaces:**
- Produces:
  - `ship_pads.json`: `{ "version": 1, "entries": [{ "ship": "anaconda", "name": "Anaconda", "pad": "L", "source": "...", "verified": true }, …] }` con todas las naves jugables (incluidas Mk II, Type-11 Prospector, Kestrel Mk II…); clave = `Ship` del diario en minúsculas.
  - `mining_yield.json`: `{ "version": 1, "entries": [{ "content": "Low|Medium|High", "tons_per_asteroid": f64, "source": "estimación propia …", "verified": false }] }` (valores iniciales propuestos 10/20/30 t, a ajustar con Task 27).
  - `data::ship_pads() -> &'static ShipPads`; `ShipPads::pad_for(&self, ship: &str) -> Option<PadSize>`, `ShipPads::display_name(&self, ship: &str) -> Option<&str>` (respaldo de `Ship_Localised`).
  - `data::mining_yield() -> &'static MiningYield`; `MiningYield::tons_for(&self, c: AsteroidContent) -> Option<f64>` (`Unknown` → `None`).
  - `fn parse_ship_pads(json: &str) -> ShipPads` / `fn parse_mining_yield(json: &str) -> MiningYield` tolerantes (entrada inválida ignorada).

- [ ] **Step 1: Tests:**

```rust
#[test] fn ship_pads_known_sizes() { /* "anaconda"→Large, "python"→Medium, "sidewinder"→Small, "type9"→Large, "nave_inexistente"→None */ }
#[test] fn every_ship_pad_entry_has_source_and_valid_pad() { /* todas con source no vacío y pad en S/M/L; len >= 40 */ }
#[test] fn mining_yield_has_three_levels_unverified() { /* tons_for(High) > tons_for(Medium) > tons_for(Low) > 0; Unknown→None; todas verified==false */ }
#[test] fn corrupt_data_file_loads_empty_without_panic() { /* parse_ship_pads("{oops") → pad_for("anaconda")==None; entrada con pad "X" ignorada y las demás cargan */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(modules): datos propios de plataforma por nave y rendimiento de asteroides`

### Task 10: i18n — `combat.ftl` y `mining.ftl`

**Files:** Create `locales/{es,en,fr,de,pt}/combat.ftl`, `mining.ftl`; Modify `locales/*/main.ftl` (títulos de disposición `layout-combat`, `layout-mining`), `crates/tmc-i18n/src/lib.rs` (si la carga de archivos o el empaquetado de la UI necesita lista explícita — verificar en código al ejecutar).

**Interfaces:**
- Produces: claves base de ambos módulos (títulos de panel, «escaneando…», `combat-badge-wanted = BUSCADO · { $reward }`, `combat-badge-clean = LIMPIO · NO DISPARAR`, estados de calor, nombres de tipo de subsistema `combat-subsystem-{powerplant,drive,fsd,lifesupport,distributor,sensors,weapons,cargohatch,other}`, `mining-verdict-worth = ✔ merece la pena · ≈ { $credits }`, `mining-verdict-not-worth`, `mining-no-data`…). Las tareas de UI añaden las que falten.
- Lista de argumentos del test de formato ampliada con `reward`, `credits`, `price`, `rate`, `tons`, `fee`, `faction`, `ship`, `material`, `remaining`, `distance`, `days` (misma lista que «Restricciones globales»).

- [ ] **Step 1: Tests:** el test de claves por idioma existente debe fallar al crear solo `es`; `combat_and_mining_resources_load_in_all_languages` → `format("es","combat-badge-clean", {})` == `"LIMPIO · NO DISPARAR"` y `format("en", …)` == `"CLEAN · DO NOT FIRE"`.
- [ ] **Step 2–4:** FAIL → completar 5 idiomas con traducciones reales → PASS; Checks; `npm --prefix ui test` (test de claves de la UI).
- [ ] **Step 5: Commit** `feat(i18n): textos base de combate y minería en cinco idiomas`

---

## Bloque 2 — Combate

Todas las piezas de `combat` son puras (sin E/S): reciben eventos y `now: DateTime<Utc>` (marca de tiempo del evento o `Status`), devuelven si cambiaron.

### Task 11: `combat` — objetivo actual

**Files:** Create `crates/tmc-modules/src/combat/target.rs`, `crates/tmc-modules/src/combat/ax.rs` (solo `THARGOID_SHIPS`, `is_thargoid_ship(key)` e `is_thargoid_faction(f)`, con valores del Spike AX; Task 15 añade `AxTracker`) y el esqueleto `crates/tmc-modules/src/combat/mod.rs` (solo `pub mod`; Task 16 lo completa).

**Interfaces:**
- Produces: `pub enum LegalBadge { Scanning, Wanted { bounty: u64 }, Clean, Lawless, Enemy, Unknown }` (`ScanStage` < 3 → `Scanning`; `LegalStatus` `"Wanted"` o `bounty > 0` → `Wanted`; `"Clean"` → `Clean`; `"Lawless"` → `Lawless`; `"Enemy"`/`"WantedEnemy"`/`"Hostile"` → `Enemy`, salvo `bounty > 0` → `Wanted`; otro → `Unknown`). `pub enum SubsystemKind { PowerPlant, Drive, Fsd, LifeSupport, Distributor, Sensors, Weapons, CargoHatch, Other }` con `fn from_journal(s: &str)`. `TargetView { ship_key, ship_name: Option<String>, pilot, rank, faction, badge: LegalBadge, shield_pct: Option<f64>, hull_pct: Option<f64>, subsystem: Option<SubsystemView { kind, name: Option<String>, health_pct: Option<f64> }>, thargoid: bool, lost_at: Option<DateTime<Utc>> }`. `TargetTracker { fn apply(&mut self, ev: &ShipTargeted, now) -> bool; fn view(&self, now) -> Option<TargetView> }` — `view` devuelve `None` si `lost_at + 10 s <= now`. `ship_name` = `Ship_Localised` o `data::ship_pads().display_name`. `thargoid` = `ax::is_thargoid_ship(&ship_key)` (creado aquí, ampliado en Task 15). `LegalBadge` serializa etiquetado (`#[serde(tag = "kind", rename_all = "snake_case")]`: `{"kind":"wanted","bounty":125000}`) y `SubsystemKind` en `snake_case`; `types.ts` (Task 17) los refleja.

- [ ] **Step 1: Tests:**

```rust
#[test] fn wanted_target_shows_bounty_badge() { /* ScanStage 3, LegalStatus "Wanted", Bounty 125000 → Wanted{bounty:125000} */ }
#[test] fn clean_target_shows_clean_badge() { /* ScanStage 3, "Clean", Bounty None → Clean */ }
#[test] fn scan_stage_below_3_is_scanning() { /* ScanStage 1, "Wanted" → Scanning */ }
#[test] fn unknown_legal_status_is_unknown_badge() { /* ScanStage 3, "Foo" → Unknown */ }
#[test] fn subsystem_kind_and_health() { /* Subsystem "$int_powerplant_size3_class5_name;", SubsystemHealth 62.0 → PowerPlant, Some(62.0) */ }
#[test] fn target_lost_sets_lost_at_and_keeps_data() { /* lock, luego TargetLocked:false a t0 → view(t0+9s).hull_pct == Some(80.0), lost_at==Some(t0) */ }
#[test] fn view_clears_10s_after_loss() { /* view(t0+10s) == None */ }
#[test] fn new_lock_after_loss_resets_lost_at() { /* lock nuevo a t0+5s → lost_at None */ }
#[test] fn legal_badge_serializes_tagged() { /* Wanted{bounty:125000} → {"kind":"wanted","bounty":125000}; Clean → {"kind":"clean"} */ }
#[test] fn identical_targeted_event_reports_no_change() { /* apply dos veces el mismo → segunda devuelve false */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(combat): seguimiento del objetivo con estado legal y subsistema`

### Task 12: `combat` — mi nave

**Files:** Create `crates/tmc-modules/src/combat/ship.rs`.

**Interfaces:**
- Consumes: `Status.flags` (`SHIELDS_UP`, `OVERHEATING`, `IS_IN_DANGER`, `BEING_INTERDICTED`, `HARDPOINTS_DEPLOYED`), `Status.pips: Option<[u8; 3]>` (medios pips 0–8), eventos `Loadout`, `HullDamage`, `RepairAll`, `Repair`, `ShieldState`, `HeatWarning`, `HeatDamage`, `UnderAttack`, `Interdicted`, `Died`, `Resurrect`.
- Produces:
  - `Pips { sys: f32, eng: f32, wep: f32 }` con `fn from_half_pips(p: [u8; 3]) -> Pips` (valores en unidades 0.0–4.0 con paso 0.5).
  - `pub enum HeatState { Normal, Overheating, HeatDamage }`; `HeatTracker { fn on_status(&mut self, overheating: bool, now) -> bool; fn on_heat_warning(&mut self, now) -> bool; fn on_heat_damage(&mut self, now) -> bool; fn state(&self) -> HeatState; fn overheating_since(&self) -> Option<DateTime<Utc>>; fn overheat_total_s(&self) -> u64 }` — `HeatDamage` persiste hasta que `Status` llega sin `OVERHEATING` o pasan 10 s sin otro `HeatDamage`; `overheating_since` se fija al entrar en `Overheating`/`HeatDamage` y se borra al volver a `Normal` (acumulando en `overheat_total_s`).
  - `HullTracker { fn apply(&mut self, ev) -> bool; fn pct(&self) -> Option<u8> }` — escalón = último valor conocido redondeado a entero: `Loadout.HullHealth` y `HullDamage.Health` (×100; se ignora `HullDamage` con `Fighter == true` o `PlayerPilot == false`), `RepairAll` → 100, `Repair` con ítem de casco (`"Hull"`/`"Wear"`, verificar en diario) → 100, `Died` → `None`, `Resurrect` → 100. **Sin duplicar lógica:** `state.rs` ya guarda `ship.hull_health` desde `Loadout`/`HullDamage` (filtro `PlayerPilot`); `HullTracker` parte de ese valor y solo añade lo que `GameState` no hace (`Fighter`, `RepairAll`/`Repair`, `Died`, `Resurrect`).
  - `ShipView { shields_up: Option<bool>, hull_pct: Option<u8>, heat: HeatState, overheating_since, overheat_total_s, pips: Option<Pips>, hardpoints: bool, alerts: Vec<ShipAlert> }`; `pub enum ShipAlert { HullLow, ShieldsDown, Overheating, UnderAttack, Interdiction }` (`HullLow` < 25 %; `UnderAttack` 30 s tras el evento; `Interdiction` mientras `BEING_INTERDICTED`).
  - `ShipTracker` que compone lo anterior: `fn on_status(&mut self, s: &Status, now) -> bool`, `fn on_event(&mut self, ev: &EventKind, now) -> bool`, `fn view(&self, now) -> ShipView`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn pips_half_units_exact() { /* [4,8,0] → (2.0,4.0,0.0); [3,5,4] → (1.5,2.5,2.0); [8,2,2] → (4.0,1.0,1.0) */ }
#[test] fn hull_steps_follow_last_known_value() { /* Loadout HullHealth 1.0 → 100; HullDamage 0.8 → 80; HullDamage 0.6 → 60; RepairAll → 100 */ }
#[test] fn fighter_hull_damage_is_ignored() { /* HullDamage{Health:0.2, Fighter:true} → sigue 100 */ }
#[test] fn heat_three_states_and_timer() { /* status overheating a t0 → Overheating, since t0; HeatDamage a t0+3s → HeatDamage; status sin flag a t0+8s → Normal, overheat_total_s == 8 */ }
#[test] fn heat_damage_expires_after_10s_without_repeat() { /* HeatDamage t0, status overheating t0+11s → Overheating */ }
#[test] fn alerts_hull_low_and_shields_down() { /* hull 20 % y SHIELDS_UP ausente → alerts == [HullLow, ShieldsDown] (orden fijo) */ }
#[test] fn identical_status_burst_marks_ship_dirty_once() { /* 500 Status idénticos → on_status true solo la 1.ª vez */ }
#[test] fn died_clears_hull_and_heat() { /* Died → hull None, heat Normal, since None */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(combat): estado de mi nave con pips exactos, casco por escalones y calor de tres estados`

### Task 13: `combat` — libro de vales

**Files:** Create `crates/tmc-modules/src/combat/vouchers.rs`.

**Interfaces:**
- Produces: `pub enum VoucherKind { Bounty, CombatBond }` (`FactionKillBond` y `CapShipBond` → `CombatBond`; los bonos AX son `CombatBond` con `ax: true`). `VoucherEntry { kind, faction: String, amount: u64, system: String, earned_at: DateTime<Utc>, ax: bool }` (`ax` = `ax::is_thargoid_faction(victim_faction)`, de Task 11). `VoucherLedger { fn apply_event(&mut self, ev: &EventKind, system: &str, now) -> bool; fn unredeemed(&self) -> Vec<FactionTotal { kind, faction, amount, systems: Vec<String>, ax: bool }>; fn total(&self, kind: VoucherKind) -> u64; fn reset(&mut self) }`.
- Reglas (spec §10): `Bounty` → una entrada por `normalized_rewards`; `RedeemVoucher` con `Type` `"bounty"` → `Bounty`, `"CombatBond"` → `CombatBond`, otros tipos ignorados; con `Factions[]` resta por facción su importe; con `Faction` resta de esa facción; sin facción resta `Amount` del tipo empezando por la entrada más antigua; con `BrokerPercentage` aplica la regla del Spike AX (nominal = `Amount` o `Amount/(1−p/100)`), codificada en `const BROKER_AMOUNT_IS_NET: bool` y `fn nominal(amount: u64, pct: f64, is_net: bool) -> u64`; nunca baja de 0. `Died` → `reset()`. Orden FIFO estable por `earned_at`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn bounties_and_bonds_accumulate_by_faction_and_kind() { /* Bounty [{A,50000},{B,10000}] + FactionKillBond A 20000 → Bounty A 50000, Bounty B 10000, CombatBond A 20000; total(Bounty)==60000 */ }
#[test] fn redeem_with_factions_subtracts_per_faction() { /* RedeemVoucher bounty Factions [{A,50000}] → Bounty A 0, B 10000 */ }
#[test] fn redeem_combat_bond_with_single_faction() { /* RedeemVoucher CombatBond Faction A Amount 20000 → CombatBond total 0 */ }
#[test] fn redeem_without_faction_subtracts_oldest_first() { /* Bounty A 30000 (t0), B 20000 (t1); RedeemVoucher bounty Amount 40000 → A 0, B 10000 */ }
#[test] fn broker_redeem_clears_face_value() { /* Bounty A 50000; RedeemVoucher bounty Amount 37500 BrokerPercentage 25, sin Factions → total(Bounty)==0 con `BROKER_AMOUNT_IS_NET = true` (regla de Task 0; si el spike dice lo contrario, se invierte la constante y la entrada del test) */ }
#[test] fn nominal_value_rule_both_variants() { /* nominal(37500, 25.0, true)==50000 y nominal(50000, 25.0, false)==50000 */ }
#[test] fn other_voucher_types_are_ignored() { /* RedeemVoucher "scannable" 1000 → sin cambios, apply_event false */ }
#[test] fn died_zeroes_all_unredeemed_vouchers() { /* tras 3 vales, Died → unredeemed() vacío */ }
#[test] fn never_goes_negative() { /* Redeem 999999 sin vales → total 0 */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(combat): libro de vales sin cobrar con reglas de canje y muerte`

### Task 14: `combat` — sesión de caza y reconstrucción

**Files:** Create `crates/tmc-modules/src/combat/session.rs`, `tests/fixtures/journal/combat_session.log` (sintético, contenido descrito en Task 26; Task 26 lo completa y valida).

**Interfaces:**
- Consumes: `VoucherLedger`, eventos `LoadGame`, `Bounty`, `FactionKillBond`, `CapShipBond`, `PvpKill`, `Rank`, `Progress`, `Promotion`; paginador de historial de `exploration::unsold` (orden `(ts, archivo, línea, id)`, páginas de 256 — verificar en código al ejecutar; si es privado, moverlo a `crate::history` en esta tarea sin cambiar su comportamiento).
- Produces: `HuntingSession { started_at, kills: u32, assists: u32, credits: u64, kills_by_type: BTreeMap<String, u32>, combat_rank: Option<u8>, combat_progress: Option<u8> }` + `fn apply_event(&mut self, ev, now) -> bool`, `fn credits_per_hour(&self, now) -> Option<u64>` (`None` si < 60 s), `fn reset(&mut self, now)`. **Decisión del redactor:** baja = `Bounty`/`FactionKillBond`/`CapShipBond` sin `SharedWithOthers` > 0 o `PvpKill`; asistencia = `SharedWithOthers` > 0 (a confirmar en Task 0/2). La sesión empieza en `LoadGame` o en la acción `reset_session`.
- Reconstrucción al arrancar: `fn rebuild_task(cmdr: String) -> StoreTask` que lee `event` del comandante desde el último `Died` y aplica `VoucherLedger::apply_event` siguiendo `Location`/`FsdJump` para conocer el `system` de cada vale (resultado `Stored { tag: "combat.vouchers" }` con el libro serializado y el `cmdr`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn session_counts_kills_assists_and_credits() { /* 3 Bounty (100k, 200k, 300k), 1 con SharedWithOthers 1 → kills 2, assists 1, credits 600000 */ }
#[test] fn credits_per_hour_after_30_min() { /* 600000 cr en 30 min → Some(1_200_000) */ }
#[test] fn credits_per_hour_none_under_one_minute() { /* 30 s → None */ }
#[test] fn promotion_updates_combat_rank() { /* Progress Combat 40 → progress 40; Promotion Combat 5 → rank 5 */ }
#[test] fn reset_session_keeps_ledger() { /* reset → kills 0, pero ledger.total(Bounty) intacto */ }
#[test] fn ledger_rebuilt_after_restart_matches_live_ledger() { /* store en memoria con combat_session.log importado → rebuild_task(cmdr) produce el mismo unredeemed() que aplicar el fixture en vivo */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(combat): sesión de caza y reconstrucción de vales desde el historial`

### Task 15: `combat` — zona de conflicto y AX

**Files:** Create `crates/tmc-modules/src/combat/cz.rs`, `tests/fixtures/journal/ax_session.log` (según Task 0; sintético si no hay datos reales); Modify `crates/tmc-modules/src/combat/ax.rs` (creado en Task 11: aquí se añade `AxTracker`).

**Interfaces:**
- Produces:
  - `ConflictZone { system: String, side: Option<String>, kills: u32, bonds_by_faction: BTreeMap<String, u64> }`; `CzTracker { fn apply_event(&mut self, ev, system, now) -> bool; fn view(&self) -> Option<ConflictZone> }` — activo desde el primer `FactionKillBond` no thargoide en el sistema; `side` = `AwardingFaction` del primero; se cierra en `FsdJump`, `Docked` o `Died`.
  - `THARGOID_SHIPS`, `is_thargoid_ship` e `is_thargoid_faction` ya existen (Task 11); aquí solo se contrastan con Task 0. El bando de la ZC es el `AwardingFaction` del primer bono (los `Comms` no se tipan en F2: omisión a anotar en «Desviaciones»).
  - `AxTracker { kills_by_type: BTreeMap<String, u32>, bonds: u64, presence: bool, last_activity: Option<DateTime<Utc>> }` + `fn apply_event(&mut self, ev, now) -> bool`, `fn on_location(&mut self, loc: &LocationInfo) -> bool` (usa `thargoid_presence`), `fn active(&self, now) -> bool` (presencia o actividad ≤ 5 min); recuerda el `Ship` del último `ShipTargeted` thargoide (`last_target_ship`) para atribuir cada bono AX a un tipo.

- [ ] **Step 1: Tests:**

```rust
#[test] fn cz_side_and_bonds_by_faction() { /* 2 FactionKillBond A (20000, 30000) → side Some("A"), kills 2, bonds A 50000 */ }
#[test] fn cz_closes_on_jump() { /* FsdJump → view None */ }
#[test] fn thargoid_bond_goes_to_ax_not_cz() { /* FactionKillBond VictimFaction thargoide 6_000_000 → ax.bonds 6_000_000, cz.view None */ }
#[test] fn ax_kills_by_type() { /* ShipTargeted Ship <scout confirmado> seguido de bono AX → kills_by_type[<scout>]==1 */ }
#[test] fn thargoid_presence_from_location() { /* Location con ThargoidWar CurrentState "Thargoid_Probing" → presence true; FsdJump a sistema limpio → false */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(combat): zona de conflicto y seguimiento AX`

### Task 16: `combat` — módulo, topics, relevancia, red y registro

**Files:** Create `crates/tmc-modules/src/inflight.rs`, `crates/tmc-modules/src/combat/tests.rs`; Modify `crates/tmc-modules/src/combat/mod.rs` (esqueleto de Task 11), `crates/tmc-modules/src/{lib.rs, registry.rs}`.

**Interfaces:**
- Consumes: `Module`, `ModuleCtx::{request, store_task, alert}`, `SourceEvent::{Journal, Status, Net, Stored, Settings, Action}`, `GameState` (sistema, galaxia, comandante — verificar nombres de campos en código al ejecutar).
- Produces: `pub struct Combat` con `Combat::new()`; `id() = "combat"`; topics `combat.target`, `combat.ship`, `combat.session`, `combat.conflict_zone` (`null` fuera de una ZC), `combat.ax { active, presence, kills_by_type, bonds, target }` (formato de nombre de topic como en `exploration` — verificar); `combat.session` incluye `vouchers: Vec<FactionTotal>` (máx. 32 filas; el resto agregado en «otras»), `nearest_if: Option<RemoteStation>`, `if_fee_pct: 25`.
  - Red: tras `FsdJump`/`Location`, si hay vales sin cobrar, `online_services` y galaxia Live → `ctx.request("if:<addr>", NetQuery::NearestService { system_address, service: InterstellarFactors })`; respuesta aplicada solo si `addr` sigue siendo el sistema actual; control de vuelo con `inflight::Inflight { fn begin(&mut self, key: &str, now) -> bool /* false si ya en vuelo y no caducada */; fn finish(&mut self, key: &str); }` (TTL `INFLIGHT_TTL_S = 60`).
  - Acción `reset_session` → `HuntingSession::reset`.
  - `relevance`: `Active` si objetivo fijado, `UnderAttack` ≤ 30 s, hardpoints desplegados con ZC activa, o `AxTracker::active` (spec §3.1); si no, `Normal`. Sin reloj: usa `last_ts` (Restricciones).
  - Alertas (las `ShipAlert` de Task 12 al activarse, solo en vivo y con `alerts_enabled`) con claves i18n de `combat.ftl`.

- [ ] **Step 1: Tests** (en `combat/tests.rs`):

```rust
#[test] fn combat_session_fixture_produces_expected_topics() { /* combat_session.log → combat.target badge Wanted{bounty:<valor>}, combat.session.vouchers suma == <valor> */ }
#[test] fn replay_emits_no_requests_or_store_writes() { /* mismo fixture con replaying=true → take_outputs() sin NetRequest ni StoreTask */ }
#[test] fn no_requests_in_legacy_or_offline() { /* galaxia Legacy → 0; online_services=false → 0 */ }
#[test] fn stored_result_for_other_cmdr_is_ignored() { /* Stored{tag:"combat.vouchers"} con cmdr "OTRO" → vouchers sin cambios */ }
#[test] fn late_if_response_for_previous_system_not_applied() { /* salto a X, luego a Y; llega Stations de X → nearest_if None */ }
#[test] fn inflight_if_request_expires_after_60s() { /* sin respuesta, nuevo FsdJump al mismo sistema a t+61s → se emite de nuevo */ }
#[test] fn relevance_active_with_locked_target() { /* ShipTargeted TargetLocked true → Active; TargetLocked false y un Status 11 s después → Normal */ }
#[test] fn topics_stay_under_16_kib_with_200_factions() { /* 200 facciones con vales → combat.session serializado < 16384 bytes y 33 filas */ }
#[test] fn reset_session_action_zeroes_session_only() { /* Action reset_session → session.kills 0, vouchers sin cambios */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(combat): módulo de combate con topics, relevancia y consulta de Interstellar Factors`

### Task 17: UI — paneles «Objetivo» y «Mi nave»

**Files:** Create `ui/src/modules/combat/{index.ts, types.ts, TargetPanel.svelte, ShipPanel.svelte, PipsGauge.svelte}`; Modify `ui/src/modules/registry.ts`, `locales/*/combat.ftl`; Create `ui/tests/combat.test.ts`.

**Interfaces:**
- Consumes: topics `combat.target`, `combat.ship` (tipos espejo en `types.ts`; `badge` etiquetado `{kind:'wanted',bounty}` como Task 11).
- Produces: según spec §3.2 / maqueta v2: insignia grande arriba a la derecha (BUSCADO rojo / LIMPIO · NO DISPARAR verde / «escaneando…»); barras de escudo/casco del objetivo; recuadro de subsistema (nombre grande, icono por `SubsystemKind`, barra). El panel se vacía con un `setTimeout` único a `lost_at + 10 s`. «Mi nave»: escudos, casco por escalones, calor de 3 estados con cronómetro (`setInterval` 1 s solo si `overheating_since`), `PipsGauge` 3 columnas × 4 unidades con medio pip, alertas.

- [ ] **Step 1: Tests** (Vitest + Testing Library):

```ts
test('wanted badge shows localized bounty', /* target badge Wanted 125000 → texto contiene "BUSCADO" y "125.000" (formato es) */);
test('clean badge', /* → "LIMPIO · NO DISPARAR" */);
test('pips gauge renders half pips', /* pips {sys:1.5,eng:2.5,wep:2} → 1 + 2 + 2 unidades llenas y 2 medias (data-state="half") */);
test('heat timer only runs while overheating', /* heat Normal → vi.getTimerCount()==0; Overheating → 1 */);
test('target panel empties 10 s after lost_at', /* vi.advanceTimersByTime(10000) → muestra estado vacío */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; `npm run check/test/build`; Checks.
- [ ] **Step 5: Commit** `feat(ui): paneles de objetivo y de mi nave`

### Task 18: UI — «Sesión de caza», «Zona de conflicto» y «AX»

**Files:** Create `ui/src/modules/combat/{HuntingPanel.svelte, ConflictZonePanel.svelte, AxPanel.svelte}`; Modify `index.ts`, `locales/*/combat.ftl`, `ui/tests/combat.test.ts`.

**Interfaces:** Sesión: bajas, asistencias, créditos, créditos/hora (≈), vales sin cobrar por facción y tipo con sistema(s) donde se ganaron, IF más cercano con coste −25 % (`fee`) y antigüedad, progreso de rango, botón «reiniciar sesión» (`POST` acción con JSON + `Content-Type`). ZC: bando, bajas, bonos por facción. AX: objetivo thargoide, bonos y bajas por tipo, aviso de presencia.

- [ ] **Step 1: Tests:**

```ts
test('vouchers grouped by faction and kind with IF fee', /* Bounty A 50000 + IF → muestra "50.000" y coste "12.500" */);
test('reset session posts json action', /* click → fetch con method POST, headers Content-Type application/json */);
test('no IF data shows no-data label', /* nearest_if null → texto de combat-no-data */);
test('ax panel lists kills by type and presence banner', /* {kills_by_type:{scout:3}, presence:true} → "3" y aviso */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks UI y Rust (claves). **Step 5: Commit** `feat(ui): paneles de sesión de caza, zona de conflicto y AX`

---

## Bloque 3 — Minería

### Task 19: `mining` — prospector

**Files:** Create `crates/tmc-modules/src/mining/prospector.rs`, `crates/tmc-modules/src/mining/prices.rs` (solo `PriceSource`, `PriceInfo` y `pub type PriceFn<'a> = &'a dyn Fn(&str) -> Option<PriceInfo>`; Task 21 añade `PriceBook`), `crates/tmc-modules/src/mining/mod.rs` (esqueleto con `pub mod`; Task 22 lo completa), `tests/fixtures/journal/mining_session.log` (sintético; Task 26 lo completa y valida).

**Interfaces:**
- Consumes: `ProspectedAsteroid`, `data::mining_yield`, `PriceFn` (definido aquí en `prices.rs`; la resolución real es `PriceBook::best_price` de Task 21), `ModuleSettings::{mining_interest, mining_threshold_cr}`.
- Produces: `MaterialRow { key: String, name: Option<String>, proportion: f64, est_value: Option<u64>, interest: bool }`; `pub enum Verdict { Worth { est_value: Option<u64> }, NotWorth, NoData }`; `AsteroidView { content: AsteroidContent, motherlode: Option<String>, materials: Vec<MaterialRow>, remaining: Option<f64>, verdict: Verdict, est_total: Option<u64> }`; `ProspectorTracker { fn apply(&mut self, ev: &ProspectedAsteroid, prices, settings) -> bool; fn current(&self) -> Option<&AsteroidView>; fn history(&self) -> &VecDeque<AsteroidView> /* máx. 8 */ }`.
- `Verdict` serializa etiquetado (`{"kind":"worth","est_value":2079000}`, `not_worth`, `no_data`) como `LegalBadge`; `types.ts` (Task 23) lo refleja.
- Fórmula (spec §3.3): `est_value = round(proportion/100 × tons_for(content) × price)`. **Decisión del redactor:** el veredicto suma solo materiales de la lista de interés: `Worth` si `motherlode ∈ interés` o suma ≥ umbral; sin precio para ningún material de interés → `Worth` si hay material de interés con contenido `High`, si no `NoData`; en otro caso `NotWorth`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn est_value_formula() { /* platinum 31.5 %, High (30 t), precio 220000 → 2_079_000 */ }
#[test] fn verdict_worth_over_threshold() { /* el anterior con umbral 1_000_000 → Worth{Some(2_079_000)} */ }
#[test] fn verdict_worth_for_motherlode_in_interest() { /* Low, 5 % bauxita, motherlode "Monazite" → Worth */ }
#[test] fn verdict_not_worth_below_threshold() { /* platinum 3 %, Low (10 t), 220000 → 66_000 → NotWorth */ }
#[test] fn verdict_without_prices_uses_interest_and_high_content() { /* sin precios, High con platinum → Worth{None}; Medium → NoData */ }
#[test] fn history_keeps_last_8() { /* 10 asteroides → history.len()==8, el más antiguo es el 3.º */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(mining): prospector con composición, valor estimado, veredicto e histórico`

### Task 20: `mining` — sesión de minería

**Files:** Create `crates/tmc-modules/src/mining/session.rs`.

**Interfaces:**
- Consumes: `MiningRefined`, `LaunchDrone`, `ProspectedAsteroid`, `AsteroidCracked`, `Cargo` (evento e inventario), `Loadout.cargo_capacity`, `EjectCargo`, `MarketSell`, `LoadGame`.
- Produces: `MiningSession { started_at: Option<DateTime<Utc>>, refined: BTreeMap<String, u32>, limpets_launched: BTreeMap<String, u32>, limpets_left: Option<u32>, cargo: Option<u32>, capacity: Option<u32>, asteroids_prospected: u32, cracked: u32 }` + `fn apply_event(&mut self, ev, now) -> bool`, `fn tons_per_hour(&self, now) -> Option<f64>` (`None` si < 60 s), `fn est_credits(&self, prices: PriceFn) -> Option<u64>`, `fn credits_per_hour(&self, now, prices: PriceFn) -> Option<u64>`, `fn refinery_bins(&self) -> Vec<(String, u32)>` (máx. 10 tipos más recientes; estimado), `fn reset(&mut self, now)`. Sesión desde `LoadGame` o `reset_session` (spec §10, decisión 4); limpets restantes = `drones` del último `Cargo`, −1 por cada `LaunchDrone` posterior; `cargo` = último `Cargo.count`, +1 por cada `MiningRefined` posterior (estimado hasta el siguiente `Cargo`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn refined_tons_per_mineral() { /* 12 MiningRefined platinum + 3 painite → refined {platinum:12, painite:3} */ }
#[test] fn tons_per_hour_over_30_min() { /* 12 t en 30 min → Some(24.0) */ }
#[test] fn limpets_launched_by_type_and_left_from_cargo() { /* 3 Prospector + 5 Collection; Cargo drones 12 → launched {Prospector:3, Collection:5}, left Some(12) */ }
#[test] fn est_credits_uses_prices() { /* 12 t platinum a 220000 → 2_640_000 */ }
#[test] fn refinery_bins_capped_at_10() { /* 12 tipos distintos → 10 */ }
#[test] fn session_starts_at_load_game_and_reset() { /* LoadGame t0 → started_at Some(t0); reset(t0+1h) → Some(t0+1h) y refined vacío */ }
#[test] fn launch_and_refine_after_cargo_adjust_counters() { /* Cargo Count 40 con drones 12, luego 2 LaunchDrone y 3 MiningRefined → limpets_left Some(10), cargo Some(43) */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(mining): sesión con refinados, t/h, refinería, limpets y carga`

### Task 21: `mining` — `PriceBook` y «Dónde vender»

**Files:** Modify `crates/tmc-modules/src/mining/prices.rs` (creado en Task 19); Create `crates/tmc-modules/src/mining/sell.rs`.

**Interfaces:**
- Consumes: `MarketSell`, contenido de `Market.json` si `SourceEvent::Companion` lo trae (verificar en código al ejecutar; si solo avisa del cambio, se omite `Market.json` y se anota en «Desviaciones»), `NetOutcome::Markets`, `data::ship_pads`, `Loadout.ship`.
- Produces:
  - `pub enum PriceSource { Remote, LocalSell, LocalMarket }`; `PriceInfo { price: u64, source: PriceSource, at: DateTime<Utc> }`; `PriceBook { fn on_market_sell(&mut self, ev, now); fn on_remote(&mut self, commodity: &str, markets: &[RemoteMarket], fetched_at); fn best_price(&self, commodity: &str, now: DateTime<Utc>) -> Option<PriceInfo> }` — prioridad: mejor precio remoto compatible (< 15 min respecto a `now`) > último `MarketSell` > `Market.json`. `fn price_rebuild_task(cmdr: String) -> StoreTask` lee de la tabla `event` el último `MarketSell` por mineral (resultado `Stored { tag: "mining.prices" }` con `cmdr`), para que el respaldo local sobreviva a un reinicio (spec §10).
  - `SellOption { station_name, system_name, distance_ly, sell_price, demand, pad: Option<PadSize>, pad_ok: Option<bool>, updated_at }`; `fn rank_markets(markets: &[RemoteMarket], ship_pad: Option<PadSize>, top: usize) -> Vec<SellOption>` (excluye los que no caben si se conoce `ship_pad`; nave desconocida → no excluye y `pad_ok = None`); `SellPlan { by_commodity: BTreeMap<String, Vec<SellOption>> /* top 5 */, best_station_total: Option<(String, u64)>, fetched_at, source }` con `fn build(cargo, results, ship_pad) -> SellPlan` (la «mejor estación» maximiza la venta total de la carga).

- [ ] **Step 1: Tests:**

```rust
#[test] fn large_pad_market_hidden_for_large_ship_only_when_incompatible() { /* mercado max_pad Medium y nave Large → excluido; nave Medium → incluido, pad_ok Some(true) */ }
#[test] fn unknown_ship_keeps_markets_with_unknown_pad_ok() { /* ship_pad None → incluidos, pad_ok None */ }
#[test] fn price_prefers_recent_remote_over_local() { /* remoto 230000 hace 5 min y MarketSell 210000 → 230000 Remote */ }
#[test] fn price_falls_back_to_last_market_sell() { /* sin remoto → 210000 LocalSell */ }
#[test] fn stale_remote_price_older_than_15_min_falls_back() { /* remoto hace 16 min → LocalSell */ }
#[test] fn prices_rebuilt_from_history_after_restart() { /* store en memoria con MarketSell platinum 210000 y luego 215000, y painite 500000 → price_rebuild_task: platinum 215000 LocalSell, painite 500000 */ }
#[test] fn best_station_maximizes_total_sale() { /* carga 20 platinum + 10 painite; X paga 220000 y 500000, Y solo platinum a 240000 → best == ("X", 9_400_000) */ }
```
(`late_markets_for_previous_system_not_applied` vive en Task 22, donde está el estado del sistema actual.)

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(mining): libro de precios con respaldo local y ranking de mercados de venta`

### Task 22: `mining` — módulo, topics, relevancia, red y registro

**Files:** Create `crates/tmc-modules/src/mining/tests.rs`; Modify `crates/tmc-modules/src/mining/mod.rs` (esqueleto de Task 19), `crates/tmc-modules/src/{lib.rs, registry.rs}`.

**Interfaces:**
- Produces: `pub struct Mining` (`Mining::new()`, `id() = "mining"`); topics `mining.prospector` (actual + histórico de 8), `mining.session`, `mining.sell` (`SellPlan` + `status: "ok" | "no_data" | "disabled" | "loading"`).
  - Red: por cada mineral de la carga (máx. 10) y por cada material de interés del asteroide actual sin precio, `ctx.request("sell:<addr>:<commodity>", CommodityImportsNearby { system_address, commodity, max_distance_ly: settings.sell_search_radius_ly, min_pad })` tras `FsdJump`/`Location`/`Cargo` con cambios; nunca en replay, Legacy u offline; respuestas de otro sistema no se aplican; control de vuelo con `inflight::Inflight` (Task 16; TTL 60 s); `min_pad` = plataforma de la nave si se conoce y `PadSize::Small` (sin filtro) si no; `Failed` reintento según `retry_at`.
  - Precios: `PriceBook::best_price(commodity, now)` es el único origen para el `PriceFn` de prospector y sesión. Al arrancar/`LoadGame`: `store_task(price_rebuild_task(cmdr))`; `Stored { tag: "mining.prices" }` solo se aplica si el comandante (y generación) coincide y nunca en replay.
  - Acción `reset_session` → `MiningSession::reset`. `SourceEvent::Settings` recalcula veredictos.
  - `relevance`: `Active` si `LaunchDrone` Prospector/Collection o `ProspectedAsteroid` ≤ 10 min (sin reloj: `last_ts`, Restricciones); si no, `Normal`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn mining_session_fixture_matches_refined_and_tph() { /* mining_session.log → session.refined == <mapa del fixture>, tons_per_hour == <valor> */ }
#[test] fn prospector_topic_shows_composition_and_verdict() { /* fixture → materials.len()==<n>, verdict Worth */ }
#[test] fn replay_emits_no_requests_or_store_writes() { /* replaying=true → sin NetRequest ni StoreTask */ }
#[test] fn no_requests_in_legacy_or_offline() { /* Legacy → 0; online_services=false → 0 y mining.sell.status "disabled" */ }
#[test] fn stored_result_for_other_cmdr_is_ignored() { /* Stored{tag:"mining.prices"} con cmdr "OTRO" → best_price("platinum", now) sigue None */ }
#[test] fn late_markets_for_previous_system_not_applied() { /* respuesta Markets de X tras saltar a Y → mining.sell sin ella; PriceBook sí la guarda */ }
#[test] fn settings_change_recomputes_verdict() { /* umbral 1_000_000 → Worth; Settings con 5_000_000 → NotWorth y topic sucio */ }
#[test] fn sell_topic_stays_under_16_kib() { /* 10 minerales × 50 mercados → < 16384 bytes, 5 por mineral */ }
#[test] fn relevance_active_after_prospector_launch() { /* LaunchDrone Prospector → Active; un evento 11 min después → Normal */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(mining): módulo de minería con topics, relevancia y consulta de mercados`

### Task 23: UI — panel «Prospector»

**Files:** Create `ui/src/modules/mining/{index.ts, types.ts, ProspectorPanel.svelte}`; Modify `ui/src/modules/registry.ts`, `locales/*/mining.ftl`; Create `ui/tests/mining.test.ts`.

**Interfaces:** según spec §3.3: contenido (bajo/medio/alto), núcleo, barra de composición apilada (SVG/CSS estático), tabla (nombre localizado, %, valor ≈, barra), veredicto grande, restante, histórico de 8 por valor estimado.

- [ ] **Step 1: Tests:**

```ts
test('stacked composition widths follow proportions', /* 31.5 % y 12 % → estilos width "31.5%" y "12%" (formato vía helper) */);
test('worth verdict shows approx value', /* Worth 2079000 → "≈" y "2.079.000" (es) */);
test('no data verdict', /* NoData → texto mining-no-data */);
test('history lists 8 rows', /* 8 entradas → 8 filas */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): panel del prospector`

### Task 24: UI — «Sesión de minería» y «Dónde vender»

**Files:** Create `ui/src/modules/mining/{MiningSessionPanel.svelte, SellPanel.svelte}`; Modify `index.ts`, `locales/*/mining.ftl`, `ui/tests/mining.test.ts`.

**Interfaces:** Sesión: refinados por mineral, t/h, créditos ≈ y créditos/hora, refinería por depósitos (marcada estimada), limpets por tipo y restantes, carga/capacidad, «reiniciar sesión». Vender: por mineral, top 5 con precio, distancia, demanda, plataforma (aviso si `pad_ok` es `null`), antigüedad (relativa, `Intl.RelativeTimeFormat`), fuente; venta estimada total en la mejor estación; «sin datos» si `status` ≠ `ok`.

- [ ] **Step 1: Tests:**

```ts
test('sell panel shows price distance pad and age', /* 1 opción → "220.000", "12,3 al", "M", "hace 2 h" */);
test('disabled status shows no data without error', /* status "disabled" → mining-no-data, sin console.error */);
test('session shows tons per hour', /* 24 → "24 t/h" */);
test('reset posts json action', /* POST con Content-Type application/json */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): paneles de sesión de minería y dónde vender`

---

## Bloque 4 — Disposiciones, ajustes, fixtures, eval y cierre

### Task 25: UI — disposiciones «Combate» y «Minería», relevancia y ajustes

**Files:** Modify `ui/src/shell/layouts.ts`, `ui/src/settings/Settings.svelte`, `locales/*/main.ftl`, `ui/tests/layouts.test.ts`; (selección automática de disposición por relevancia: verificar en código al ejecutar dónde vive — `App.svelte`/`Sidebar.svelte`).

**Interfaces:**
- Produces: `layouts` + `{ id: 'combat', titleKey: 'layout-combat', icon: 'crosshair', slots: [target span 2, hunting 1, ship 1, ax 2 when axVisible, cz 2 when czVisible] }` y `{ id: 'mining', titleKey: 'layout-mining', icon: 'pickaxe', slots: [prospector 2, mining-session 1, sell 1] }`; `export function autoLayout(mods, manual): 'exploration' | 'combat' | 'mining' | null` (manual manda; `combat` Active/Urgent > `mining` Active > nada). Visibilidad AX/ZC desde los topics (`combat.ax.active`, `combat.conflict_zone != null`).
- Ajustes: lista de interés (casillas de minerales conocidos + texto), umbral (número) y radio (10–500) → `PUT /api/config` con JSON y `Content-Type`; error HTTP revierte al último confirmado (como F1).

- [ ] **Step 1: Tests:**

```ts
test('combat active switches to combat layout', /* [{id:'combat',relevance:'active'}] → 'combat' */);
test('manual choice wins', /* manual 'mining' con combat active → 'mining' */);
test('ax and cz slots hidden when not applicable', /* ax inactive, cz null → 3 paneles visibles */);
test('settings put sends json with content type and reverts on 400', /* 400 → valor anterior */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): disposiciones de combate y minería con relevancia y ajustes de minería`

### Task 26: fixtures sintéticos y pruebas de extremo a extremo

**Files:** Create `tests/fixtures/status/combat_{pips_half,overheating,normal}.json`; Modify (creados en Tasks 14, 15 y 19) `tests/fixtures/journal/{combat_session.log, ax_session.log, mining_session.log}` para completar el contenido descrito abajo; Modify las pruebas e2e/rendimiento de F1 (verificar en código al ejecutar su ubicación).

**Interfaces:** Fixtures escritos a mano, `CMDR TESTER`, sistemas/facciones/pilotos ficticios, sin FID real. `combat_session.log`: objetivo limpio y buscado, subsistema, 6 `Bounty` (uno compartido), 2 `FactionKillBond`, `HullDamage`, `HeatWarning`/`HeatDamage`, `RedeemVoucher` parcial con `Factions[]` y otro sin facción, `Died` y vales posteriores. `mining_session.log`: 10 `ProspectedAsteroid` (uno con núcleo), `MiningRefined` conocidos, `LaunchDrone`, `Cargo`, `MarketSell`. `ax_session.log` según Task 0.

- [ ] **Step 1: Tests:**

```rust
#[test] fn every_line_of_f2_fixtures_is_typed() { /* 0 errores; eventos en Other == EXPECTED_OTHER explícito */ }
#[test] fn combat_fixture_vouchers_match_redeemed_sum() { /* criterio 4: suma ganada antes del canje == suma de RedeemVoucher (±0); tras Died, unredeemed == solo lo posterior: <valor> */ }
#[test] fn combat_fixture_ship_states() { /* criterio 3: pips [3,5,4] → (1.5,2.5,2.0); casco 100→80→60; calor Normal→Overheating→HeatDamage→Normal */ }
#[test] fn ax_fixture_kills_bonds_and_presence() { /* criterio 7: kills_by_type, bonds y presence según fixture */ }
#[test] fn burst_of_1000_combat_events_stays_under_10hz_and_500ms() { /* criterio 2: p95 evento→UI < 500 ms y ≤ 10 difusiones/s */ }
```
  más e2e Playwright (servidor con `online_services=false` y otro con Ardent falso vía la URL base sustituible de Task 7): `combat_layout_shows_wanted_badge_and_subsystem` (criterio 2), `mining_sell_lists_fake_markets_with_pad_and_age` y `mining_sell_no_data_when_disabled` (criterio 6), `prospector_shows_composition_core_and_verdict` (criterio 5).
- [ ] **Step 2:** revisar los fixtures (sin datos reales de comandante). **Step 3–4:** PASS; checks.
- [ ] **Step 5: Commit** `test: fixtures sintéticos de combate, minería y AX con pruebas de extremo a extremo`

### Task 27: `tmc-headless eval` — vales y rendimiento de asteroides

**Files:** Modify `apps/headless/src/eval.rs`, `crates/tmc-modules/src/eval.rs`.

**Interfaces:** `eval` añade dos informes agregados (sin nombres): (a) conciliación de vales por `RedeemVoucher` (ganado vs. canjeado por tipo, diferencia); (b) toneladas refinadas por asteroide prospectado por nivel de contenido (media, n; cada `MiningRefined` se atribuye al último `ProspectedAsteroid` anterior, heurística indicada en la salida; reutiliza `VoucherLedger::apply_event` y `MiningSession::apply_event`, sin lógica propia) para ajustar `mining_yield.json` (si n ≥ 5 por nivel, se propone el valor; `verified` sigue `false` salvo aprobación).

- [ ] **Step 1: Tests:** `eval_reports_voucher_reconciliation_for_fixture` (diferencia 0 en `combat_session.log`); `eval_reports_tons_per_asteroid_by_content` (High: media == <valor del fixture>, n == <n>).
- [ ] **Step 2–4:** FAIL → implementar → PASS; checks; ejecutar sobre el historial del propietario y anotar solo agregados en «Verificación F2».
- [ ] **Step 5: Commit** `feat(headless): eval de conciliación de vales y rendimiento por asteroide`

### Task 28: avisos, consumo, verificación de los 10 criterios y cierre

**Files:** Modify `THIRD_PARTY_NOTICES.md` (Ardent como servicio de red AGPL, no incorporado; datos propios), este plan («Verificación F2», «Desviaciones»), `status.md`.

- [ ] **Step 1:** Checks completos de Rust y UI (criterio 1) con salida.
- [ ] **Step 2:** `pwsh scripts/measure-footprint.ps1` con «Combate» y «Minería» visibles durante el replay de los fixtures y en reposo (criterio 9).
- [ ] **Step 3:** Evidencia por criterio (comando + salida) en «Verificación F2»; revisión completa de la rama contra la spec.
- [ ] **Step 4: Commit** `docs: verificación de criterios y cierre de F2`

---

## Auto-revisión (cobertura de la spec)

| Spec | Tareas |
|---|---|
| §2.1 objetivo actual | 3, 11, 16, 17 |
| §2.2 mi nave (pips, casco, calor, alertas) | 12, 16, 17 |
| §2.3 sesión de caza, vales, dónde cobrar, IF | 6, 7, 13, 14, 16, 18 |
| §2.4 zona de conflicto · §2.5 AX | 0, 5, 15, 18 |
| §2.6 prospector · §2.7 sesión · §2.8 dónde vender | 4, 9, 19–24 |
| §3.1 disposiciones y relevancia | 16, 22, 25 |
| §4 tipado, Ardent, datos propios | 3–7, 9 |
| §5 servicios y opt-out | 1, 7, 16, 22 |
| §6 rendimiento | 12, 16, 22, 26, 28 |
| §7 i18n | 10, 17, 18, 23, 24, 25 |
| §8 criterios 1–10 | 26, 27, 28 |
| §10 decisiones del redactor | 8, 13, 14, 16, 21, 25 |

## Verificación F2

| # | Criterio (spec §8) | Evidencia (comando + salida) |
|---|---|---|
| 1 | Checks Rust y UI en verde | (pendiente) |
| 2 | Objetivo BUSCADO/LIMPIO, subsistema, escudos/casco; p95 < 500 ms | (pendiente) |
| 3 | Pips exactos, casco por escalones, 3 estados de calor | (pendiente) |
| 4 | Vales sin cobrar = canjeado (±0) y reconstrucción | (pendiente) |
| 5 | Prospector y t/h/refinados del fixture | (pendiente) |
| 6 | «Dónde vender» con servicio falso y «sin datos» desactivado | (pendiente) |
| 7 | AX: bajas/bonos por tipo y presencia | (pendiente) |
| 8 | Cadenas en 5 idiomas | (pendiente) |
| 9 | Footprint < 150 MB y < 1 % CPU | (pendiente) |
| 10 | Avisos y procedencia; nada GPL/AGPL incorporado | (pendiente) |

## Desviaciones

(Anotar aquí cualquier cambio respecto a este plan o a la spec, con fecha, motivo y tareas afectadas. Las omisiones AX del spike y los ajustes de adaptador de Ardent se registran aquí.)

- 2026-10-18 · Spike AX (Task 0): los diarios del propietario no tienen ningún dato AX. Se mantiene `THARGOID_SHIPS` como constante pequeña **no verificada** y tolerante a valores desconocidos (Tasks 11/15); bonos AX y `ThargoidWar` según fuentes secundarias con fixtures sintéticos. Se **omiten** las asistencias (`SharedWithOthers` no aparece): cada `Bounty` cuenta como baja completa. `BrokerPercentage`: se usa `Amount` tal cual (informativo); si un diario real demuestra que es neto, se aplicará `Amount/(1−B/100)`. Afecta a Tasks 5, 11, 13, 14, 15, 26.
- 2026-10-18 · Controlador: `NearestService` incluye `min_pad`; la sesión de minería empieza en `LoadGame` (spec §10.4).
- 2026-10-18 · Preflight: aplicadas las resoluciones de la revisión previa (ver preflight.md del ledger).
- 2026-10-18 · Task 15: se **omite** el bando de la ZC por `Comms` (no se tipan en F2); el bando es el `AwardingFaction` del primer bono. La atribución de tipo AX a bonos es una heurística (objetivo thargoide fijado ≤ 60 s antes, bonos dentro de 5 s del primero = misma baja; si no, «unknown»), no verificada.
