# TMC F3 «Comercio, misiones y pasajeros; recepción de EDDN» — Plan de implementación

> **Para agentes:** SUB-SKILL OBLIGATORIA: usar superpowers:subagent-driven-development (recomendado) o superpowers:executing-plans para ejecutar este plan tarea a tarea. Los pasos usan casillas (`- [ ]`) para el seguimiento. Lo marcado «(verificar en código al ejecutar)» es una interfaz existente que el redactor no confirmó: se comprueba antes de escribir el test y, si difiere, se adapta el nombre sin cambiar el comportamiento y se anota en «Desviaciones».

**Goal:** Que la segunda pantalla diga al comerciante qué comprar aquí y dónde venderlo (con la antigüedad del dato y plataforma compatible), cuánto ha pagado y ganado, y qué ruta seguir; que quien hace misiones no pierda ninguna (agrupadas por destino, avisos, pila de masacre exacta) y que el piloto de pasajeros sepa al aceptar si un turismo compensa. Todo con precios frescos de EDDN guardados en local.

**Architecture:** Se conserva la de F0–F2 (actor único dueño del estado, módulos sin E/S, difusión a 10 Hz, red vía `ModuleCtx::request` → `SourceEvent::Net`, SQLite vía `ModuleCtx::store_task` → `SourceEvent::Stored`, `core.clock` como base de tiempo). Nuevo: crate **`tmc-eddn`** (SUB de ZeroMQ, zlib + JSON, filtro por esquema y distancia, lotes) compuesto por `tmc-app`, que escribe los lotes en la **tabla de precios** de `tmc-store` y avisa al núcleo con `SourceEvent::Eddn`. Dos módulos nuevos: **`trade`** (mercado actual, mi carga, ruta Spansh, sesión, buscar mercancía) y **`missions`** (misiones, masacre, avisos, recompensas, pasajeros, turismo, objetivos comunitarios). Adaptadores nuevos en `tmc-net`: ruta comercial de Spansh (trabajo asíncrono), exportaciones cercanas de Ardent y coordenadas de sistema por nombre. `mining` pasa a usar también la tabla de precios.

**Tech Stack:** el de F2 + **biblioteca ZeroMQ** elegida en el spike (preferencia Rust puro y permisiva; libzmq MPL-2.0 solo si la pura falla; nada GPL/LGPL) + `flate2` (MIT/Apache, ya en `Cargo.lock` como transitiva) como dependencia directa de `tmc-eddn`.

**Spec:** [`docs/specs/2026-10-19-tmc-f3-comercio-misiones-design.md`](../specs/2026-10-19-tmc-f3-comercio-misiones-design.md) (aprobada 2026-10-19; manda sobre este plan). Base: specs de arquitectura/F0, F1 y F2. Dominio: `docs/research/notes/player_roles_domain.md` (comercio, misiones, pasajeros), `community_apis_core.md` (EDDN, Spansh, Ardent).

## Restricciones globales

- Código e identificadores en inglés; documentación, commits y textos de referencia (`es`) en español.
- Presupuesto (requisito): **< 150 MB** RAM del árbol de procesos, **< 1 %** CPU en reposo **con EDDN activo**, difusión **máx. 10 Hz**, latencia evento→UI **p95 < 500 ms**. Sin animaciones infinitas ni `requestAnimationFrame`; las cuentas atrás de misiones/objetivos usan un `setInterval` de 1 s **solo mientras haya algo que contar** y anclado a `core.clock` (como F2).
- **Todo test afirma un valor concreto; no hay tests vacíos.** Los `/* … */` describen entrada y valor esperado; el test real los escribe como datos y `assert_eq!`. Los `<valor>`/`<n>` de tests con fixture se sustituyen por el número concreto al escribir el fixture, en el mismo commit. Los literales de structs en tests dan todos los campos (solo `..Default::default()` si el tipo implementa `Default`).
- **Cada tarea compila y prueba lo suyo:** la primera tarea de un módulo nuevo añade `pub mod <módulo>;` a `crates/tmc-modules/src/lib.rs` y su `mod.rs` esqueleto; cada tarea siguiente declara su `mod x;` en `mod.rs`. **Cada tarea crea el fixture que usan sus tests**; Task 32 solo los completa.
- Toda dependencia nueva se anota en `THIRD_PARTY_NOTICES.md` **en la tarea que la añade** (`flate2` en Task 10, ZeroMQ en Task 11); Task 34 solo verifica.
- Tipos: variantes grandes en `Box` (clippy `large_enum_variant`). Variantes nuevas de `EventKind`, `NetQuery`, `NetOutcome`, `SourceEvent` siguen el patrón existente (`de(name, raw).map(...)`; lo que no deserializa cae en `Other`, nunca error).
- **Módulos** (lecciones F1/F2, obligatorias para `trade` y `missions`):
  - nunca emiten `NetQuery` ni `StoreTask` de **escritura** en replay (test por módulo). **Convención de tags:** toda `StoreTask` que escribe lleva un tag terminado en `.write` (p. ej. `trade.market.write`, `missions.stats.write`); las de lectura, no. Los tests de replay afirman 0 `NetRequest` y 0 tags `*.write`;
  - todo `SourceEvent::Stored` comprueba comandante (y generación) antes de aplicarse (patrón `check_cmdr` de `combat`); al **cambiar de comandante** se vacía el estado del módulo;
  - control de vuelo **solo** con `crate::inflight::Inflight` (`INFLIGHT_TTL_S = 60`, deduplicación, descarte de tardías); `Failed` se reintenta solo tras `NetClient::retry_at`;
  - un topic se marca sucio **solo si cambió**, ignorando `as_of` (las dos copias privadas de `strip_as_of`, en `combat/mod.rs` y `mining/mod.rs`, se mueven a `crate::topicx` en Task 19 sin cambiar comportamiento); cada topic < **16 KiB** con test;
  - `relevance(&self, &GameState)` sin reloj: `last_ts` del último evento; tests avanzan el tiempo con un evento posterior;
  - toda acción nueva (`trade.*`, `missions.*`) necesita su prefijo en `MODULE_PREFIXES` de `crates/tmc-app/src/clipboard.rs` (**lección F2**: sin él la acción nunca llega al núcleo) y un test de integración que lo pruebe.
- Red: nunca con `Galaxy::Legacy` ni con «Consultar servicios online» desactivado; tampoco EDDN (sus precios son de Live). Spansh: 1 trabajo a la vez (F1), caché 15 min por parámetros; Ardent: 1 petición/s, caché 15 min; coordenadas de sistema: caché 30 días (no cambian). Solo se envía sistema/estación actuales, mercancía y parámetros de nave; nada del comandante.
- **EDDN**: solo recepción; descompresión y filtrado fuera del actor; descarte temprano (esquema ≠ `commodity/3` y `journal/1`, sufijo `/test`, lejos, carriers, marcas de tiempo > 5 min en el futuro o > 7 días); tamaño descomprimido máx. **1 MiB**; escritura en SQLite **por lotes** en `spawn_blocking`; retroceso exponencial 1→60 s; ningún mensaje corrupto tumba nada. Datos de EDDN solo a la base local, nunca al repo ni a fixtures (fixtures sintéticos con la forma real).
- **Tabla de precios**: una sola fuente de verdad para precios remotos y locales persistidos; `buy_price` = lo que **pagas** al comprar en la estación, `sell_price` = lo que **recibes** al vender (semántica de `Market.json`/EDDN). Oferta válida para comprar: `buy_price > 0 && stock > 0`; para vender: `sell_price > 0 && demand > 0`. «Mercado actual» y «mejor venta» usan **48 h**; búsqueda de mercancía muestra hasta 7 días con antigüedad. Limpieza de > 7 días al arrancar y cada hora. Fleet carriers excluidos (no se guardan).
- Plataforma: solo con `data::ship_pads` (F2) y `PadSize::fits`; estación sin dato → incluida con `pad_ok: None` y aviso (spec §10.11).
- Licencias: Spansh y Ardent solo por red; ZeroMQ según spike; toda dependencia y fuente nueva en `THIRD_PARTY_NOTICES.md`. **Datos estáticos propios** (`rare_goods.json`, `cabins.json`) con `source` y `verified` por entrada, carga tolerante (`tracing::warn!`, nunca `panic!`), test de carga real y corrupta; nada copiado de fuentes GPL/AGPL ni de SrvSurvey.
- Privacidad: los spikes sobre diarios del propietario emiten **solo agregados** (recuentos, presencia de campos, coincidencias sí/no); nunca nombres de comandante, FID, facciones de sus misiones, chats ni pasajeros.
- i18n: claves en `locales/{es,en,fr,de,pt}/{trade,missions}.ftl` (+ `main.ftl` para ajustes/disposiciones), traducciones reales, mismas claves; nombres de mercancías/facciones/tipos/estaciones desde `_Localised` con respaldo; rango de comercio **en inglés** en los 5 idiomas (como `combat-rank-*`). Toda variable Fluent nueva (`profit`, `tons`, `paid`, `stops`, `kills`, `remaining`, `seats`, `crly`, `percentile`, `age`…) se añade a la lista fija del test `embedded_resources_parse_and_format_all_messages_without_errors` (`crates/tmc-i18n/src/lib.rs`) en el mismo commit.
- UI: números, créditos, distancias, fechas y antigüedades **solo** con `ui/src/lib/format.ts`; lo estimado con «≈»; acciones y `PUT /api/config` con JSON y `Content-Type: application/json`.
- **Checks** antes de cada commit: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; si se toca `ui/`: `npm --prefix ui run check`, `npm --prefix ui test`, `npm --prefix ui run build`.
- Commits pequeños (uno por tarea) en español con prefijo convencional y `Co-Authored-By: <modelo> <noreply@anthropic.com>`. **Sin push.** `status.md` al empezar y terminar cada tarea.
- Una sola fuente por concepto: coste pagado **solo** con `CostBasis::apply_event`; bajas de masacre **solo** con `MassacreStack::apply_event` (`Mission` no guarda bajas: se leen con `MassacreStack::kills_for`); misiones **solo** con `MissionBook::apply_event` (en vivo, al reconstruir y en `eval`); fusión de fuentes de venta **solo** con `mining::sell::merge_sources` (neutral, la usan `trade` y `mining`) seguida de `rank_markets`; ruta de turismo **solo** con `tourism::optimal_order`.

## Foco de revisión

Fallos reales que la spec implica y que los tests «felices» no cubrirían. Cada uno tiene su test en la tarea dueña:

1. **EDDN hostil** (relay caído o lento, zlib corrupto, carga gigante, JSON de esquema raro, ráfagas de cientos de mercados): sin pánico, reconexión con retroceso, CPU y difusión acotadas → `corrupt_zlib_is_dropped_and_counted`, `oversized_payload_is_dropped`, `test_schema_and_unknown_schema_are_dropped` (Task 10), `reconnects_with_backoff_after_relay_drop`, `disabled_filter_disconnects_within_1s` (Task 11), `burst_of_500_eddn_markets_keeps_10hz_and_one_requery` (Task 32).
2. **Hueco en el historial de misiones** (`Missions` lista un `MissionID` sin `MissionAccepted` en la base; o hay `MissionAccepted` de misiones que ya no están en `Missions` porque se cerraron con la app apagada; `Expires` en segundos relativo al evento) → `missions_event_keeps_unknown_mission_minimal` y `accepted_not_in_missions_event_is_dropped` (Task 23).
3. **Pila de masacre mal contada** (baja anterior a aceptar, otra facción objetivo, dos misiones del mismo emisor, redirigida sin cuadrar, reinicio de la app) → `kill_before_accept_not_counted`, `one_kill_counts_once_per_issuer_oldest_first`, `redirected_marks_complete_even_if_count_short` (Task 24), `massacre_rebuilt_after_restart_matches_live` (Task 28).
4. **Precios viejos o inaplicables mostrados como buenos** (venta de hace 49 h, instantánea vieja de EDDN que pisa una nueva, carrier, plataforma desconocida, respuesta tardía de otro sistema, sistema sin coordenadas) → `older_snapshot_does_not_overwrite_newer`, `sell_seen_49h_ago_is_excluded` (Task 6), `carrier_market_dropped`, `unknown_coords_commodity_dropped_and_counted` (Task 10), `merge_sources_dedups_by_market_newest_wins_even_if_cheaper`, `unknown_pad_included_with_warning` (Task 16), `late_exports_for_previous_system_not_applied` (Task 19).
5. **Cambio de comandante, Legacy y replay** (estado de carga/misiones de otro CMDR, EDDN conectado en Legacy, escrituras al reconstruir) → `commander_switch_clears_cost_basis_and_route` (Task 19), `commander_switch_clears_missions` (Task 28), `eddn_disabled_in_legacy_or_offline` (Task 12), `replay_emits_no_requests_or_store_writes` (Tasks 19 y 28).

## Resumen de tareas

| # | Tarea | Bloque |
|---|---|---|
| 0 | Spike EDDN — volumen, CPU, coordenadas y biblioteca ZeroMQ | 0 |
| 1 | Spike red — ruta comercial de Spansh, exportaciones de Ardent, coordenadas por nombre | 0 |
| 2 | Spike misiones — diarios del propietario (agregados) y documentación | 0 |
| 3 | Aprobaciones del propietario tras los spikes | 0 |
| 4 | `tmc-model` — eventos de comercio (`MarketBuy`, `Docked` ampliado, carga de misión) | 1 |
| 5 | `tmc-model` — eventos de misiones, pasajeros y objetivos comunitarios | 1 |
| 6 | `tmc-model::market` + `tmc-store` — tabla de precios, estaciones y coordenadas | 1 |
| 7 | `tmc-model` — consultas de red nuevas y ajustes de F3 | 1 |
| 8 | `tmc-net` — ruta comercial de Spansh y coordenadas por nombre | 1 |
| 9 | `tmc-net` — exportaciones cercanas de Ardent | 1 |
| 10 | `tmc-eddn` — decodificación y filtro (puro) | 1 |
| 11 | `tmc-eddn` — suscriptor, reconexión y lotes | 1 |
| 12 | `tmc-app` — servicio EDDN, `SourceEvent::Eddn`, limpieza y configuración | 1 |
| 13 | Datos estáticos `rare_goods.json` y `cabins.json` | 1 |
| 14 | i18n — `trade.ftl` y `missions.ftl` | 1 |
| 15 | `trade` — coste pagado (`CostBasis`) y «Mi carga» | 2 |
| 16 | `trade` — «Mercado actual» y mejor venta en 48 h | 2 |
| 17 | `trade` — ruta comercial con avance automático | 2 |
| 18 | `trade` — sesión de comercio y buscar mercancía | 2 |
| 19 | `trade` — módulo, topics, relevancia, red, almacén y registro | 2 |
| 20 | `mining` — «Dónde vender» con la tabla de precios | 2 |
| 21 | UI — «Mercado actual» y «Mi carga» | 2 |
| 22 | UI — «Ruta comercial», «Sesión» y «Buscar mercancía» | 2 |
| 23 | `missions` — libro de misiones y reconstrucción | 3 |
| 24 | `missions` — pila de masacre | 3 |
| 25 | `missions` — avisos, recompensas de la sesión y objetivos comunitarios | 3 |
| 26 | `missions` — pasajeros y cabinas | 3 |
| 27 | `missions` — turismo: ruta óptima, cr/al y veredicto | 3 |
| 28 | `missions` — módulo, coordenadas, topics, relevancia y registro | 3 |
| 29 | UI — «Misiones activas», «Masacre» y «Avisos» | 3 |
| 30 | UI — «Pasajeros», «Turismo», «Recompensas» y «Objetivos comunitarios» | 3 |
| 31 | UI — disposiciones «Comercio» y «Misiones», prioridad y ajustes | 4 |
| 32 | Fixtures sintéticos y pruebas de extremo a extremo | 4 |
| 33 | `tmc-headless eval` — conciliación de coste pagado y de masacre | 4 |
| 34 | Avisos de terceros, consumo con EDDN, verificación de los 11 criterios y cierre | 4 |

## Mapa de archivos

```
crates/tmc-model/src/{events.rs*, net.rs*, settings.rs*, source.rs*, market.rs+, lib.rs*}
crates/tmc-store/src/{prices.rs+, migrations.rs*, lib.rs*}
crates/tmc-eddn/{Cargo.toml+, src/lib.rs+, src/decode.rs+, src/filter.rs+, src/coords.rs+, src/subscriber.rs+, src/transport.rs+, examples/fake_relay.rs+}
crates/tmc-net/src/{spansh.rs*, ardent.rs*, edsm.rs*, client.rs*, cache_policy.rs*}
crates/tmc-modules/data/{rare_goods.json+, cabins.json+}
crates/tmc-modules/src/{lib.rs*, registry.rs*, data.rs*, eval.rs*, topicx.rs+}
crates/tmc-modules/src/trade/{mod.rs+, cost.rs+, market.rs+, route.rs+, session.rs+, search.rs+, tests.rs+}
crates/tmc-modules/src/missions/{mod.rs+, book.rs+, massacre.rs+, alerts.rs+, rewards.rs+, passengers.rs+, tourism.rs+, coords.rs+, tests.rs+}
crates/tmc-modules/src/mining/{mod.rs*, sell.rs*}   crates/tmc-modules/src/combat/mod.rs*   (strip_as_of → topicx)
crates/tmc-core/src/actor.rs*                    (match exhaustivo de SourceEvent::Eddn)
crates/tmc-app/src/{eddn_service.rs+, wiring.rs*, config.rs*, clipboard.rs*, net_service.rs*, lib.rs*}
crates/tmc-i18n/src/lib.rs*   apps/headless/src/{eval.rs*, main.rs*}
locales/{es,en,fr,de,pt}/{main.ftl*, trade.ftl+, missions.ftl+}
ui/src/modules/registry.ts*   ui/src/shell/layouts.ts*   ui/src/settings/Settings.svelte*
ui/src/modules/trade/{index.ts+, types.ts+, MarketPanel.svelte+, CargoPanel.svelte+, RoutePanel.svelte+, TradeSessionPanel.svelte+, SearchPanel.svelte+}
ui/src/modules/missions/{index.ts+, types.ts+, MissionsPanel.svelte+, MassacrePanel.svelte+, MissionAlerts.svelte+, PassengersPanel.svelte+, TourismPanel.svelte+, RewardsPanel.svelte+, CommunityGoalsPanel.svelte+}
ui/tests/{trade.test.ts+, missions.test.ts+, layouts.test.ts*, e2e/*}
tests/fixtures/journal/{trade_session.log+, missions_session.log+, massacre_session.log+, tourism_session.log+}
tests/fixtures/companion/market_trade.json+
tests/fixtures/net/{spansh_trade_job.json+, spansh_trade_result.json+, ardent_nearby_exports_gold.json+, coords_by_name.json+}
tests/fixtures/eddn/{commodity_ok.json+, journal_docked.json+, burst.jsonl+}
docs/research/notes/{spike_eddn_f3.md+, spike_red_f3.md+, spike_misiones_f3.md+}
THIRD_PARTY_NOTICES.md*
```
(`+` crear, `*` modificar.)

---

## Bloque 0 — Spikes y aprobaciones

Sin código de producción. Programas desechables en el scratchpad (nunca en el repo); peticiones reales ≤ 1/s. Las interfaces internas de Bloques 1–4 **no cambian** por los spikes: solo los adaptadores (`tmc-eddn::transport`, `tmc_net::{spansh,ardent,edsm}`, constantes de clasificación de misiones).

### Task 0: Spike EDDN

**Files:** Create `docs/research/notes/spike_eddn_f3.md`, `tests/fixtures/eddn/{commodity_ok.json, journal_docked.json}` (sintéticos con la forma real).

**Interfaces:** Produces `EddnMode` (`Always` | `OnDemand`) y la biblioteca para `tmc-eddn::transport` (Task 11); reglas de carriers, coordenadas y plataformas para Task 10.

- [ ] **Step 1:** Biblioteca: probar `zeromq` (zmq.rs, Rust puro) y, solo si falla con `tcp://eddn.edcd.io:9500`, `zmq` (libzmq MPL-2.0, compilación MSVC). Anotar licencia, versión, tamaño del binario y si soporta PUB local (para el relay falso).
- [ ] **Step 2:** 10 min de flujo real: mensajes/s por esquema, tamaño comprimido/descomprimido (p50/p99/máx.), CPU y RAM del proceso con descompresión + filtro + descarte, % de `commodity/3` dentro de 200 al de la última posición del propietario (solo el %), % de mensajes `commodity/3` cuyo sistema tenía coordenadas por `journal/1` (con y sin arranque en frío), presencia de `LandingPads`/`StationType` en `journal/1` `Docked`, regla para reconocer carriers (`StationType == "FleetCarrier"` y/o `marketId ≥ 3_700_000_000`).
- [ ] **Step 3:** Documentar tabla de mediciones, forma de `commodity/3` y `journal/1` recortadas, decisión de biblioteca, regla de carriers, fecha.
- [ ] **Criterio de decisión:** si CPU en reposo con filtro > 0,3 % (margen sobre el 1 % total) → `EddnMode::OnDemand` (spec §10.13); si no, `Always`. Si `commodity/3` sin coordenadas conocidas supera el 50 % en frío, se activa la siembra de coordenadas desde el historial propio y `system_coords` (Task 12), sin cambiar interfaces.
- [ ] **Step 4: Commit** `docs: spike de EDDN para F3 (volumen, CPU, coordenadas y ZeroMQ)`

### Task 1: Spike red — Spansh trade, Ardent exports, coordenadas

**Files:** Create `docs/research/notes/spike_red_f3.md`, `tests/fixtures/net/{spansh_trade_job.json, spansh_trade_result.json, ardent_nearby_exports_gold.json, coords_by_name.json}`.

**Interfaces:** Produces el mapeo a `TradeRouteParams`/`TradeRoutePlan`, `RemoteExport` y `SystemCoords` (Task 7) y las URL/formularios de Task 8–9.

- [ ] **Step 1:** Spansh: `POST /api/trade/route` (parámetros reales: `system`, `station`, `starting_capital`, `max_cargo`, `max_hops`, `max_hop_distance`, `max_system_distance`, `max_price_age`, `requires_large_pad`, `allow_planetary`, `allow_player_owned`, `unique`, `permit`…), trabajo y sondeo `/api/results/{job}` (¿mismo `parse_job` y sondeo que F1?), forma de cada salto (origen, destino, mercancías con `buy_price`/`sell_price`/`amount`/`profit`, `distance`, antigüedad del mercado).
- [ ] **Step 2:** Ardent `nearby/exports` (parámetros como `nearby/imports` de F2; campos `buyPrice`, `stock`, `maxLandingPadSize`, `updatedAt`). Coordenadas por nombre: Spansh (búsqueda de sistemas) frente a EDSM `api-v1/system?systemName=&showCoordinates=1`; elegir una con la otra de respaldo.
- [ ] **Step 3:** Documentar y escribir los fixtures sintéticos con la forma real (incluir un salto con dos mercancías y un mercado de hace 3 días).
- [ ] **Criterio de decisión:** si Spansh no acepta tamaño de plataforma M (solo `requires_large_pad`), el adaptador envía `requires_large_pad = (pad == Large)` y filtra en local las paradas sin plataforma compatible; la interfaz no cambia.
- [ ] **Step 4: Commit** `docs: spike de red para F3 (ruta comercial, exportaciones y coordenadas)`

### Task 2: Spike misiones

**Files:** Create `docs/research/notes/spike_misiones_f3.md`.

**Interfaces:** Produces las constantes de `missions::book::classify_kind`, el separador real de destinos múltiples, la regla de turismo con regreso y si `FactionKillBond` cuenta para masacre (Tasks 5, 23, 24, 27).

- [ ] **Step 1:** Script desechable sobre `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous` (**solo lectura, solo agregados**): recuentos de `Missions`, `MissionAccepted/Completed/Failed/Abandoned/Redirected`, `CargoDepot`, `Passengers`, `CommunityGoal`, `MarketBuy`; prefijos de `Name` (`Mission_Massacre*`, `Mission_Sightseeing*`, `Mission_Passenger*`…) con recuento; presencia de `$MISSIONUTIL_MULTIPLE_` en `DestinationSystem`; campos presentes en cada evento; semántica de `Missions.Active[].Expires`; si `Bounty.VictimFaction` coincide literalmente con `TargetFaction` (solo «coincide n/m»); valores de `CargoDepot.UpdateType`; campos del manifiesto de `Passengers` (¿clase de cabina?); `MissionID` en inventario de `Cargo`; `LandingPads`/`StationServices` en `Docked`.
- [ ] **Step 2:** Completar con documentación del diario (Frontier Journal Manual) lo no visto; marcar «no visto / fuente secundaria».
- [ ] **Criterio de decisión:** sin `$MISSIONUTIL_MULTIPLE_` real ni documentado → turismo sin ruta («paradas no disponibles en el diario») y desviación (spec §10.7). Sin clase de cabina en el manifiesto → ocupación agregada (Task 26). `FactionKillBond` cuenta solo si el spike lo confirma (`MASSACRE_COUNTS_KILL_BONDS`, por defecto `false`).
- [ ] **Step 3: Commit** `docs: spike de misiones para F3 (agregados de diarios y documentación)`

### Task 3: Aprobaciones del propietario tras los spikes

**Files:** Modify `status.md` («Bloqueos»).

- [ ] **Step 1:** Pedir aprobación solo de lo que cambie alcance: `EddnMode`, biblioteca ZeroMQ y su licencia, omisiones del spike de misiones, ambigüedades resueltas por el redactor (ver «Desviaciones»).
- [ ] **Step 2:** Tasks 4–31 avanzan mientras tanto (interfaces independientes). **Step 3: Commit** `docs(status): aprobaciones pendientes tras los spikes de F3`

---

## Bloque 1 — Eventos, precios, red, EDDN, datos e i18n

### Task 4: `tmc-model` — eventos de comercio

**Files:** Modify `crates/tmc-model/src/events.rs`.

**Interfaces:**
- Consumes: `Docked`, `CargoItem`, `MarketSell`, `commodity_key` existentes.
- Produces: `MarketBuy { market_id: Option<u64> (rename "MarketID"), type_: String (rename "Type"), type_localised: Option<String>, count: u32, buy_price: u64, total_cost: u64 }` y `EventKind::MarketBuy(MarketBuy)`; `Docked` + `station_services: Vec<String>`, `landing_pads: Option<LandingPads { small: u32, medium: u32, large: u32 }>`, `dist_from_star_ls: Option<f64>`; `impl Docked { fn has_market(&self) -> bool /* "commodities" en servicios */; fn max_pad(&self) -> Option<PadSize> /* Large si large>0 … */ }`; `CargoItem` + `mission_id: Option<u64> (rename "MissionID")`; `impl CargoItem { fn is_mission(&self) -> bool; fn stolen_count(&self) -> u32 }`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn market_buy_parses() { /* MarketID 128000000, Type "gold", Count 100, BuyPrice 9400, TotalCost 940000 → kind MarketBuy, total_cost 940000 */ }
#[test] fn docked_with_pads_and_services() { /* LandingPads{Small:4,Medium:8,Large:2}, StationServices ["dock","commodities"] → max_pad Some(Large), has_market true */ }
#[test] fn docked_outpost_without_large_pads() { /* LandingPads{Small:2,Medium:1,Large:0} → Some(Medium) */ }
#[test] fn cargo_item_with_mission_id_is_mission() { /* {Name:"gold",Count:10,MissionID:65000001} → is_mission true; {Stolen:3} → stolen_count 3 */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar y clasificar; brazos nuevos en `match` exhaustivos sin cambiar comportamiento. **Step 4:** PASS; Checks. Si los campos nuevos de `Docked`/`CargoItem` alteran snapshots `insta` existentes, se revisa el diff (solo campos nuevos con valor vacío/`null`) y se actualizan en el mismo commit.
- [ ] **Step 5: Commit** `feat(model): eventos de comercio, plataformas del atraque y carga de misión`

### Task 5: `tmc-model` — eventos de misiones, pasajeros y objetivos comunitarios

**Files:** Modify `crates/tmc-model/src/events.rs`.

**Interfaces:**
- Produces (campos `Option` salvo `mission_id`; `_Localised` con `rename`):
  - `MissionsList { active: Vec<MissionRef>, failed: Vec<MissionRef>, complete: Vec<MissionRef> }`, `MissionRef { mission_id: u64 (rename "MissionID"), name: String, passenger_mission: bool, expires: Option<i64> /* s restantes */ }`.
  - `MissionAccepted { mission_id, name, localised_name, faction, commodity, commodity_localised, count: Option<u32>, target_faction, kill_count: Option<u32>, target_type, target_type_localised, destination_system, destination_station, destination_settlement, expiry: Option<DateTime<Utc>>, wing: Option<bool>, influence, reputation, reward: Option<u64>, donation: Option<u64>, passenger_count: Option<u32>, passenger_vips: Option<bool> (rename "PassengerVIPs"), passenger_wanted: Option<bool>, passenger_type }`.
  - `MissionCompleted { mission_id, name, faction, reward: Option<u64>, donation: Option<u64>, materials_reward: Vec<MaterialReward { name, name_localised, count: u32 }>, faction_effects: Vec<FactionEffect { faction: String, influence: Vec<InfluenceEffect { system_address: Option<u64>, influence: String }>, reputation: Option<String> }> }`.
  - `MissionFailed { mission_id, name, fine: Option<u64> }`, `MissionAbandoned` (misma forma), `MissionRedirected { mission_id, name, new_destination_system, new_destination_station, old_destination_system, old_destination_station }`.
  - `CargoDepot { mission_id, update_type: String, cargo_type, count: Option<u32>, items_collected: u32, items_delivered: u32, total_items_to_deliver: u32, progress: Option<f64> }`.
  - `Passengers { manifest: Vec<PassengerEntry { mission_id, type_: String (rename "Type"), vip: bool (rename "VIP"), wanted: bool, count: u32 }> }`.
  - `CommunityGoal { current_goals: Vec<CgEntry { cgid: u64 (rename "CGID"), title: String, system_name, market_name, expiry: Option<DateTime<Utc>>, is_complete: bool, current_total: Option<u64>, player_contribution: Option<u64>, num_contributors: Option<u64>, player_percentile_band: Option<u32>, tier_reached: Option<String>, bonus: Option<u64> }> }`.
  - `EventKind` + `Missions(Box<MissionsList>)`, `MissionAccepted(Box<MissionAccepted>)`, `MissionCompleted(Box<MissionCompleted>)`, `MissionFailed(..)`, `MissionAbandoned(..)`, `MissionRedirected(..)`, `CargoDepot(..)`, `Passengers(..)`, `CommunityGoal(Box<CommunityGoal>)`.
  - `pub fn split_multi_destination(raw: &str) -> Vec<String>` (separadores del spike; sin marcadores → `vec![raw]`; vacío → `vec![]`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn missions_event_parses_active_with_expires() { /* Active [{MissionID:65000001,Name:"Mission_Delivery_name",PassengerMission:false,Expires:5400}] → active[0].expires Some(5400) */ }
#[test] fn mission_accepted_massacre_parses() { /* TargetFaction "Alpha Corp", KillCount 24, Reward 4000000, Expiry "2026-10-20T12:00:00Z" → kill_count Some(24), reward Some(4000000) */ }
#[test] fn mission_completed_with_materials_and_effects() { /* MaterialsReward [{Name:"tellurium",Count:2}], FactionEffects [{Faction:"A",Influence:[{Influence:"+++"}],Reputation:"++"}] → materials_reward[0].count 2, influence[0].influence "+++" */ }
#[test] fn redirected_and_cargo_depot_parse() { /* NewDestinationSystem "Sol"; CargoDepot UpdateType "Deliver", ItemsDelivered 30, TotalItemsToDeliver 60 */ }
#[test] fn passengers_manifest_parses_vip_wanted() { /* [{MissionID:1,Type:"Tourist",VIP:true,Wanted:false,Count:4}] → vip true, count 4 */ }
#[test] fn community_goal_parses() { /* CGID 750, PlayerContribution 1200, PlayerPercentileBand 25, Bonus 15000000 */ }
#[test] fn split_multi_destination_three_stops() { /* "Sol$MISSIONUTIL_MULTIPLE_INNER_SEPARATOR;Lave$MISSIONUTIL_MULTIPLE_FINAL_SEPARATOR;Leesti" → ["Sol","Lave","Leesti"]; "Sol" → ["Sol"]; "" → [] */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(model): eventos de misiones, pasajeros y objetivos comunitarios`

### Task 6: `tmc-model::market` y `tmc-store` — tabla de precios, estaciones y coordenadas

**Files:** Create `crates/tmc-model/src/market.rs`, `crates/tmc-store/src/prices.rs`; Modify `crates/tmc-model/src/lib.rs`, `crates/tmc-store/src/{lib.rs, migrations.rs}`.

**Interfaces:**
- Produces (`tmc-model::market`; todos los tipos `Debug + Clone + PartialEq + Serialize + Deserialize`, porque viajan como JSON en `SourceEvent::Stored`): `enum PriceOrigin { Local, Eddn, Ardent }`; `CommodityPrice { commodity: String /* commodity_key */, name_localised: Option<String>, buy_price: u64, sell_price: u64, stock: u64, demand: u64 }`; `MarketSnapshot { market_id: u64, station_name: String, system_name: String, system_address: Option<u64>, star_pos: Option<[f64; 3]>, max_pad: Option<PadSize>, station_type: Option<String>, updated_at: DateTime<Utc>, origin: PriceOrigin, prices: Vec<CommodityPrice> }`; `StationInfo { market_id, station_name, system_name, star_pos: Option<[f64;3]>, max_pad: Option<PadSize>, station_type: Option<String>, dist_ls: Option<f64> }`; `PricedMarket { market_id, station_name, system_name, distance_ly: Option<f64>, max_pad: Option<PadSize>, price: u64, volume: u64 /* stock o demanda */, updated_at: DateTime<Utc>, origin: PriceOrigin }`; `pub enum Side { Buy, Sell }`; `pub fn is_fleet_carrier(market_id: u64, station_type: Option<&str>) -> bool` (regla de Task 0).
- Produces (`tmc-store`, migración núcleo `f3_prices`): tablas `market_price(market_id, commodity, name_localised, buy_price, sell_price, stock, demand, updated_at, origin, PRIMARY KEY(market_id, commodity))` + índice `(commodity, updated_at)`, `market_station(market_id PK, station_name, system_name, system_address, x, y, z, max_pad, station_type, dist_ls, updated_at)`, `system_coords(name_key PK, system_address, x, y, z, seen_at)`. Métodos `Store`:
  - `fn replace_markets(&self, snaps: &[MarketSnapshot]) -> Result<usize, StoreError>` (una transacción; solo si `updated_at` > el guardado; borra filas viejas de ese mercado);
  - `fn upsert_partial_prices(&self, rows: &[(StationInfo, CommodityPrice, DateTime<Utc>, PriceOrigin)]) -> Result<usize, StoreError>` (Ardent; la más nueva gana);
  - `fn upsert_stations(&self, st: &[StationInfo]) -> Result<usize, StoreError>`, `fn upsert_coords(&self, c: &[(String, Option<u64>, [f64; 3])], at) -> Result<usize, StoreError>`, `fn coords_by_names(&self, names: &[String]) -> Result<HashMap<String, [f64; 3]>, StoreError>`, `fn coords_within(&self, center: [f64;3], radius_ly: f64, limit: usize) -> Result<Vec<(String, [f64;3])>, StoreError>`;
  - `fn best_prices(&self, commodity: &str, side: Side, center: [f64; 3], radius_ly: f64, since: DateTime<Utc>, exclude_market: Option<u64>, limit: usize) -> Result<Vec<PricedMarket>, StoreError>` (caja envolvente en SQL + distancia en Rust; filas sin coordenadas excluidas; ordena por precio: desc si `Sell`, asc si `Buy`);
  - `fn market_prices(&self, market_id: u64) -> Result<Vec<CommodityPrice>, StoreError>`, `fn known_commodities(&self, limit: usize) -> Result<Vec<(String, Option<String>)>, StoreError>` (clave + último nombre localizado no nulo);
  - `fn purge_prices_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize, StoreError>` (precios y estaciones > 7 días; coordenadas > 30 días).

- [ ] **Step 1: Tests** (en `prices.rs`, `Store::open_in_memory`):

```rust
#[test] fn replace_then_best_sell_within_radius() { /* 2 mercados a 10 al y 300 al vendiendo gold a 10500 y 11000; radio 100 → 1 resultado, price 10500, distance_ly ≈ 10.0 */ }
#[test] fn older_snapshot_does_not_overwrite_newer() { /* snapshot t=12:00 gold 10500; luego t=11:00 gold 9000 → market_prices(gold).sell_price 10500 */ }
#[test] fn replace_removes_commodities_missing_from_new_snapshot() { /* t1 {gold,silver}; t2 {gold} → market_prices len 1 */ }
#[test] fn sell_seen_49h_ago_is_excluded() { /* venta de hace 49 h a 12000 y otra de hace 2 h a 11000, since = now−48h → 1 resultado, price 11000 */ }
#[test] fn known_commodities_keep_localised_name() { /* gold con Name_Localised "Oro" → [("gold", Some("Oro"))] */ }
#[test] fn sell_requires_demand_and_buy_requires_stock() { /* demand 0 → no aparece en Sell; stock 0 → no en Buy */ }
#[test] fn purge_deletes_rows_older_than_7_days() { /* filas de hace 8 d y 1 d → purge devuelve 1 y queda 1 */ }
#[test] fn coords_within_and_by_names() { /* 3 sistemas, 2 dentro de 50 al → coords_within len 2; coords_by_names(["sol"]) devuelve [0,0,0] (name_key en minúsculas) */ }
#[test] fn is_fleet_carrier_rule() { /* (3_700_000_123, None) → true; (128_000_000, Some("Coriolis")) → false; (1, Some("FleetCarrier")) → true */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(store): tabla de precios de mercado, estaciones y coordenadas con limpieza de 7 días`

### Task 7: `tmc-model` — consultas de red nuevas y ajustes de F3

**Files:** Modify `crates/tmc-model/src/{net.rs, settings.rs}`, `crates/tmc-net/src/{client.rs, ardent.rs}`, `crates/tmc-app/src/config.rs` (`Config::module_settings()`, literal sin `..Default`), `crates/tmc-app/src/lib.rs` (literal de `ModuleSettings` en tests, ~l. 588), `crates/tmc-app/src/net_service.rs` (`match` exhaustivos de `NetQuery`/`NetOutcome` y `CacheKind`; verificar en código al ejecutar).

**Interfaces:**
- Produces:
  - `RemoteMarket` + `origin: Option<PriceOrigin>` (`ardent::parse_markets` lo rellena con `Some(Ardent)`; los demás constructores, `None`).
  - `TradeRouteParams { system: String, station: String, starting_capital: u64, max_cargo: u32, max_hops: u8, max_hop_distance_ly: f64, min_pad: PadSize, max_price_age_days: u32, allow_planetary: bool }` (carriers siempre excluidos); `TradeCommodity { commodity: String, name_localised: Option<String>, amount: u32, buy_price: u64, sell_price: u64, profit_per_ton: i64 }`; `TradeStop { from_system, from_station, to_system, to_station: String, from_market_id: Option<u64>, to_market_id: Option<u64>, commodities: Vec<TradeCommodity>, stop_profit: i64, distance_ly: f64, data_updated_at: Option<DateTime<Utc>> }`; `TradeRoutePlan { stops: Vec<TradeStop>, total_profit: i64 }`.
  - `RemoteExport { station_name, system_name, market_id: Option<u64>, distance_ly: f64, distance_to_arrival_ls: Option<f64>, buy_price: u64, stock: Option<u64>, max_pad: Option<PadSize>, station_type: Option<String>, updated_at: Option<DateTime<Utc>> }`; `SystemCoords { name: String, id64: Option<u64>, star_pos: [f64; 3] }`.
  - `NetQuery` + `TradeRoute(TradeRouteParams)`, `CommodityExportsNearby { system_address: u64, commodity: String, max_distance_ly: u32, min_pad: PadSize }`, `SystemCoordsByName { name: String }`; `NetOutcome` + `TradeRoute(TradeRoutePlan)`, `Exports(Vec<RemoteExport>)`, `Coords(SystemCoords)` (no encontrado → `NoData`).
  - `ModuleSettings` + `eddn_radius_ly: u32` (200), `tourism_reference_cr_ly: u64` (100 000); valores de F1/F2 sin cambios. `Config` gana los dos campos con `#[serde(default)]` y `module_settings()` los copia (la validación del `PUT` es de Task 12).

- [ ] **Step 1: Tests:**

```rust
#[test] fn trade_route_query_roundtrips() { /* TradeRoute{system:"Lave",station:"Lave Station",starting_capital:5_000_000,max_cargo:256,max_hops:5,max_hop_distance_ly:30.0,min_pad:Large,max_price_age_days:2,allow_planetary:true} → roundtrip igual */ }
#[test] fn ardent_markets_carry_ardent_origin() { /* fixture de F2 ardent_nearby_imports_platinum.json → todos origin Some(Ardent) */ }
#[test] fn trade_route_outcome_roundtrips() { /* 2 paradas, total_profit 3_200_000 → roundtrip */ }
#[test] fn coords_outcome_roundtrips() { /* Coords{name:"Sol",id64:Some(10477373803),star_pos:[0.0,0.0,0.0]} */ }
#[test] fn module_settings_f3_defaults() { /* eddn_radius_ly 200, tourism_reference_cr_ly 100_000, sell_search_radius_ly 100 */ }
```

- [ ] **Step 2:** FAIL. **Step 3:** implementar; brazos nuevos en `match` exhaustivos de `tmc-net` y `net_service.rs` → `Disabled` provisional. **Step 4:** PASS; `cargo test --workspace` verde (incluido `tmc-app`); Checks.
- [ ] **Step 5: Commit** `feat(model): consultas de ruta comercial, exportaciones y coordenadas; ajustes de F3`

### Task 8: `tmc-net` — ruta comercial de Spansh y coordenadas por nombre

**Files:** Modify `crates/tmc-net/src/{spansh.rs, edsm.rs, client.rs, cache_policy.rs}`, `crates/tmc-app/src/net_service.rs` (caché y `CacheKind` de las consultas nuevas).

**Interfaces:**
- Consumes: sondeo de trabajos de F1 (`parse_job`, `results_url`, bucle de sondeo de `NeutronRoute` — verificar en código al ejecutar), fixtures de Task 1.
- Produces: `spansh::trade_url() -> String`, `spansh::trade_form(p: &TradeRouteParams) -> Vec<(String, String)>`, `spansh::parse_trade_result(body: &str) -> Result<TradePoll, NetFailure>` (`TradePoll::{Pending, Done(TradeRoutePlan)}`; paradas en carriers descartadas; `profit_per_ton = sell − buy`); `parse_coords(body) -> Result<Option<SystemCoords>, NetFailure>` en el servicio elegido en Task 1 (el otro de respaldo); `cache_policy`: `spansh:trade:<hash de parámetros>` 15 min, `coords:<name_key>` 30 días; `NetClient::execute` con brazos nuevos, `retry_at`/`is_down_for` por servicio; un único trabajo de Spansh a la vez (compartido con F1).

- [ ] **Step 1: Tests:**

```rust
#[test] fn trade_form_has_real_params() { /* contiene ("max_cargo","256"), ("starting_capital","5000000"), ("requires_large_pad","1") para Large */ }
#[test] fn parses_trade_result_fixture() { /* spansh_trade_result.json → stops.len()==<n>, stops[0].commodities[0].profit_per_ton == <valor>, total_profit == <valor> */ }
#[test] fn trade_result_pending_is_pending() { /* {"status":"queued"} → Pending */ }
#[test] fn parses_coords_fixture() { /* coords_by_name.json → star_pos == <valor> */ }
#[tokio::test] async fn trade_route_disabled_offline_never_reaches_transport() { /* online=false → Disabled, total()==0 */ }
#[tokio::test] async fn coords_cached_for_30_days() { /* 2.ª consulta igual → NetSource::Cache, total()==1 */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(net): ruta comercial de Spansh y coordenadas de sistema por nombre`

### Task 9: `tmc-net` — exportaciones cercanas de Ardent

**Files:** Modify `crates/tmc-net/src/{ardent.rs, client.rs, cache_policy.rs}`, `crates/tmc-app/src/net_service.rs`.

**Interfaces:**
- Consumes: `ardent::parse_markets` y límites de F2.
- Produces: `ardent::exports_nearby_url(system_address: u64, commodity: &str, max_distance_ly: u32) -> String`; `ardent::parse_exports(body: &str) -> Result<Vec<RemoteExport>, NetFailure>` (sin obras ni carriers; `stock > 0`; orden por `buy_price` asc; máx. 50); caché `ardent:exports:<addr>:<commodity>:<radio>` 15 min.

- [ ] **Step 1: Tests:**

```rust
#[test] fn exports_url_has_filters() { /* addr 10477373803, "gold", 100 → contiene "/commodity/name/gold/nearby/exports", "maxDistance=100", "fleetCarriers=false" */ }
#[test] fn parses_exports_fixture_sorted_by_price() { /* ardent_nearby_exports_gold.json → [0].buy_price == <valor mínimo>, ninguno con stock 0 */ }
#[tokio::test] async fn exports_share_ardent_one_per_second_limit() { /* imports + exports seguidos → el 2.º sale ≥ 1 s después */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(net): exportaciones cercanas de Ardent`

### Task 10: `tmc-eddn` — decodificación y filtro

**Files:** Create `crates/tmc-eddn/{Cargo.toml, src/lib.rs, src/decode.rs, src/filter.rs, src/coords.rs}`; Modify `Cargo.toml` (workspace), `THIRD_PARTY_NOTICES.md` (`flate2`).

**Interfaces:**
- Consumes: `tmc-model::{market::*, events::commodity_key, PadSize}` (única dependencia interna, spec §4).
- Produces:
  - `pub const COMMODITY_SCHEMA: &str = "https://eddn.edcd.io/schemas/commodity/3"`, `JOURNAL_SCHEMA` (`journal/1`); `pub const MAX_INFLATED_BYTES: usize = 1 << 20`.
  - `pub enum Decoded { Market(MarketSnapshot), Station(StationInfo, Option<(String, Option<u64>, [f64;3])>), Coords(String /* name_key */, Option<u64>, [f64;3]), Ignored(DropReason) }` (`FSDJump`/`Location`/`CarrierJump` → `Coords`; `Docked` → `Station`); `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)] pub enum DropReason { Corrupt, TooLarge, Schema, TestSchema, Stale, Future, Carrier, Far, UnknownCoords }`.
  - `pub fn decode(frame: &[u8], now: DateTime<Utc>) -> Decoded` (zlib con lector acotado `take(MAX_INFLATED_BYTES + 1)` → `TooLarge` si se supera; `$schemaRef` primero con struct ligero + `RawValue`; `journal/1` solo `Docked`/`FSDJump`/`Location`/`CarrierJump` para coordenadas y plataformas; `commodity/3` → instantánea `origin: Eddn`, claves con `commodity_key`).
  - `pub struct CoordIndex` (`fn insert(&mut self, name_key: String, id64: Option<u64>, pos: [f64;3], center: Option<[f64;3]>)`, `fn get(&self, name_key: &str) -> Option<[f64;3]>`, `fn len(&self) -> usize`, `fn recenter(&mut self, center: [f64;3], keep_radius_ly: f64)`; capacidad máx. `COORD_INDEX_CAP = 30_000`: al llenarse descarta la entrada más lejana a `center`, o la más antigua si `center` es `None`).
  - `pub struct EddnFilter { pub enabled: bool, pub center: Option<[f64; 3]>, pub radius_ly: f64 }`; `pub fn accept(snap: &mut MarketSnapshot, idx: &CoordIndex, f: &EddnFilter) -> Result<(), DropReason>` (rellena `star_pos`; sin coordenadas → `UnknownCoords`; fuera de radio → `Far`).
  - `pub struct EddnStats { pub received: u64, pub accepted: u64, pub dropped: BTreeMap<DropReason, u64> }`.

- [ ] **Step 1: Tests** (fixtures `tests/fixtures/eddn/*` comprimidos en el test):

```rust
#[test] fn decodes_commodity_fixture() { /* commodity_ok.json → Market, prices.len()==<n>, prices[0].commodity "gold", origin Eddn */ }
#[test] fn corrupt_zlib_is_dropped_and_counted() { /* b"\x78\x9cbasura" → Ignored(Corrupt) */ }
#[test] fn oversized_payload_is_dropped() { /* 2 MiB de espacios comprimidos → Ignored(TooLarge) */ }
#[test] fn journal_fsdjump_yields_coords_only() { /* journal/1 FSDJump StarSystem "Lave", StarPos [75.75,48.75,70.75] → Decoded::Coords("lave", _, [75.75,48.75,70.75]) */ }
#[test] fn test_schema_and_unknown_schema_are_dropped() { /* ".../commodity/3/test" → TestSchema; ".../outfitting/2" → Schema */ }
#[test] fn stale_and_future_timestamps_dropped() { /* timestamp now-8d → Stale; now+10 min → Future */ }
#[test] fn carrier_market_dropped() { /* marketId 3_700_000_123 → Ignored(Carrier) */ }
#[test] fn journal_docked_feeds_coords_and_pads() { /* journal_docked.json → Station con max_pad Some(Large) y StarPos <valor> */ }
#[test] fn unknown_coords_commodity_dropped_and_counted() { /* índice vacío → accept == Err(UnknownCoords) */ }
#[test] fn far_market_dropped_near_market_accepted() { /* centro [0,0,0], radio 200; sistema a 150 al → Ok y star_pos rellenado; a 250 al → Err(Far) */ }
#[test] fn coord_index_is_bounded() { /* 30_001 inserciones con center [0,0,0], la última a 1 al → len()==30_000 y get(última) Some */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(eddn): crate tmc-eddn con decodificación, filtro por esquema y distancia`

### Task 11: `tmc-eddn` — suscriptor, reconexión y lotes

**Files:** Create `crates/tmc-eddn/src/{subscriber.rs, transport.rs}`, `crates/tmc-eddn/examples/fake_relay.rs`, `tests/fixtures/eddn/burst.jsonl` (500 `commodity/3` cercanos + 20 corruptos, sintéticos); Modify `crates/tmc-eddn/src/lib.rs`, `Cargo.toml`, `THIRD_PARTY_NOTICES.md` (biblioteca ZeroMQ y su licencia).

**Interfaces:**
- Produces:
  - `pub trait FrameSource: Send { fn connect(&mut self, url: &str) -> Result<(), EddnError>; fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, EddnError>; fn disconnect(&mut self); }` + adaptador real `ZmqSource` (biblioteca de Task 0; SUB sin filtro de tema).
  - `pub const RELAY_URL: &str = "tcp://eddn.edcd.io:9500"`; `pub enum EddnMode { Always, OnDemand }`.
  - `pub enum EddnOutput { Batch(Vec<MarketSnapshot>, Vec<StationInfo>, Vec<(String, Option<u64>, [f64;3])>), Status(EddnStatus { connected: bool, stats: EddnStats }) }`; `Status { connected: true }` se emite **al establecerse la conexión** (señal que usan los tests y los e2e; no se cuentan suscriptores en el relay).
  - `pub trait Clock: Send { fn now(&self) -> Instant; fn sleep(&self, d: Duration); }` con `SystemClock` (real) y `FakeClock` (tests: avanza el tiempo y registra las esperas).
  - `pub fn spawn_subscriber(source: Box<dyn FrameSource>, clock: Box<dyn Clock>, url: String, filter: tokio::sync::watch::Receiver<EddnFilter>, out: tokio::sync::mpsc::Sender<EddnOutput>, seed: Vec<(String, Option<u64>, [f64;3])>) -> std::thread::JoinHandle<()>` (hilo propio; tiempos y esperas solo vía `clock`; lote cada **5 s o 20 mercados**, deduplicado por `market_id` quedándose el más nuevo; retroceso 1, 2, 4… 60 s y vuelta a 1 tras un mensaje válido; `enabled=false` → `disconnect` en ≤ 1 s; sale al cerrarse `out`).
  - `examples/fake_relay.rs`: PUB en `127.0.0.1:<puerto>` que, tras 1 s de calentamiento, **re-publica en bucle** (cada 1 s) el archivo indicado comprimido hasta que lo terminen (PUB pierde lo enviado antes de la suscripción; repetir es idempotente porque las instantáneas repetidas no son más nuevas). Solo pruebas.

- [ ] **Step 1: Tests** (`FakeSource` en memoria con guion de frames/errores; `FakeClock`):

```rust
#[test] fn batches_flush_at_20_markets() { /* 25 mercados cercanos → 1 lote de 20 inmediato y otro de 5 a los 5 s */ }
#[test] fn duplicate_market_in_batch_keeps_newest() { /* mismo market_id t1 y t2 → 1 instantánea con t2 */ }
#[test] fn reconnects_with_backoff_after_relay_drop() { /* recv → Err 3 veces → FakeClock registra esperas [1 s, 2 s, 4 s]; tras un frame válido la siguiente espera vuelve a 1 s */ }
#[test] fn connected_status_emitted_on_connect() { /* connect Ok → primer EddnOutput es Status{connected:true} */ }
#[test] fn disabled_filter_disconnects_within_1s() { /* filtro enabled=false → disconnect() llamado y 0 connect() mientras siga false */ }
#[test] fn corrupt_frames_never_stop_the_loop() { /* 100 frames basura + 1 válido → 1 mercado en el lote, stats.dropped[Corrupt]==100 */ }
#[test] fn zmq_loopback_roundtrip() { /* PUB local de la biblioteca elegida en 127.0.0.1:0 → ZmqSource recibe el frame idéntico (<= 2 s) */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(eddn): suscriptor ZeroMQ con reconexión, lotes y relay falso de pruebas`

### Task 12: `tmc-app` — servicio EDDN, `SourceEvent::Eddn`, limpieza y configuración

**Files:** Create `crates/tmc-app/src/eddn_service.rs`; Modify `crates/tmc-model/src/source.rs`, `crates/tmc-core/src/actor.rs` (`match` exhaustivo de `SourceEvent`, ~l. 380), `crates/tmc-app/src/{wiring.rs, config.rs (validación, junto a validate_f2), clipboard.rs (llamada a la validación en el PUT, ~l. 177), lib.rs}`, `apps/headless/src/main.rs`.

**Interfaces:**
- Produces:
  - `SourceEvent` + `Eddn(EddnNotice)`; `EddnNotice { markets: Vec<u64>, commodities: Vec<String> /* acotado a 200 */, connected: bool, at: DateTime<Utc> }` (aviso ligero: los datos viven en la tabla). Brazos en `match` exhaustivos del actor; ningún módulo previo cambia de comportamiento.
  - `pub fn eddn_wanted(s: &ModuleSettings, galaxy: Galaxy, mode: EddnMode, trade_or_mining_active: bool, inactive_since: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool` (offline o Legacy → `false`; `OnDemand`: activo o inactivo < 60 s).
  - `pub fn spawn_eddn_service(store: Store, core_tx: mpsc::Sender<SourceEvent>, filter_rx: watch::Receiver<EddnFilter>, mode: EddnMode, source: Box<dyn FrameSource>, url: String)`: al arrancar, siembra única de `system_coords` desde el diario propio (`FSDJump`/`Location`/`CarrierJump` de la tabla `event`, `StarSystem`+`SystemAddress`+`StarPos` vía `json_extract`, en `spawn_blocking`) y luego `CoordIndex` con `store.coords_within(centro, radio + 100, 30_000)`; cada `Batch` → `spawn_blocking(replace_markets + upsert_stations + upsert_coords)` (las coordenadas de `journal/1` de EDDN también se persisten) → `SourceEvent::Eddn`; `purge_prices_older_than(now − 7 d)` al arrancar y cada hora.
  - Coordenadas propias en vivo: cada cambio de sistema que `wiring` usa para el centro de `EddnFilter` se escribe con `upsert_coords` (nombre, `SystemAddress`, `StarPos`), aunque EDDN esté desactivado.
  - `wiring`: alimenta `watch<EddnFilter>` desde ajustes, galaxia, posición y relevancias del núcleo (verificar en código al ejecutar cómo `wiring` observa topics/estado). En depuración, `TMC_E2E_EDDN_RELAY` (solo `tcp://127.0.0.1:<puerto>`) sustituye `RELAY_URL`, como `TMC_E2E_ARDENT_BASE` en F2.
  - Validación en el `PUT` de los campos añadidos en Task 7: `eddn_radius_ly` 20–500, `tourism_reference_cr_ly` 1 000–10 000 000 (`Config::validate_f3` en `config.rs`, invocada junto a `validate_f2` desde `clipboard.rs`).

- [x] **Step 1: Tests:**

```rust
#[test] fn eddn_disabled_in_legacy_or_offline() { /* Legacy → false; online_services=false → false; Live+online+Always → true */ }
#[test] fn on_demand_keeps_60s_after_inactive() { /* inactivo hace 59 s → true; 61 s → false */ }
#[tokio::test] async fn batch_is_written_and_notified() { /* FakeSource con 3 mercados cercanos → market_prices(id) no vacío y llega SourceEvent::Eddn con markets.len()==3 */ }
#[tokio::test] async fn purge_runs_at_start() { /* fila de hace 8 d antes de arrancar → 0 filas tras el arranque */ }
#[tokio::test] async fn own_jumps_seed_system_coords() { /* tabla event con FSDJump "Lave" StarPos [75.75,48.75,70.75] → coords_by_names(["lave"]) == {"lave": [75.75,48.75,70.75]} tras arrancar */ }
#[test] fn f2_config_without_f3_fields_loads_with_defaults() { /* TOML de F2 → eddn_radius_ly 200, tourism_reference_cr_ly 100000 */ }
#[test] fn put_config_rejects_eddn_radius_out_of_range() { /* 600 → 400 y sin cambios */ }
#[test] fn e2e_relay_override_only_loopback() { /* "tcp://10.0.0.1:1" → rechazado; "tcp://127.0.0.1:5555" → aceptado (solo debug) */ }
```

- [x] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(app): servicio EDDN con escritura por lotes, limpieza y ajustes de F3`

### Task 13: datos estáticos `rare_goods.json` y `cabins.json`

**Files:** Create `crates/tmc-modules/data/{rare_goods.json, cabins.json}`; Modify `crates/tmc-modules/src/data.rs`.

**Interfaces:** `RareGood { key: String /* commodity_key */, origin_system: String, origin_station: String, sell_distance_ly: u32, source: String, verified: bool }`, `fn rare_goods() -> &'static HashMap<String, RareGood>`; `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)] CabinClass { Economy, Business, First, Luxury }`, `CabinSpec { symbol: String, class: CabinClass, seats: u32, source, verified }`, `fn cabin_spec(symbol: &str) -> Option<&'static CabinSpec>` (símbolo en minúsculas, p. ej. `int_passengercabin_size4_class1`). Hechos del juego redactados por el proyecto (no copiados de listas con licencia incompatible), `verified: false` salvo comprobación en el juego.

- [x] **Step 1: Tests:**

```rust
#[test] fn rare_goods_load_and_have_provenance() { /* len > 0, todas con source no vacío; "lavianbrandy" → origin_system "Lave", sell_distance_ly >= 150 */ }
#[test] fn cabin_specs_known_sizes() { /* size2_class1 → Economy 2 plazas; size4_class1 → Economy 8; size5_class4 → Luxury 4; size6_class3 → First 12 */ }
#[test] fn corrupt_static_data_is_ignored_with_warning() { /* JSON inválido → mapa vacío, sin pánico */ }
```

- [x] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(modules): datos propios de mercancías raras y cabinas con procedencia`

### Task 14: i18n — `trade.ftl` y `missions.ftl`

**Files:** Create `locales/{es,en,fr,de,pt}/{trade.ftl, missions.ftl}`; Modify `locales/*/main.ftl` (disposiciones y ajustes), `crates/tmc-i18n/src/lib.rs` (lista de argumentos).

**Interfaces:** claves de todos los paneles (títulos, columnas, «RARA», «VENDER AQUÍ», «LLEVAR», «solo mercado negro», «sin datos», «atraca para calcular», «RENTABLE», «NO COMPENSA», «puedes abandonarla ahora», «paradas no disponibles en el diario», «BUSCADO», «VIP», avisos), `trade-rank-0..8` en inglés en los 5 idiomas, `nav-trade` (en `trade.ftl`) y `nav-missions` (en `missions.ftl`), como `nav-mining` en `mining.ftl`; `layout-trade`, `layout-missions`, ajustes `eddn-radius`, `tourism-reference` (en `main.ftl`). Es la base: las tareas de UI (21, 22, 29, 30, 31) pueden añadir las claves que les falten, siempre en los 5 idiomas y en el mismo commit.

- [x] **Step 1: Tests:** los existentes `all_languages_have_same_keys_as_es`, `keys_include_all_ftl_files` y el de formato con la lista de argumentos ampliada; nuevo `trade_rank_names_are_english_in_all_languages` (`trade-rank-8` = «Elite» en es/en/fr/de/pt).
- [x] **Step 2–4:** FAIL → añadir → PASS; Checks. **Step 5: Commit** `feat(i18n): textos de comercio y misiones en cinco idiomas`

---

## Bloque 2 — Comercio

### Task 15: `trade` — coste pagado y «Mi carga»

**Files:** Create `crates/tmc-modules/src/trade/{mod.rs (esqueleto con `mod cost;`), cost.rs}`, `tests/fixtures/journal/trade_session.log` (base, ver Task 32); Modify `crates/tmc-modules/src/lib.rs` (`pub mod trade;`).

**Interfaces:**
- Consumes: `MarketBuy`, `MarketSell`, `EjectCargo`, `Cargo` (`inventory: Option<Vec<events::CargoItem>>`, la única fuente de inventario), `CargoItem::{is_mission, stolen_count}`.
- Produces: `CostBasis { fn apply_event(&mut self, ev: &EventKind); fn reconcile(&mut self, inventory: &[CargoItem]); fn avg_paid(&self, commodity: &str) -> Option<u64>; fn is_partial(&self, commodity: &str) -> bool }` (media ponderada; vender/expulsar resta a la media sin cambiarla; `reconcile` escala si sobra y marca parcial si falta: carga de origen desconocido); `CargoLine { commodity, name_localised, tons: u32, paid_avg: Option<u64>, paid_partial: bool, price_here: Option<u64>, best_nearby: Option<SellOption>, est_profit: Option<i64>, advice: Option<CargoAdvice>, kind: CargoKind }`, `enum CargoKind { Trade, Mission, Stolen }`, `enum CargoAdvice { SellHere, Carry }` (`SellHere` si `price_here ≥ 0,98 × best_nearby` o no hay mejor venta y `price_here > paid_avg`); `CargoView { lines: Vec<CargoLine>, used: u32, capacity: Option<u32> }` con `fn build(inventory: &[CargoItem], basis: &CostBasis, here: &HashMap<String, u64> /* precio de venta aquí, de Market.json */, best: &HashMap<String, SellOption>, capacity: Option<u32>) -> CargoView`; `fn cost_rebuild_task(cmdr: String, since: DateTime<Utc>) -> StoreTask` (tag `trade.cost`, lectura; paginando con `history::for_each_after`; 7 días).

- [x] **Step 1: Tests:**

```rust
#[test] fn weighted_average_over_two_buys() { /* 100 t a 9400 y 50 t a 9700 → avg_paid("gold") Some(9500) */ }
#[test] fn sell_keeps_average_and_reduces_tons() { /* tras vender 60 t → avg 9500, 90 t */ }
#[test] fn mission_and_stolen_cargo_excluded_from_profit() { /* 10 t con MissionID y 3 robadas → kind Mission / Stolen, est_profit None */ }
#[test] fn unknown_origin_cargo_marks_paid_partial() { /* MarketBuy 50 t; Cargo con 80 t → is_partial true, avg sobre lo conocido 9400 */ }
#[test] fn profit_and_advice() { /* paid 9500, here 10000, best 11000 a 12 al → est_profit (11000−9500)×90 = 135000, Carry; here 10900 → SellHere */ }
#[test] fn cost_rebuilt_after_restart_matches_live() { /* store en memoria con los mismos eventos → avg y toneladas iguales a las de en vivo */ }
```

- [x] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(trade): coste pagado medio ponderado y vista de la carga`

### Task 16: `trade` — «Mercado actual» y mejor venta en 48 h

**Files:** Create `crates/tmc-modules/src/trade/market.rs`, `tests/fixtures/companion/market_trade.json`; Modify `trade/mod.rs` (`mod market;`), `crates/tmc-modules/src/mining/sell.rs` (`merge_sources`, `SellOption.origin`; `pad_ok` público si hace falta; resto sin cambios).

**Interfaces:**
- Consumes: `Store::best_prices` (ya filtra 48 h), `Store::replace_markets`, `mining::sell::{rank_markets, SellOption}`, `data::{ship_pads, rare_goods}`, `CompanionFile::Raw(Market, Value)`.
- Produces:
  - `mining::sell`: `SellOption` + `origin: Option<PriceOrigin>` (copiado de `RemoteMarket.origin`); `pub fn merge_sources(local: &[PricedMarket], remote: &[RemoteMarket]) -> Vec<RemoteMarket>` (convierte `PricedMarket` con su `origin`; filas sin distancia fuera; **deduplica por `market_id`** —o por (estación, sistema) si falta— quedándose el dato **más nuevo** aunque sea más barato; después se ordena con `rank_markets`). Neutral: la usan `trade` y `mining`.
  - `fn snapshot_from_market_json(json: &Value, docked: &Docked, star_pos: [f64;3], now) -> Option<MarketSnapshot>` (origen `Local`; tolerante; guarda `Name_Localised`); `fn best_sells(rows: &[PricedMarket], ardent: &[RemoteMarket], ship_pad: Option<PadSize>, top: usize) -> Vec<SellOption>` = `rank_markets(&merge_sources(rows, ardent), ship_pad, top)` (sin lógica de tiempo: la ventana de 48 h la aplica `Store::best_prices`); `CurrentMarketRow { commodity, name_localised, buy_price, stock, best_sell: Option<SellOption>, profit_per_ton: Option<i64>, age_s: Option<i64>, rare: Option<u32 /* sell_distance_ly */> }`; `CurrentMarket { station, system, rows: Vec<CurrentMarketRow>, as_of }`: 5 filas; primero las compras con venta conocida por beneficio/t desc; si hay menos de 5, se completan con compras sin venta conocida (`best_sell: None`, «sin datos») por precio de compra desc; `fn best_sells_task(cmdr: String, commodities: Vec<String>, center: [f64;3], radius_ly: f64, since, exclude_market: u64) -> StoreTask` (tag `trade.best_sells`, lectura).

- [ ] **Step 1: Tests:**

```rust
#[test] fn top_five_buys_by_profit_per_ton() { /* 7 mercancías compradas aquí con mejor venta conocida → 5 filas ordenadas; la 1.ª gold con 11000−9400 = 1600 */ }
#[test] fn merge_sources_dedups_by_market_newest_wins_even_if_cheaper() { /* mismo market_id: Ardent t1 230000 y EDDN t2 225000 → 1 entrada, 225000, origin Some(Eddn) */ }
#[test] fn fewer_than_five_known_sells_filled_with_no_data_rows() { /* 3 con venta conocida + 4 sin → 5 filas; filas 4–5 best_sell None ordenadas por buy_price desc */ }
#[test] fn incompatible_pad_excluded_for_large_ship() { /* mejor venta solo M y nave L → la siguiente compatible */ }
#[test] fn unknown_pad_included_with_warning() { /* max_pad None → incluida, pad_ok None */ }
#[test] fn no_known_sell_is_no_data() { /* sin filas → best_sell None, profit None */ }
#[test] fn rare_good_is_marked() { /* "lavianbrandy" → rare Some(<distancia del json>) */ }
#[test] fn market_json_to_snapshot() { /* market_trade.json → prices.len()==<n>, gold buy 9400 stock 5000 */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(trade): mercado actual con mejor venta de las últimas 48 h`

### Task 17: `trade` — ruta comercial con avance automático

**Files:** Create `crates/tmc-modules/src/trade/route.rs`; Modify `trade/mod.rs` (`mod route;`).

**Interfaces:**
- Consumes: `TradeRoutePlan`, `Docked`, `ShipState { cargo_capacity, max_jump_range }`, `CommanderState.credits`, `data::ship_pads`.
- Produces: `fn route_params(state: &GameState, ship_pad: Option<PadSize>, max_hops: u8) -> Result<TradeRouteParams, RouteBlocker>` (`RouteBlocker::{NotDocked, NoShip, NoCredits}`; `max_hop_distance_ly` = alcance real); `TradeRouteState { plan: Option<TradeRoutePlan>, current: usize, completed: bool, copy_target: Option<String>, copy_seq: u64, requested_at: Option<DateTime<Utc>>, fn on_plan(&mut self, plan); fn on_docked(&mut self, d: &Docked) -> Option<String> /* sistema siguiente a copiar */; fn jumps(&self, stop: usize, range: f64) -> Option<u32> /* ceil(distancia/alcance) */ }`. Avance: atracar en `to_station` de la parada actual → `current += 1`, `copy_target = Some(to_system de la nueva parada)`, `copy_seq += 1` y lo devuelve; atracar en cualquier otra estación no cambia nada; tras la última, `completed = true`, `None` y `copy_seq` sin cambios. `on_plan` pone `current = 0`, `completed = false` y `copy_target` = `to_system` de la primera parada (`copy_seq += 1`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn params_use_real_ship_and_credits() { /* capacidad 256, créditos 5_000_000, alcance 22.5, nave Large → max_cargo 256, starting_capital 5000000, min_pad Large, max_hop_distance_ly 22.5 */ }
#[test] fn not_docked_blocks_route() { /* en supercrucero → Err(NotDocked) */ }
#[test] fn docking_at_next_stop_advances_and_returns_next_system() { /* paradas A→B, B→C; tras on_plan copy_seq 1; Docked B → current 1, devuelve Some("C"), copy_target Some("C"), copy_seq 2 */ }
#[test] fn docking_elsewhere_does_not_advance() { /* Docked X → current 0, None */ }
#[test] fn last_stop_completes_route() { /* Docked C con current 1 → current 2, None, completed true */ }
#[test] fn jumps_estimate() { /* 47 al con alcance 22.5 → 3 */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(trade): ruta comercial con avance automático`

### Task 18: `trade` — sesión de comercio y buscar mercancía

**Files:** Create `crates/tmc-modules/src/trade/{session.rs, search.rs}`; Modify `trade/mod.rs` (`mod session; mod search;`).

**Interfaces:**
- Produces: `TradeSession { fn apply_event(&mut self, ev: &EventKind, basis: &CostBasis, at); fn reset(&mut self, at); started_at, profit: i64, revenue: u64, cost_of_sold: u64, tons_bought: u32, tons_sold: u32, operations: u32, best_trade: Option<(String, i64)>, fn credits_per_hour(&self, now) -> Option<i64> }` (beneficio = Σ `MarketSell.total_sale` − Σ `count × avg_paid` del `CostBasis` **antes** de la venta: **orden obligatorio por evento** `TradeSession::apply_event` → después `CostBasis::apply_event`; si no hay media, usa `AvgPricePaid` del evento; `LoadGame` reinicia, spec §10.4); rango: `Rank.trade` y `Progress.trade` del estado. `SearchResult { commodity, rows: Vec<BuyOption { station_name, system_name, distance_ly, buy_price, stock, pad: Option<PadSize>, pad_ok: Option<bool>, updated_at, origin }> }`; `fn merge_buys(local: &[PricedMarket], ardent: &[RemoteExport], ship_pad, top: usize) -> Vec<BuyOption>` (los `RemoteExport` se convierten con `origin: Ardent`; mismo mercado → el dato más nuevo; luego precio asc); `fn search_task(cmdr, commodity, center, radius_ly) -> StoreTask` (7 días, tag `trade.search`, lectura).

- [ ] **Step 1: Tests:**

```rust
#[test] fn session_profit_equals_sales_minus_avg_cost() { /* compra 100 t a 9400, vende 100 t a 11000 → profit 160000, ops 2, tons_bought 100, tons_sold 100 */ }
#[test] fn session_reads_basis_before_sale_is_applied() { /* compra 10 t a 9000; venta de 10 t a 11000: session.apply_event y luego basis.apply_event → profit 20000 (no 110000) */ }
#[test] fn session_uses_avg_price_paid_when_no_basis() { /* venta sin compra previa con AvgPricePaid 9000, 10 t a 11000 → profit 20000 */ }
#[test] fn load_game_resets_session() { /* beneficio previo → LoadGame → profit 0 */ }
#[test] fn credits_per_hour_after_30_min() { /* 160000 en 30 min → 320000 */ }
#[test] fn merge_buys_dedups_by_market_newest_wins() { /* mismo market_id local (t2) y Ardent (t1) → 1 fila con t2 */ }
#[test] fn search_excludes_incompatible_pad() { /* nave L, estación M → fuera; desconocida → pad_ok None */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(trade): sesión de comercio y búsqueda de mercancía`

### Task 19: `trade` — módulo, topics, relevancia, red, almacén y registro

**Precisión de Task 15:** `trade.cost` devuelve `{cmdr,basis,cut,cut_pos}`; el módulo comprueba comandante/generación y concilia lo recibido desde el corte, como `combat`. El llamador pasa `since = reloj − 7 días`.

**Files:** Create `crates/tmc-modules/src/trade/tests.rs`, `crates/tmc-modules/src/topicx.rs`; Modify `trade/mod.rs`, `crates/tmc-modules/src/{lib.rs, registry.rs}`, `crates/tmc-modules/src/{combat/mod.rs, mining/mod.rs}` (sus `strip_as_of` privados pasan a `topicx::strip_as_of`), `crates/tmc-app/src/clipboard.rs` (`MODULE_PREFIXES` + `"trade."`, `"missions."`; `copy-next-system` también lee `trade.route.copy_target/copy_seq`).

**Interfaces:**
- Produces: `pub struct Trade` (`Trade::new()`, `id() = "trade"`; guarda el último `Docked` propio para `relevance` y los parámetros de ruta, porque `LocationState` no lo conserva); topics `trade.market` (`CurrentMarket` + `eddn_connected: bool` del último `SourceEvent::Eddn`), `trade.cargo` (`CargoView`), `trade.route` (`TradeRouteState` + `status: "idle"|"loading"|"ok"|"no_data"|"disabled"|"blocked"`), `trade.session`, `trade.search` (`SearchResult` + `status`), `trade.commodities` (≤ 400 pares clave + nombre localizado, de `commodities_task(cmdr)` — tag `trade.commodities`, lectura con `Store::known_commodities(400)` — al arrancar y tras cada `Market.json` propio), todos con `as_of`.
  - Orden por evento: `TradeSession::apply_event` antes de `CostBasis::apply_event` (Task 18).
  - Atraque con mercado o `Market.json` nuevo → `store_task` `trade.market.write` (`replace_markets`, origen `Local`; nunca en replay) + `best_sells_task`; `SourceEvent::Eddn` con alguna mercancía de la vista → nueva consulta como máximo cada **10 s** (`EDDN_REQUERY_MIN_S`); `CommodityImportsNearby` (F2) para las mercancías de la carga y del top del mercado sin venta en 48 h (máx. 10); `CommodityExportsNearby` al buscar.
  - Ingesta de Ardent: `fn ardent_rows(markets: &[RemoteMarket], exports: &[RemoteExport], commodity: &str, fetched_at) -> Vec<(StationInfo, CommodityPrice, DateTime<Utc>, PriceOrigin)>` (`RemoteMarket` → `sell_price`/`demand`; `RemoteExport` → `buy_price`/`stock`; `updated_at` o `fetched_at`; origen `Ardent`); `ardent_write_task(rows)` (tag `trade.ardent.write`, nunca en replay) rellena `star_pos` con `coords_by_names` dentro de la propia tarea y **solo escribe las filas con coordenadas**; los resultados de Ardent del sistema actual se usan además en memoria (las respuestas de otro sistema no se aplican).
  - Acciones: `trade.route_request { max_hops }`, `trade.route_clear`, `trade.reset_session`, `trade.search { commodity }`. Avance de ruta → `trade.route.copy_target/copy_seq` (Task 17); `clipboard.rs` lo copia cuando cambia `copy_seq` y `auto_copy_next_system` está activo, igual que `navigation.route`.
  - `relevance`: `Active` si atracado en estación con mercado (`Docked::has_market`) o con carga `Trade` > 0; si no, `Normal`.
  - Cambio de comandante → `CostBasis`, ruta, sesión y búsqueda vacías; `cost_rebuild_task` al arrancar/`LoadGame`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn trade_fixture_current_market_top5() { /* trade_session.log + market_trade.json + store sembrado → trade.market.rows.len()==5, rows[0].commodity "gold" */ }
#[test] fn replay_emits_no_requests_or_store_writes() { /* replaying → 0 NetRequest, 0 tags "*.write" */ }
#[test] fn ardent_rows_without_coords_stay_in_memory_only() { /* Markets de 2 estaciones, coords solo de una → ardent_write_task escribe 1 fila; trade.market usa las 2 */ }
#[test] fn no_requests_in_legacy_or_offline() { /* Legacy → 0; offline → 0 y trade.route.status "disabled" */ }
#[test] fn stored_result_for_other_cmdr_is_ignored() { /* Stored{tag:"trade.best_sells"} con cmdr "OTRO" → sin cambios */ }
#[test] fn commander_switch_clears_cost_basis_and_route() { /* Commander "B" tras cargar en "A" → trade.cargo sin paid_avg, ruta None */ }
#[test] fn late_exports_for_previous_system_not_applied() { /* Exports de X tras saltar a Y → trade.search sin ellos */ }
#[test] fn eddn_burst_requeries_at_most_every_10s() { /* 50 SourceEvent::Eddn en 3 s → 1 store_task trade.best_sells */ }
#[test] fn route_advance_copies_next_system() { /* plan de 2 paradas, Docked en B → topic trade.route copy_target "C", copy_seq incrementado */ }
#[test] fn trade_topics_stay_under_16_kib() { /* 400 mercancías, carga de 30 líneas, 10 paradas → cada topic < 16384 */ }
#[test] fn relevance_active_when_docked_with_market() { /* Docked con "commodities" → Active; Undocked sin carga → Normal */ }
```
  más en `crates/tmc-app/src/clipboard.rs` (tests): `trade_and_missions_actions_reach_core` (`trade.reset_session` y `missions.dismiss_alert` llegan como `SourceEvent::Action` y no como acción de aplicación; solo comprueba el prefijo, la acción de `missions` la consume Task 28) y `trade_route_copy_target_reaches_clipboard` (`trade.route` con `copy_target "C"` y `copy_seq` nuevo + `auto_copy_next_system` → portapapeles falso contiene "C"; con el ajuste desactivado, vacío). Los tests de F2 de `combat`/`mining` pasan sin cambios tras mover `strip_as_of`.

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(trade): módulo de comercio con topics, relevancia, red y registro`

### Task 20: `mining` — «Dónde vender» con la tabla de precios

**Files:** Modify `crates/tmc-modules/src/mining/{mod.rs, sell.rs}`, `crates/tmc-modules/src/mining/tests.rs`.

**Interfaces:** `mining` pide además `best_prices(commodity, Sell, pos, sell_search_radius_ly, now−48h)` por mineral de la carga (tag `mining.local_sells`, lectura, con `cmdr`) y, por mineral, pasa a `SellPlan::build` (que sigue recibiendo `BTreeMap<String, Vec<RemoteMarket>>`) el vector `mining::sell::merge_sources(local, ardent)` (Task 16; sin depender de `trade`); `SourceEvent::Eddn` relanza la consulta como máximo cada 10 s; `PriceBook::best_price` sigue siendo el único origen del valor del prospector (lo local de la tabla entra como `Remote` con su `at`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn sell_plan_includes_eddn_prices_when_ardent_disabled() { /* offline + tabla con platinum 230000 de EDDN hace 1 h → mining.sell.status "ok", 1 opción origin Eddn */ }
#[test] fn eddn_and_ardent_same_market_newest_wins_even_if_cheaper() { /* EDDN t2 225000 y Ardent t1 240000 del mismo mercado → 1 opción, 225000, origin Some(Eddn) */ }
#[test] fn sell_topic_with_two_sources_stays_under_16_kib() { /* 10 minerales × (50 Ardent + 50 tabla) → < 16384 bytes, 5 por mineral; los tests de F2 de mining pasan sin cambios */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(mining): dónde vender con precios locales y de EDDN`

### Task 21: UI — «Mercado actual» y «Mi carga»

**Files:** Create `ui/src/modules/trade/{index.ts, types.ts, MarketPanel.svelte, CargoPanel.svelte}`, `ui/tests/trade.test.ts`; Modify `ui/src/modules/registry.ts`, `locales/*/trade.ftl`.

**Interfaces:** según spec §3.2: mercado (5 filas: mercancía, compra, mejor venta con estación·distancia·plataforma·antigüedad, beneficio/t, insignia «RARA» con distancia recomendada); carga (líneas con t, pagado, aquí, mejor cercana, beneficio ≈, «VENDER AQUÍ»/«LLEVAR», marcas misión/robada «solo mercado negro», barra de bodega). Registro `{ id: 'trade', titleKey: 'nav-trade', panels: tradePanels }`.

- [ ] **Step 1: Tests:**

```ts
test('market row shows profit distance pad and age', /* gold 9400 → 11000, 12,3 al, L, hace 2 h → "1.600" y "hace 2 h" (es) */);
test('unknown pad shows warning', /* pad_ok null → icono/aviso trade-pad-unknown */);
test('rare badge with distance', /* rare 160 → "RARA" y "160 al" */);
test('stolen cargo shows black market only and no profit', /* kind Stolen → trade-black-market-only, sin beneficio */);
test('hold bar used over capacity', /* 180/256 → aria-valuenow 180 */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): paneles de mercado actual y mi carga`

### Task 22: UI — «Ruta comercial», «Sesión» y «Buscar mercancía»

**Files:** Create `ui/src/modules/trade/{RoutePanel.svelte, TradeSessionPanel.svelte, SearchPanel.svelte}`; Modify `index.ts`, `locales/*/trade.ftl`, `ui/tests/trade.test.ts`.

**Interfaces:** ruta (paradas «compra X en A → vende en B», beneficio/t, por parada, distancia, saltos, antigüedad; parada actual destacada; total; «Recalcular» → `trade.route_request`; `blocked` → «atraca para calcular»); sesión (duración, beneficio, cr/h, t compradas/vendidas, operaciones, mejor operación, rango de comercio en inglés con %; «reiniciar sesión»); búsqueda (campo con `<datalist>` de `trade.commodities` → `trade.search`, tabla de dónde comprar).

- [ ] **Step 1: Tests:**

```ts
test('route highlights current stop and total', /* current 1 de 3 → fila 2 con clase current; total "3.200.000" */);
test('recalculate posts json action', /* POST trade.route_request con Content-Type application/json */);
test('blocked route shows dock hint', /* status "blocked" → trade-route-dock-first */);
test('session shows english trade rank', /* rank 8 → "Elite" */);
test('search autocompletes known commodities', /* 3 claves → 3 <option> */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): paneles de ruta comercial, sesión y búsqueda de mercancía`

---

## Bloque 3 — Misiones, masacre, pasajeros y turismo

### Task 23: `missions` — libro de misiones y reconstrucción

**Files:** Create `crates/tmc-modules/src/missions/{mod.rs (esqueleto con `mod book;`), book.rs}`, `tests/fixtures/journal/missions_session.log` (base, ver Task 32); Modify `crates/tmc-modules/src/lib.rs` (`pub mod missions;`).

**Interfaces:**
- Produces: `enum MissionKind { Massacre, Sightseeing, Passenger, Delivery, Collect, Courier, Other }` con `fn classify_kind(name: &str, passenger: bool) -> MissionKind` (prefijos del spike); `Mission { id: u64, kind, name_localised: String, faction: Option<String>, destination_system: Option<String>, destination_station: Option<String>, stops: Vec<String>, accepted_at: Option<DateTime<Utc>>, expiry: Option<DateTime<Utc>>, reward: Option<u64>, progress: MissionProgress { delivered: u32, collected: u32, total: Option<u32>, kills_needed: Option<u32>, passengers: Option<u32> } /* las bajas no se guardan aquí: MassacreStack::kills_for */, redirected: bool, details_missing: bool, target_faction: Option<String>, wing: bool }`; `MissionBook { fn apply_event(&mut self, ev: &EventKind, at: DateTime<Utc>) -> Vec<BookChange>; fn active(&self) -> impl Iterator<Item=&Mission>; fn on_history_accepted(&mut self, accepted: Vec<(MissionAccepted, DateTime<Utc>)>) }`; `BookChange::{Added(u64), Completed(u64), Failed(u64), Abandoned(u64), Redirected { id: u64, system: String, station: Option<String> }, Progress(u64)}` (de aquí salen los avisos de fallida/redirigida, Task 25). Regla `Missions`: la lista activa **manda** (lo que no está se descarta; lo que está sin detalles queda con `details_missing: true`, nombre desde `Name` y caducidad = `ts + Expires`); `fn accepted_rebuild_task(cmdr, ids: Vec<u64>) -> StoreTask` (tag `missions.accepted`, lectura; SQL propio sobre la tabla `event` con `json_extract(…,'$.MissionID') IN (…)`, no `history::*`).

- [ ] **Step 1: Tests:**

```rust
#[test] fn accepted_then_completed_lifecycle() { /* Accepted 65000001 → active 1; Completed → active 0, BookChange::Completed */ }
#[test] fn missions_event_keeps_unknown_mission_minimal() { /* Missions con 65000009 sin historial, Expires 3600 a 12:00 → details_missing true, expiry 13:00 */ }
#[test] fn accepted_not_in_missions_event_is_dropped() { /* historial con 65000002 que no está en Missions.Active → no activa */ }
#[test] fn history_details_fill_minimal_entries() { /* tras Stored missions.accepted con 65000009 → details_missing false, reward Some(<valor>) */ }
#[test] fn redirected_updates_destination() { /* NewDestinationSystem "Lave" → destination_system Some("Lave"), redirected true */ }
#[test] fn cargo_depot_updates_progress() { /* Deliver ItemsDelivered 30 Total 60 → delivered 30, total Some(60) */ }
#[test] fn sightseeing_stops_from_destination() { /* DestinationSystem con 3 paradas → stops.len()==3, kind Sightseeing */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(missions): libro de misiones con reconstrucción desde Missions y el historial`

### Task 24: `missions` — pila de masacre

**Files:** Create `crates/tmc-modules/src/missions/massacre.rs`, `tests/fixtures/journal/massacre_session.log`; Modify `missions/mod.rs` (`mod massacre;`).

**Interfaces:**
- Consumes: `Mission` (kind `Massacre`, `target_faction`, `kills_needed`, `accepted_at`, `faction`), `Bounty::victim_faction`, `FactionKillBond` solo si `MASSACRE_COUNTS_KILL_BONDS`.
- Produces: `MassacreStack` (**única fuente de bajas**) `{ fn apply_event(&mut self, ev: &EventKind, at, book: &MissionBook); fn kills_for(&self, mission_id: u64) -> u32; fn view(&self, book: &MissionBook) -> Vec<MassacrePile> }`; misiones con `accepted_at: None` (sin detalles) quedan **fuera** del reparto de bajas y se muestran con `details_missing`; `MassacrePile { target_faction: String, issuers: Vec<IssuerBar { faction: String, kills: u32, needed: u32, missions: u32 }>, kills_remaining: u32 /* máx. por emisor de (needed − kills) */, total_reward: u64 }`. Regla exacta (spec §10.10): solo bajas con `victim_faction == target_faction` y `at > accepted_at`; cada baja, **por cada facción emisora**, va a su misión activa no completa más antigua de esa facción objetivo (empate → `MissionID` menor); `MissionRedirected` → completa aunque no cuadre; facciones objetivo distintas = pilas distintas.

- [ ] **Step 1: Tests:**

```rust
#[test] fn one_kill_counts_once_per_issuer_oldest_first() { /* emisores A (2 misiones de 10 y 5) y B (1 de 8); 12 bajas → A: misión 1 10/10, misión 2 2/5; B 8/8 → kills_remaining 3 */ }
#[test] fn kill_before_accept_not_counted() { /* Bounty a 11:59, aceptada a 12:00 → kills 0 */ }
#[test] fn other_target_faction_is_other_pile() { /* objetivos X e Y → 2 pilas; baja de X no suma en Y */ }
#[test] fn redirected_marks_complete_even_if_count_short() { /* 7/10 + MissionRedirected → misión completa, la siguiente baja va a la siguiente misión del emisor */ }
#[test] fn tie_on_accept_time_goes_to_lower_mission_id() { /* misma hora, ids 65000005 y 65000004 → primero 65000004 */ }
#[test] fn mission_without_accept_time_gets_no_kills() { /* misión de masacre con accepted_at None + 3 bajas → kills_for == 0 y las bajas van a la otra misión del emisor */ }
#[test] fn fixture_five_missions_three_factions_matches_manual() { /* massacre_session.log (su misión redirigida llega a 10/10 antes de MissionRedirected) → por emisor y kills_remaining == <cálculo manual en el fixture> (criterio 7) */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(missions): pila de masacre con la regla exacta del juego`

### Task 25: `missions` — avisos, recompensas de la sesión y objetivos comunitarios

**Files:** Create `crates/tmc-modules/src/missions/{alerts.rs, rewards.rs}`; Modify `missions/mod.rs` (`mod alerts; mod rewards;`).

**Interfaces:**
- Produces: `MissionAlert { mission_id: u64, kind: AlertKind, at }`, `enum AlertKind { ExpiringSoon, Redirected { system: String, station: Option<String> }, Failed, CargoMissing { commodity: String, missing: u32 } }`; `fn alert_from_change(change: &BookChange, at) -> Option<MissionAlert>` (**fallida y redirigida** salen del `BookChange` que devuelve `MissionBook::apply_event`); `fn evaluate_alerts(book: &MissionBook, cargo: &[CargoItem], now: DateTime<Utc>, already: &HashSet<(u64, u8)>) -> Vec<MissionAlert>` (**solo** las de tiempo y carga: caduca < 1 h una vez por misión; carga que falta = entrega pendiente − carga de esa misión a bordo); `SessionRewards { fn apply_event(&mut self, ev, at); fn reset(&mut self, at); credits: u64, completed: u32, failed: u32, materials: BTreeMap<String, u32>, factions: BTreeMap<String, FactionDelta { reputation: String, influence: String }> }` (`LoadGame` reinicia); `CommunityGoals { fn apply(&mut self, ev: &CommunityGoal); goals: Vec<CgView { cgid, title, system, station, contribution, percentile, tier, expiry, bonus, complete }> }` (cada `CommunityGoal` sustituye la lista).

- [ ] **Step 1: Tests:**

```rust
#[test] fn expiring_under_one_hour_alerts_once() { /* expiry now+59 min → 1 alerta; segunda evaluación → 0 */ }
#[test] fn redirected_and_failed_alert_from_book_changes() { /* BookChange::Redirected{system:"Lave"} y BookChange::Failed → alert_from_change da Redirected{system:"Lave"} y Failed; evaluate_alerts no las repite */ }
#[test] fn missing_cargo_alert() { /* entrega de 60 gold, 30 entregadas, 20 a bordo → CargoMissing{missing:10} */ }
#[test] fn session_rewards_sum_and_reset() { /* 2 Completed 1_000_000 + 500_000 y 1 Failed → credits 1500000, completed 2, failed 1; LoadGame → 0 */ }
#[test] fn rewards_track_materials_and_faction_trends() { /* MaterialsReward tellurium 2 + 1 → 3; Influence "+++" → factions["A"].influence "+++" */ }
#[test] fn community_goal_view() { /* CGID 750, PercentileBand 25, Bonus 15000000 → percentile Some(25), bonus Some(15000000) */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(missions): avisos, recompensas de la sesión y objetivos comunitarios`

### Task 26: `missions` — pasajeros y cabinas

**Files:** Create `crates/tmc-modules/src/missions/passengers.rs`; Modify `missions/mod.rs` (`mod passengers;`).

**Interfaces:** `Cabins { by_class: BTreeMap<CabinClass, CabinOccupancy { occupied: Option<u32>, seats: u32 }>, occupied_total: u32, seats_total: u32 }` con `fn from_loadout(modules: &[LoadoutModule]) -> Cabins` (vía `data::cabin_spec`; símbolos desconocidos se ignoran); `fn apply_manifest(&mut self, p: &Passengers)` (total por manifiesto; por clase solo si el spike encontró la clase, si no `occupied: None` por clase y se muestra el agregado — ver «Desviaciones»); `PassengerGroup { destination: String, count: u32, vip: bool, wanted: bool, mission_id }`; `fn groups(book: &MissionBook, p: &Passengers) -> Vec<PassengerGroup>`; `fn wanted_aboard(p: &Passengers) -> bool`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn cabins_from_loadout() { /* size4_class1 (Economy 8) + size5_class4 (Luxury 4) → seats_total 12, by_class[Luxury].seats == 4 */ }
#[test] fn manifest_sets_occupancy() { /* 3 + 4 pasajeros → occupied_total 7; sin clase en el manifiesto → by_class[Economy].occupied None */ }
#[test] fn wanted_passenger_flag() { /* Wanted true en uno → wanted_aboard true */ }
#[test] fn groups_by_destination() { /* 2 misiones a "Lave" y 1 a "Leesti" → 2 grupos, Lave count = suma */ }
#[test] fn unknown_cabin_symbol_ignored() { /* "int_passengercabin_size9_class9" → seats_total 0 */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(missions): pasajeros a bordo y cabinas por clase`

### Task 27: `missions` — turismo: ruta óptima, cr/al y veredicto

**Files:** Create `crates/tmc-modules/src/missions/tourism.rs`, `tests/fixtures/journal/tourism_session.log`; Modify `missions/mod.rs` (`mod tourism;`). La migración `pub const PASSENGER_STATS_MIGRATION: Migration` (`missions_passenger_stats(mission_id PK, cmdr, accepted_at, reward, route_ly, completed_at)`) se define aquí y la devuelve `Missions::migrations()` en Task 28; los tests de esta tarea la aplican a mano sobre `Store::open_in_memory`.

**Interfaces:**
- Produces: `pub const EXACT_MAX_STOPS: usize = 8`; `fn optimal_order(start: [f64;3], stops: &[[f64;3]], return_to: Option<[f64;3]>) -> (Vec<usize>, f64)` (permutaciones exactas ≤ 8; por encima `heuristic_order`); `pub fn exact_order(..) -> (Vec<usize>, f64)` y `pub fn heuristic_order(..) -> (Vec<usize>, f64)` (vecino más cercano + 2-opt), misma firma, expuestas para test; `fn path_length(start, stops_in_order, return_to) -> f64`; `TourismPlan { mission_id, order: Vec<String>, board_order_ly: f64, optimal_ly: f64, jumps: Option<u32>, cr_per_ly: f64, cr_per_hour: Option<f64> /* ≈, constantes SECONDS_PER_JUMP=45, SECONDS_PER_STOP=240, verified false */, reference_cr_ly: f64, reference_is_own: bool, verdict: Verdict { Profitable, NotWorth }, can_abandon_now: bool /* sigue atracado donde aceptó */ }`; `fn own_reference(rows: &[(u64 /*reward*/, f64 /*ly*/)]) -> Option<f64>` = Σ recompensa / Σ al de las completadas (`None` sin datos → ajuste `tourism_reference_cr_ly`); `fn stats_write_task(cmdr, mission_id, accepted_at, reward, route_ly) -> StoreTask` (tag `missions.stats.write`), `fn stats_complete_task(cmdr, mission_id, at) -> StoreTask` (tag `missions.stats_complete.write`), `fn stats_read_task(cmdr) -> StoreTask` (tag `missions.passenger_stats`, lectura). Se calcula al aceptar y al redirigir (spec §10.8); sin paradas → `stops_unavailable`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn optimal_is_le_board_order() { /* inicio [0,0,0], paradas [10,0,0],[−10,0,0],[11,0,0] en ese orden → tablón 10+20+21=51, óptimo 0→−10→10→11 = 10+20+1 = 31, orden [1,0,2] */ }
#[test] fn return_trip_included_when_required() { /* 1 parada a 10 al con regreso → 20 */ }
#[test] fn exact_and_heuristic_agree_on_small_case() { /* 6 puntos de un LCG propio del test (semilla 7, sin dependencia rand) → heuristic_order ≥ exact_order y exact_order ≤ tablón */ }
#[test] fn nine_stops_use_heuristic_and_return_valid_permutation() { /* 9 puntos del LCG → orden es permutación de 0..9 y longitud == path_length del orden; sin aserción de tiempo */ }
#[test] fn cr_per_ly_and_verdict_vs_reference() { /* recompensa 3_200_000, 32 al → 100_000 cr/al; referencia 120_000 → NotWorth; 80_000 → Profitable */ }
#[test] fn own_reference_is_ratio_of_sums() { /* (1_000_000, 10.0) y (500_000, 10.0) → 75_000 */ }
#[test] fn no_stops_gives_unavailable() { /* stops vacío → plan None con estado stops_unavailable */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(missions): ruta óptima de turismo, créditos por año luz y veredicto`

### Task 28: `missions` — módulo, coordenadas, topics, relevancia y registro

**Files:** Create `crates/tmc-modules/src/missions/{coords.rs, tests.rs}`; Modify `missions/mod.rs`, `crates/tmc-modules/src/{lib.rs, registry.rs}`.

**Interfaces:**
- Produces: `pub struct Missions` (`Missions::new()`, `id() = "missions"`); `CoordResolver { fn need(&mut self, names: &[String]) -> Vec<String>; fn on_local(&mut self, map: HashMap<String,[f64;3]>); fn on_remote(&mut self, c: &SystemCoords); fn get(&self, name) -> Option<[f64;3]> }` (primero `store.coords_by_names` + `FSDJump`/`Location` del historial — tag `missions.coords` —, luego `SystemCoordsByName` por nombre que falte, máx. 10 en vuelo, `Inflight`).
  - Topics: `missions.active` (grupos por destino ordenados por distancia; «siguiente parada» = destino más cercano, empate → caduca antes; sin posición o coordenadas → por caducidad; spec §10.6), `missions.massacre`, `missions.alerts`, `missions.rewards`, `missions.passengers`, `missions.tourism`, `missions.community_goals`; todos con `as_of`.
  - Acciones: `missions.to_route { system }` (→ planificador de F1 / portapapeles; verificar en código al ejecutar la acción de `navigation`), `missions.dismiss_alert { mission_id }`, `missions.reset_session`.
  - `Missions::migrations()` devuelve `[PASSENGER_STATS_MIGRATION]` (Task 27).
  - Reconstrucción encadenada al arrancar/`LoadGame`/`Missions`: (1) `accepted_rebuild_task` (tag `missions.accepted`); (2) **solo al aplicar ese `Stored`** (comandante comprobado), `bounty_rebuild_task(cmdr, since = min(accepted_at) de las misiones de masacre con fecha)` (tag `missions.bounties`, lectura, paginando con `history::for_each_after`) cuyas bajas pasan por `MassacreStack::apply_event`; (3) `stats_read_task`. Todo con `check_cmdr`; ninguna `*.write` en replay.
  - Avisos: `alert_from_change` sobre cada `BookChange` + `evaluate_alerts` en cada evento; `ctx.alert` solo en vivo.
  - `relevance`: `Active` si hay misiones activas y `mode` es atracado o supercrucero; si no, `Normal`.

- [ ] **Step 1: Tests:**

```rust
#[test] fn missions_fixture_matches_missions_event_after_restart() { /* missions_session.log reproducido en dos mitades con "reinicio" → ids activos == Missions.Active del fixture, agrupados por destino (criterio 6) */ }
#[test] fn massacre_rebuilt_after_restart_matches_live() { /* massacre_session.log en vivo vs reconstruido desde store → mismas barras */ }
#[test] fn bounty_rebuild_only_after_accepted_result() { /* arranque → solo store_task missions.accepted; tras su Stored → aparece missions.bounties con since == aceptación más antigua */ }
#[test] fn next_stop_is_nearest_then_earliest_expiry() { /* destinos a 5 y 5 al, caducan 14:00 y 13:00 → siguiente la de 13:00 */ }
#[test] fn groups_by_expiry_without_coords() { /* sin coordenadas → orden por caducidad */ }
#[test] fn tourism_computed_on_accept_with_fake_coords() { /* tourism_session.log + Coords falsos → missions.tourism optimal_ly == <valor>, cr_per_ly == <valor> (criterio 8) */ }
#[test] fn replay_emits_no_requests_or_store_writes() { /* replaying con un MissionAccepted de pasajeros → 0 NetRequest y 0 tags "*.write" */ }
#[test] fn no_coord_requests_in_legacy_or_offline() { /* Legacy/offline → 0 NetRequest; turismo "no_coords" */ }
#[test] fn commander_switch_clears_missions() { /* Commander "B" → missions.active vacío */ }
#[test] fn stored_result_for_other_cmdr_is_ignored() { /* Stored missions.accepted con cmdr "OTRO" → sin cambios */ }
#[test] fn missions_topics_stay_under_16_kib() { /* 20 misiones (límite del juego) + 5 pilas + 10 objetivos → cada topic < 16384 */ }
#[test] fn relevance_active_with_missions_docked_or_supercruise() { /* 1 misión + Docked → Active; en vuelo normal → Normal */ }
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; Checks. **Step 5: Commit** `feat(missions): módulo de misiones con coordenadas, topics, relevancia y registro`

### Task 29: UI — «Misiones activas», «Masacre» y «Avisos»

**Files:** Create `ui/src/modules/missions/{index.ts, types.ts, MissionsPanel.svelte, MassacrePanel.svelte, MissionAlerts.svelte}`, `ui/tests/missions.test.ts`; Modify `ui/src/modules/registry.ts`, `locales/*/missions.ftl`.

**Interfaces:** grupos por destino (sistema · estación, distancia, siguiente parada destacada, botón «a la ruta» → `missions.to_route`); por misión: tipo, facción, progreso, cuenta atrás anclada a `core.clock` (`setInterval` 1 s solo con misiones con caducidad), recompensa; `fn isExpiringSoon(expiry, now)` en TS para mostrar el aviso aunque no lleguen eventos; masacre: barras por emisor y «faltan N bajas», recompensa total; avisos con descarte.

- [ ] **Step 1: Tests:**

```ts
test('groups by destination with next stop highlighted', /* 2 grupos, next "Lave" → primera tarjeta con clase next */);
test('countdown uses anchored clock', /* expiry clock+30 min → "30:00" y aviso expiring-soon */);
test('massacre bars and remaining', /* A 10/10, B 8/8, A' 2/5 → "faltan 3" */);
test('to route posts json action', /* POST missions.to_route {system:"Lave"} */);
test('no interval without expiring missions', /* sin caducidades → 0 setInterval activos */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): paneles de misiones activas, masacre y avisos`

### Task 30: UI — «Pasajeros», «Turismo», «Recompensas» y «Objetivos comunitarios»

**Files:** Create `ui/src/modules/missions/{PassengersPanel.svelte, TourismPanel.svelte, RewardsPanel.svelte, CommunityGoalsPanel.svelte}`; Modify `index.ts`, `locales/*/missions.ftl`, `ui/tests/missions.test.ts`.

**Interfaces:** pasajeros (cabinas por clase ocupadas/totales o agregado, grupos por destino con VIP/BUSCADO, aviso visible de buscado); turismo (orden óptimo vs tablón, al totales, saltos, cr/al, cr/h ≈, veredicto grande, «puedes abandonarla ahora» si `NotWorth && can_abandon_now`, «paradas no disponibles en el diario»); recompensas (créditos, completadas/fallidas, materiales, reputación/influencia por facción); objetivos (aportación, percentil, nivel, tiempo restante, recompensa prevista).

- [ ] **Step 1: Tests:**

```ts
test('wanted passenger shows warning', /* wanted true → missions-wanted-warning visible */);
test('tourism not worth shows abandon hint', /* NotWorth + can_abandon_now → missions-tourism-abandon */);
test('tourism shows both distances and crly', /* tablón 51, óptimo 32, 100000 → "51 al", "32 al", "100.000" */);
test('stops unavailable message', /* status stops_unavailable → missions-tourism-no-stops */);
test('community goal remaining time', /* expiry clock+2 d → "2 d" */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): paneles de pasajeros, turismo, recompensas y objetivos comunitarios`

---

## Bloque 4 — Disposiciones, ajustes, fixtures, eval y cierre

### Task 31: UI — disposiciones «Comercio» y «Misiones», prioridad y ajustes

**Files:** Modify `ui/src/shell/layouts.ts`, `ui/src/settings/Settings.svelte`, `locales/*/main.ftl`, `ui/tests/layouts.test.ts`.

**Interfaces:** `layouts` + `{ id: 'trade', titleKey: 'layout-trade', icon: 'scale', slots: [market 2, cargo 1, trade-session 1, route full, search 2] }` y `{ id: 'missions', titleKey: 'layout-missions', icon: 'clipboard', slots: [missions 2, mission-alerts 1, massacre 1 when massacreVisible, passengers 1 when passengersVisible, tourism 2 when tourismVisible, rewards 1, community-goals 1 when cgVisible] }`; `autoLayout(mods, manual): 'exploration' | 'combat' | 'mining' | 'missions' | 'trade' | null` con prioridad manual > `combat` Active/Urgent > `mining` Active > `missions` Active > `trade` Active > exploración; `nextPin` sin cambios (regla de F2). Visibilidad en `layouts.ts`, desde los topics: `massacreVisible` = `missions.massacre` no vacío; `passengersVisible` = `missions.passengers.seats_total > 0` o manifiesto no vacío; `tourismVisible` = `missions.tourism` con algún plan o estado; `cgVisible` = `missions.community_goals.goals` no vacío. Ajustes: radio EDDN (20–500) y referencia de turismo (cr/al) → `PUT /api/config` JSON; error revierte.

- [ ] **Step 1: Tests:**

```ts
test('missions beats trade', /* missions active + trade active → 'missions' */);
test('mining beats missions', /* mining active + missions active → 'mining' */);
test('combat beats everything automatic', /* combat urgent + mining/missions/trade active → 'combat' */);
test('trade beats exploration', /* trade active + exploration active → 'trade' */);
test('manual pin lasts until auto changes', /* manual 'trade', auto 'missions' estable → 'trade'; auto pasa a 'combat' → 'combat' */);
test('settings put eddn radius and reverts on 400', /* 600 → 400 → valor anterior */);
```

- [ ] **Step 2–4:** FAIL → implementar → PASS; checks. **Step 5: Commit** `feat(ui): disposiciones de comercio y misiones con prioridad y ajustes de F3`

### Task 32: fixtures sintéticos y pruebas de extremo a extremo

**Files:** Modify/complete (creados en Tasks 11, 15, 16, 23, 24, 27) `tests/fixtures/journal/{trade_session.log, missions_session.log, massacre_session.log, tourism_session.log}`, `tests/fixtures/companion/market_trade.json`, `tests/fixtures/eddn/burst.jsonl`; e2e en `ui/tests/e2e/` y `crates/tmc-app/src/e2e_tests.rs`. Si completar un fixture cambia un `<valor>` de un test anterior, se actualiza en este commit.

**Interfaces:** fixtures a mano, `CMDR TESTER`, sistemas/estaciones/facciones ficticios, sin FID real. `trade_session.log`: `LoadGame`, `Docked` (pads L), `MarketBuy` ×2 de gold, `Cargo` con 10 t de misión y 3 robadas, salto, `Docked`, `MarketSell` (sesión ±0). `massacre_session.log`: 5 misiones de 3 emisores contra la misma facción + 1 contra otra, `Bounty` antes y después de aceptar, un `MissionRedirected` llegado con el recuento local completo (10/10; el caso corto 7/10 vive solo en el test unitario de Task 24). `missions_session.log`: entregas con `CargoDepot`, una fallida, una redirigida, una que caduca < 1 h, `Missions` tras «reinicio». `tourism_session.log`: turismo con 4 paradas. `burst.jsonl`: 500 `commodity/3` cercanos + 20 corruptos.

- [ ] **Step 1: Tests:**

```rust
#[test] fn every_line_of_f3_fixtures_is_typed() { /* 0 errores; eventos en Other == EXPECTED_OTHER explícito */ }
#[test] fn trade_fixture_cargo_avg_and_profit() { /* criterio 3: avg gold == <valor>, est_profit == <valor>, misión y robada fuera */ }
#[test] fn trade_session_profit_matches_fixture() { /* criterio 5: Σ MarketSell − coste medio == <valor> (±0) */ }
#[test] fn burst_of_500_eddn_markets_keeps_10hz_and_one_requery() { /* burst.jsonl por el relay falso → ≤ 10 difusiones/s, p95 < 500 ms, ≤ 1 consulta trade.best_sells por 10 s, 20 descartes Corrupt */ }
```
  más e2e Playwright (headless de depuración con Spansh/Ardent falsos vía la sustitución de F2 y `TMC_E2E_EDDN_RELAY` con `cargo run -p tmc-eddn --example fake_relay`): `trade_current_market_top5_excludes_49h_sale` (criterio 2), `trade_route_advances_and_copies_next_system` (criterio 4), `missions_grouped_with_alerts` (criterio 6), `massacre_counts_match_manual` (criterio 7), `tourism_verdict_on_accept` (criterio 8), `eddn_markets_appear_and_corrupt_ignored` y `eddn_not_connected_when_offline` (criterio 9; con servicios desactivados `trade.market.eddn_connected` nunca pasa a `true`, porque el suscriptor no llega a emitir `Status{connected:true}`).
- [ ] **Step 2:** revisar fixtures (sin datos reales). **Step 3–4:** PASS; checks.
- [ ] **Step 5: Commit** `test: fixtures sintéticos de comercio, misiones y EDDN con pruebas de extremo a extremo`

### Task 33: `tmc-headless eval` — conciliación de coste pagado y de masacre

**Files:** Modify `apps/headless/src/eval.rs`, `crates/tmc-modules/src/eval.rs`.

**Interfaces:** informes agregados (sin nombres): (a) `CostBasis` frente a `MarketSell.AvgPricePaid` del juego (n ventas, diferencia media y máx.); (b) masacre: misiones redirigidas con su recuento local en el momento de `MissionRedirected` (n, % que cuadraba). Reutilizan `CostBasis::apply_event`, `MissionBook::apply_event` y `MassacreStack::apply_event`. Veredicto `INSUFICIENTE` si n < 5 (como F2).

- [ ] **Step 1: Tests:** `eval_cost_basis_matches_avg_price_paid_on_fixture` (diferencia 0 en `trade_session.log`); `eval_massacre_counts_match_on_fixture` (`massacre_session.log`: todas las redirigidas cuadran, 1/1 = 100 %).
- [ ] **Step 2–4:** FAIL → implementar → PASS; checks; ejecutar sobre el historial del propietario y anotar solo agregados en «Verificación F3».
- [ ] **Step 5: Commit** `feat(headless): eval de coste pagado y de pilas de masacre`

### Task 34: avisos, consumo con EDDN, verificación de los 11 criterios y cierre

**Files:** Modify `THIRD_PARTY_NOTICES.md` (completar: EDDN como servicio de recepción, Spansh trade y Ardent por red, `rare_goods.json`/`cabins.json` propios; verificar que `flate2` y ZeroMQ ya están desde Tasks 10–11), este plan («Verificación F3», «Desviaciones»), `status.md`.

- [ ] **Step 1:** Checks completos de Rust y UI + e2e (criterio 1) con salida.
- [ ] **Step 2:** `pwsh scripts/measure-footprint.ps1` con EDDN conectado al relay real (modo de Task 0), disposiciones «Comercio» y «Misiones» visibles durante el replay de los fixtures y en reposo (criterio 10).
- [ ] **Step 3:** `git diff main -- Cargo.lock ui/package.json` y licencias de cada paquete nuevo (criterio 11); evidencia por criterio en «Verificación F3»; revisión completa de la rama contra la spec.
- [ ] **Step 4: Commit** `docs: verificación de criterios y cierre de F3`

---

## Auto-revisión (cobertura de la spec)

| Spec | Tareas |
|---|---|
| §2/§3.1 disposiciones y prioridad | 19, 28, 31 |
| §3.2 mercado actual (48 h, top 5, rara, plataforma) | 6, 13, 16, 19, 21 |
| §3.2 mi carga (media ponderada, misión/robada, bodega) | 4, 15, 19, 21 |
| §3.2 ruta comercial (Spansh, avance, portapapeles, recalcular) | 1, 7, 8, 17, 19, 22 |
| §3.2 sesión de comercio y rango | 18, 19, 22 |
| §3.2 buscar mercancía | 6, 9, 18, 19, 22 |
| §3.3 misiones activas, agrupación, siguiente parada, «a la ruta» | 2, 5, 23, 28, 29 |
| §3.3 masacre (§10.10) | 2, 24, 28, 29, 33 |
| §3.3 avisos · recompensas · objetivos comunitarios | 25, 28, 29, 30 |
| §3.3 pasajeros y cabinas | 13, 26, 30 |
| §3.3 turismo (§10.3, §10.7–9) | 2, 5, 8, 27, 28, 30 |
| §4 `tmc-eddn`, tabla de precios, tipado, adaptadores, datos propios | 4–13 |
| §4 reutilización `PriceBook`/`rank_markets`/`ship_pads`/`core.clock` | 16, 20, 29 |
| §5 servicios, opt-out, Legacy, licencias | 0, 1, 8, 9, 12, 19, 28, 34 |
| §6 rendimiento y robustez EDDN | 0, 10, 11, 12, 19, 32, 34 |
| §7 i18n | 14, 21, 22, 29, 30, 31 |
| §8 criterios 1–11 | 32, 33, 34 (tabla abajo) |
| §10 decisiones 1–13 | 2→10.7; 6→10.2/10.11; 12→10.5/10.13; 16→10.2/10.11; 18→10.4; 24→10.10; 27→10.3/10.8/10.9; 28→10.6; 0/11→10.12 |

**Consistencia de tipos revisada:** `MarketSnapshot`/`PricedMarket`/`StationInfo`/`CommodityPrice` (Task 6) los usan Tasks 10–12, 16, 18–20; `TradeRouteParams`/`TradeRoutePlan`/`TradeStop` (7) en 8, 17, 19; `RemoteExport` (7) en 9, 18, 19; `SystemCoords`/`SystemCoordsByName` (7) en 8, 28; `EddnFilter`/`EddnMode`/`FrameSource` (10–11) en 12; `SellOption`/`rank_markets` (F2) en 15, 16, 18, 20; `MissionBook`/`Mission` (23) en 24–28; `CabinClass` (13) en 26; `ModuleSettings.{eddn_radius_ly, tourism_reference_cr_ly}` (7) en 12, 27, 31.

## Verificación F3

| # | Criterio (spec §8) | Evidencia (comando + salida) | Estado |
|---|---|---|---|
| 1 | Checks Rust y UI + e2e en verde | Task 34 Step 1 | ⬜ |
| 2 | «Mercado actual»: top 5, venta de 49 h fuera, plataforma | `trade::market::tests::*`, `trade_fixture_current_market_top5`, e2e `trade_current_market_top5_excludes_49h_sale` | ⬜ |
| 3 | «Mi carga»: media pagada y beneficio; misión/robada fuera | `trade_fixture_cargo_avg_and_profit`, `cost::tests::*` | ⬜ |
| 4 | Ruta con Spansh falso, avance y portapapeles | `route::tests::*`, `route_advance_copies_next_system`, e2e `trade_route_advances_and_copies_next_system` | ⬜ |
| 5 | Sesión: beneficio = Σ venta − coste medio (±0) | `trade_session_profit_matches_fixture`, `eval_cost_basis_matches_avg_price_paid_on_fixture` | ⬜ |
| 6 | Misiones tras reiniciar = `Missions`, agrupadas; avisos | `missions_fixture_matches_missions_event_after_restart`, `alerts::tests::*`, e2e `missions_grouped_with_alerts` | ⬜ |
| 7 | Masacre 5 misiones / 3 facciones = cálculo manual | `fixture_five_missions_three_factions_matches_manual`, e2e `massacre_counts_match_manual` | ⬜ |
| 8 | Turismo: óptima ≤ tablón y cr/al = manual | `tourism::tests::*`, `tourism_computed_on_accept_with_fake_coords`, e2e `tourism_verdict_on_accept` | ⬜ |
| 9 | EDDN con relay falso; corruptos; sin conexión offline | Tasks 10–12 tests, e2e `eddn_markets_appear_and_corrupt_ignored`, `eddn_not_connected_when_offline` | ⬜ |
| 10 | Footprint < 150 MB y < 1 % CPU con EDDN activo | Task 34 Step 2 | ⬜ |
| 11 | 5 idiomas; avisos y procedencia; nada GPL/AGPL incorporado | tests de `tmc-i18n`, Task 13, Task 34 Step 3 | ⬜ |

## Desviaciones

- 2026-10-19 · Task 15: `avg_paid` redondea al crédito más próximo; la media interna se mantiene sin redondeos intermedios. Carga parcialmente robada se divide en líneas Trade/Stolen sin duplicar used; Cargo de SRV no sustituye bodega de nave. `cost_rebuild_task` recibe `since = reloj − 7 días` del llamador, usa el paginador compartido y devuelve `{cmdr,basis,cut,cut_pos}` para conciliación de eventos llegados durante la lectura (Task 19). 13 tests específicos PASS, workspace 893 PASS (2 ignorados), fmt/clippy OK; spec/calidad aprobadas.

- 2026-10-19 · Task 13: catálogo inicial explícitamente parcial: Lavian Brandy y las cuatro cabinas de referencia del plan, con source propia y verified:false; símbolos desconocidos devuelven None. No se extrapolan datos sin comprobar ni se importan tablas incompatibles. Parser por fila conserva entradas válidas y avisa de las defectuosas. TDD: 4 fallos de comportamiento → 12 tests de datos PASS; workspace 878 PASS (2 ignorados), fmt/clippy OK; revisiones spec/calidad aprobadas.

- 2026-10-19 · Task 12: el escritor de coordenadas propias no espera `ImportProgress` final; fallo de importación o pérdida de ese mensaje no impiden guardar saltos posteriores. Tests de ambas rutas con SQLite real y servicios desactivados; revisiones spec/calidad aprobadas. Verificación: fmt/clippy y 874 tests Rust PASS (2 ignorados), 144 UI PASS, Svelte check/build OK.

- 2026-10-19 · Task 12 (revisión): `core.game` expone `replaying` persistente y el filtro EDDN exige `false` explícito. `game_running` no sirve como guardia de replay: `Fileheader`/`LoadGame` reproducidos también lo activan. La señal solo cambia con eventos de diario; el valor inicial bloquea conexiones. La siembra de coordenadas propias se repite al terminar la importación histórica porque puede finalizar después del arranque del servicio; el índice EDDN se siembra una sola vez según el contrato y depende después de `journal/1` para sistemas no conocidos.

(Cambios respecto a este plan o a la spec, con fecha, motivo y tareas afectadas.)

Ambigüedades de la spec resueltas por el redactor (2026-10-19; se someten al propietario en Task 3):

- **`commodity/3` no trae coordenadas ni plataformas.** El filtro por distancia usa coordenadas de `journal/1` del mismo flujo, del historial propio y de la tabla `system_coords`; sin coordenadas el mercado se descarta y se cuenta. Las plataformas salen de `journal/1` `Docked`, del `Docked` propio y de Ardent (Tasks 6, 10, 12).
- **EDDN en Legacy:** no se conecta (sus precios son de Live), igual que el resto de servicios.
- **Radio de «mejor venta»:** el de F2 (`sell_search_radius_ly`, 100 al); el radio de EDDN (200 al) solo limita lo que se guarda.
- **«Tu media» de pasajeros:** Σ recompensas / Σ años luz de las completadas (media ponderada), no la media de cocientes.
- **Ocupación por clase de cabina:** si el diario no dice la clase de cada pasajero (spike), se muestran plazas por clase y ocupación total agregada.
- **Aviso «caduca en < 1 h» sin eventos:** la UI lo calcula con `core.clock` anclado; el aviso del sistema (`ctx.alert`) sale en el siguiente evento.
- **Ruta comercial:** requiere estar atracado (Spansh necesita estación de origen); si no, «atraca para calcular».
- **«VENDER AQUÍ»:** si el precio local ≥ 98 % de la mejor venta cercana compatible.
- **Turismo, cr/h:** estimación con 45 s por salto y 240 s por parada (constantes `verified: false`, marcadas «≈»).
- **Coordenadas de destinos:** `NetQuery::SystemCoordsByName` nuevo, porque `NetQuery::System` exige `id64` y `RemoteSystem` no trae coordenadas.

Resoluciones de la revisión previa (2026-10-19) que cambian comportamiento visible frente a la spec:

- **Ardent en la tabla de precios (§4):** solo se persisten las filas de Ardent cuyo sistema tiene coordenadas conocidas (`system_coords`); el resto se usa solo en memoria para el sistema actual (Task 19).
- **«Mercado actual» con menos de 5 ventas conocidas:** se completan las 5 filas con compras sin venta conocida («sin datos»), ordenadas por precio de compra (Task 16).
- **Mismo mercado en varias fuentes:** gana el dato más nuevo aunque sea más barato (Tasks 16 y 20).
- **Masacre con misión sin detalles** (`MissionAccepted` fuera del historial): no recibe bajas y se muestra como «detalles no disponibles»; las bajas van a la siguiente misión del emisor (Task 24).
- 2026-10-19 · Task 3 (controlador, tras los spikes): **EDDN `OnDemand`** (CPU con filtro 0,37–0,39 % de un núcleo > 0,3 %): conecta con «Comercio»/«Minería» activas y desconecta 60 s después; prefiltro por `$schemaRef` antes de parsear. Biblioteca `zeromq` 0.6 (MIT) + `flate2` (MIT/Apache-2.0). Carriers: `Docked.StationType == "FleetCarrier"` recordado por `MarketID`, o `marketId ≥ 3 700 000 000` **y** nombre `^[A-Z0-9]{3}-[A-Z0-9]{3}$`. Cobertura de coordenadas en frío 91,8 % → sin siembra extra más allá de los eventos propios.
- 2026-10-19 · Task 3: **Spansh trade** solo admite `requires_large_pad` → se envía `(pad == Large)`; la respuesta no trae plataforma, así que `pad_ok` sale de `market_station` si se conoce (si no, aviso). Resultado = array de saltos → `parse_trade_result` propio; sondeo con timeout de 60 s. **Coordenadas por nombre:** Ardent `/v2/system/name/{name}` primaria, EDSM de respaldo. Ardent `nearby/exports` reutiliza `excluded()` de F2.
- 2026-10-19 · Task 3: **Misiones:** turismo verificado (`DestinationSystem` con tokens `$MISSIONUTIL_MULTIPLE_…;`, parseo por regex; regreso señalado por `MissionRedirected`); masacre con `KillCount`/`TargetFaction`/`TargetType`, `MASSACRE_COUNTS_KILL_BONDS = false`; ocupación de cabinas agregada (el diario no da clase por pasajero); `Name` con sufijo `_name` en `MissionCompleted` se normaliza; reputación/influencia como tokens. `MissionFailed/Abandoned`, `CargoDepot`, `Passengers`, `CommunityGoal` y `Missions` con elementos: solo documentación → tolerantes y con fixtures sintéticos.
