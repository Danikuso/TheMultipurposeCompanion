# Spike F17: sincronización de servicios (Inara / EDSM / EDDN)

Fecha: 2026-11-08. Alcance de esta versión de la nota: **solo la sección «Núcleo»** (T0-núcleo, local, sin red). Las secciones EDDN, EDSM e Inara se rellenarán en sus tareas T0.

## Núcleo

### 1. Semántica de `Journal.<fecha>.<part>.log` y de `Continued`

Fuentes:
- Manual del diario de Frontier v38 §2.1, §2.2, §13.6 (resumido en `local_game_data.md` líneas 15, 22, 24): se crea un archivo por lanzamiento; si la sesión es larga, el archivo se cierra y se abre otro con `part` incrementado y la cabecera repetida; `Continued` se escribe «si el diario crece hasta 500k líneas».
- Código local: `crates/tmc-journal/src/files.rs` (`is_journal_name`: `part` = 2 dígitos; fecha de 12 o 17 caracteres; `list_journals` ordena por longitud y luego nombre).
- Diarios del propietario (**solo agregados**, 47 archivos en `Saved Games`): `part` en el nombre = `01` en 47/47; `Fileheader.part` = 1 en 47/47; eventos `Continued`: 0; archivos terminados en `Shutdown`: 45/47 (los 2 restantes son sesiones sin cierre limpio); archivos con 0/1/2 `LoadGame`: 12/29/6 (hay relogueo a menú dentro de un mismo archivo); archivo más largo 3 796 líneas; `gameversion` 4.4.1.1 / 4.4.2.0 / 4.4.2.1.

Conclusiones:
1. Un lanzamiento del juego **siempre** abre un archivo con `part` = `01` (47/47). Por tanto `part > 1` solo puede deberse a un rollover de 500 000 líneas (continuación de la misma sesión).
2. Los diarios del propietario no contienen ningún rollover (máx. 3 796 líneas vs 500 000), así que **no se ha podido observar** el nombre real del archivo tras un `Continued` (si conserva la marca de fecha con `part+1` o estrena fecha). La regla se diseña para tolerar ambas formas y se cubre con fixtures sintéticos (`tests/fixtures/sync/core/`). Es la única hipótesis no verificada empíricamente; queda como riesgo bajo (el fallo seguro es tratar como sesiones distintas y no recuperar).
3. El evento `Continued` lleva el campo `Part` (número de la parte siguiente). Es la última línea del archivo cerrado.
4. Una sesión de menú (arranque y cierre sin jugar) genera archivos de ~221 bytes con `Fileheader` y `Shutdown` (`informe-investigacion.md`). No son continuación de nada (`part` = 01, sin `Continued` en el anterior).

### 2. Regla «misma sesión» (§5.4.1)

Decisión (no contradice la spec; confirma la hipótesis «Continued o `part > 1`», precisándola):

```
same_session(prev, cur) -> bool
  // prev y cur son rutas a archivos de la carpeta de diario; prev es el inmediatamente anterior según list_journals.
  1. nombres válidos (is_journal_name) ambos; si no -> false
  2. part(cur) <= 1  -> false por la vía del nombre
     part(cur) >= 2  -> true si part(prev) + 1 == part(cur)
  3. alternativamente true si la última línea completa y no vacía de prev es un evento `Continued`
  4. en cualquier otro caso -> false
```

**Estrechamiento declarado frente a spec §5.4.1:** la spec admite «`Continued` o `part > 1`»; esta regla exige además `part(prev)+1 == part(cur)` cuando se apoya en el nombre. Falla cerrado (ante duda, no se recupera); coste: alguna recuperación no hecha en rollovers raros (ruling T0-4, desviación registrada en el plan).

Es decir: `same_session = (part(cur) >= 2 && part(prev)+1 == part(cur)) || last_event(prev) == "Continued"`.

Justificación:
- `Continued` al final de `prev` es prueba directa (la escribe el juego al cerrar por rollover), válida aunque el nombre no fuera el esperado.
- `part(cur) >= 2` es suficiente en la práctica (1.), pero se exige además `part(prev)+1 == part(cur)` para que el «inmediatamente anterior» sea realmente la parte previa y no un archivo de otro lanzamiento intercalado.
- Un `prev` con `Shutdown` al final y `cur` con `part` = 01 es siempre sesión distinta (caso `other_session.01.log`).
- La comprobación del `Continued` lee solo la cola del archivo anterior (últimos KiB); no hay red ni análisis completo.
- Nunca se mira más allá del inmediatamente anterior (spec: «Nunca archivos más antiguos»). Una cadena `01 -> 02 -> 03` se resuelve aplicando la regla de forma encadenada solo si la spec lo pidiera; el alcance actual es **un** archivo hacia atrás.

Fixtures y resultado esperado:
- `same_session(session_a.01, session_a.02) == true` (`Continued` y `part` 1 -> 2).
- `same_session(session_a.02, other_session.01) == false`.
- `same_session(other_session.01, legacy.01) == false`.

Nota de integración: la galaxia (Legacy `3.8`), beta y comandante se determinan por archivo (§5.4.5), no por la regla de sesión.

### 3. Lectura del watcher (`crates/tmc-journal/src/watcher.rs`) y del actor

Confirmado leyendo el código:
- `JournalReader::startup(dir)` (l. 124): abre **solo** `paths.last()` (el más reciente), lo lee con `drain(true, ..)` -> todos sus eventos salen con `replaying = true`.
- Anteposición de diario antiguo (l. 142-181): solo si el más reciente **no** contiene `Commander` y hay >= 2 archivos; recorre hacia atrás hasta un archivo que contenga `Commander` (`file_contains_commander`), lo lee con `replaying = true` y **antepone** sus eventos a los del vigente. Esta anteposición **no** aplica ninguna regla de sesión (puede ser un diario de otro día). Para la sincronización, el router debe decidir por `file`/cursor/§5.4, nunca por «`replaying = true`» a secas. Posible solapamiento con `Recovery` (que lee el anterior misma-sesión): el cursor evita duplicados.
- Tras el arranque, `poll(scan, false)` lee el archivo actual con `replaying = false`. Si aparece un archivo con `journal_key` mayor (`poll`, l. 215-232), se abre y **todo su contenido** sale con `replaying = false`. Esto implica que cualquier archivo que aparezca durante la ejecución (por ejemplo una copia manual o `tmc-replay`) genera líneas `Live`, no `CatchUp`; el marcador `.tmc-replay` y las guardas §5.4.6 son imprescindibles.
- Orden de emisión (`emit_journals`, l. 242-300): por cada evento `SourceEvent::Journal` se envía primero el `Journal` y, si `CompanionKind::for_event` aplica, **después** `SourceEvent::Companion` (orden `Journal -> Companion`). El canal es un único `mpsc` ordenado.
- Orden del arranque en `run` (l. 439-): `startup` -> `emit_journals(events)` -> `Theme` -> `Status` inicial -> se lanzan `poll_status` y `poll_process` (primer `GameProcess` al primer tick) -> bucle de sondeo.
- En el actor (`tmc-core/src/actor.rs` l. 445-475), `self.replaying` se fija por evento `Journal`; el `StoredEvent` enviado al writer lleva `line` (1-based) y `file_name`.
- La importación histórica (`crates/tmc-store/src/import.rs`) usa `tmc_journal::{LineTailer, list_journals}` directamente (l. 7, 39, 81) con offsets en la tabla `journal_file`: **no pasa por el actor** ni por el router, por lo que nunca puede generar envíos.

### 4. Secuencia de arranque que define `CatchUpDone` (PD-1)

Problema: `replaying = true` marca el final del arranque solo cuando llega la primera línea `replaying = false`, que puede no llegar nunca (juego cerrado, archivo estático). Hace falta un marcador explícito.

Decisión: añadir `SourceEvent::CatchUpDone`, emitido por el watcher **inmediatamente después** de `emit_journals(events, ..)` del arranque (l. 454) y **antes** de `Theme`. Garantías:
- Va por el mismo `mpsc` ordenado: se recibe tras todos los `Journal`/`Companion` del arranque (incluidos los prependidos) y antes de cualquier línea `Live`.
- Se emite **siempre**, incluso sin diarios o si el primero falla al abrir (eventos vacíos).
- El `Recovery` del anterior de la misma sesión debe ejecutarse **antes** de procesar las líneas del vigente (§5.4 «Lectura»); como el router solo ve el vigente a través del watcher, la forma segura es que el actor/router retenga las líneas `CatchUp` hasta tener el resultado del `Recovery` o que el `Recovery` se ejecute al construir el router (antes de que el actor consuma). **Fijado (ruling T0-5):** `Recovery` se ejecuta al construir el router, antes de procesar las líneas del archivo vigente. El marcador `CatchUpDone` solo delimita el final de lo recuperable.
- Tras `CatchUpDone` empieza el estado `Live`; la instantánea de Inara (§5.3) se genera tras `CatchUpDone` y después de `Recovery`.

Alternativa descartada: usar `Theme` como delimitador (acoplado a otro propósito; si en el futuro se omite en error, rompe la sincronización).

### 5. Base de numeración de líneas

`LineTailer::line_no` (`crates/tmc-journal/src/tail.rs`):
- Se incrementa **antes** de empujar la línea (`self.line_no += 1; lines.push((self.line_no, ..))`, l. 131-132): la primera línea del archivo es la **número 1** -> **1-based**.
- `line_no()` devuelve el número de la **última línea consumida**, o 0 si no ha consumido ninguna. `LineTailer::open(path, offset)` inicializa `line_no` contando los `\n` anteriores a `offset` (l. 57-66), por lo que `last_line` del cursor coincide con «líneas completas ya procesadas».
- `sync_cursor.last_line` se define como **número 1-based de la última línea procesada**; «posterior al cursor» = `line > last_line`; cursor ausente = 0. Es coherente con `StoredEvent.line`.
- Comportamiento ante truncado: si el archivo se acorta, `line_no` vuelve a 0 y se releen líneas ya vistas (`tail.rs` l. 99-110). El router debe tratar un archivo con tamaño inferior al cursor como reinicio y descartarlo (riesgo documentado: sin esta guarda se reenviarían líneas).

### 6. Detección del juego en vivo por plataforma (`LiveSignalPolicy`)

Windows: `ProcessCheck`. `tmc-journal/src/process.rs` busca el proceso `EliteDangerous64.exe` con `sysinfo` 0.39.6 (`Cargo.lock`). Verificado en esta máquina por el código y por el uso en producción. Con `TMC_E2E_GAME_RUNNING` solo en depuración.

Linux/Proton (decisión del controlador, ruling T0-1): **`LiveSignalPolicy::JournalActivity`** (en la spec: «Juego en vivo (Linux)», §5.2). La señal es: líneas `Live` nuevas en el archivo vigente **+ ausencia del marcador `.tmc-replay`** (lo creará E0-8, PD-9; hoy no existe en el código ni `tmc-replay` lo escribe) **+ el resto de guardas** (§5.4.6). Si el marcador o las guardas no se cumplen, el estado es `unavailable`.

Riesgo aceptado: una copia manual de un diario con TMC abierto se trataría como vivo, porque el watcher lee entero con `replaying = false` cualquier archivo nuevo que aparezca (`poll`, l. 215-232); igual un truncado (`line_no = 0`, relectura `Live`). El marcador `.tmc-replay` mitiga `tmc-replay`, no las copias manuales. Coste si falla: envío duplicado de datos reales (EDDN/EDSM mayormente idempotentes por el lado servidor).

Mejora futura / defecto latente de `is_game_running` (NO es política de F17): según `sysinfo-0.39.6/src/unix/linux/process.rs` (l. 579, 589, 1080-1100) `Process::name()` es el campo `comm` de `/proc/<pid>/stat`, que el kernel trunca a 15 caracteres. `EliteDangerous64(.exe)` (16) llegaría como `EliteDangerous6`, por lo que la comparación actual no coincide nunca en Linux. Una futura `ProcessCheck` en Linux debería comparar por prefijo `EliteDangerous6`; no se pudo verificar en una máquina Linux/Proton real (host Windows).

Enum resultante:

```
enum LiveSignalPolicy { ProcessCheck, JournalActivity, Unavailable }
// Windows -> ProcessCheck ("EliteDangerous64.exe")
// Linux   -> JournalActivity (Live nuevas + sin .tmc-replay + guardas); si no se cumple -> unavailable
// macOS / otros -> Unavailable
```

### 7. Resumen de decisiones

| Punto | Decisión |
|---|---|
| Misma sesión | `(part(cur) >= 2 && part(prev)+1 == part(cur)) \|\| última línea de prev == Continued` |
| Alcance de recuperación | vigente + inmediatamente anterior misma sesión; nunca más antiguos |
| Marcador `CatchUpDone` | `SourceEvent::CatchUpDone` tras `emit_journals` del arranque y antes de `Theme` |
| Base de líneas | 1-based; `last_line` = última línea procesada; cursor ausente = 0 |
| Importación histórica | no pasa por el actor; no genera envíos |
| Windows | `ProcessCheck` |
| Linux | `JournalActivity`: Live nuevas + sin `.tmc-replay` + guardas (ruling T0-1). `ProcessCheck` por prefijo `EliteDangerous6` = mejora futura |
| Hipótesis no verificada | nombre del archivo tras un `Continued` real (ausente en los diarios del propietario) |

## EDDN

### 1. Fuente consultada y licencia

- **Repositorio oficial:** `https://github.com/EDCD/EDDN`, rama `live`, commit **`4ad669bb7bbe1eae080e4c354e786dca4db91f35`**.
- **Documentación y código revisados:** `docs/Developers.md`, `schemas/README-EDDN-schemas.md`, `src/eddn/Gateway.py`, `src/eddn/Validator.py`, `src/eddn/conf/Settings.py` y los 16 esquemas JSON (`schemas/*-v*.json` + `*-README.md`), junto con la nota previa [`docs/research/notes/spike_eddn_f3.md`](spike_eddn_f3.md).
- **Licencia de `EDCD/EDDN`:** **BSD 3-Clause** (compatible con `PolyForm-Noncommercial-1.0.0`). Se conservan copias locales de solo los 16 esquemas Draft-04 vigentes y su licencia (`EDCD-LICENSE.txt`) en [`tests/fixtures/sync/eddn/_schemas/`](../../../tests/fixtures/sync/eddn/_schemas/) para validación offline en tests.

### 2. Protocolo del gateway y cabeceras

- **Endpoint:** `POST https://eddn.edcd.io:4430/upload/` (barra final obligatoria, HTTPS, **solo HTTP/1.1** `http1_only=true`, sin parámetros de consulta).
- **Cuerpo y compresión:** JSON UTF-8 (`Content-Type: application/json`). Cuando el JSON supera 16 KiB (`16384` bytes, decisión `[DR-17]`), se comprime con gzip (`Content-Encoding: gzip`); `Gateway.py` descomprime mediante `zlib.decompress(..., 15 + 32)`. Límite máximo comprimido: **`1 MiB` (`1048576` bytes)**. Un mensaje por petición.
- **Límite de tasa y edad máxima:** máximo **2 mensajes/s** en TMC (`RateLimiter::for_service(Eddn)`). El gateway no aplica corte duro de edad en el POST, aunque `Developers.md` advierte de que los listeners descartan marcas antiguas; TMC aplica `max_age = 24 h` (§5.4).
- **Envelope:**
  - `$schemaRef`: URL canónica sin `#` final (ej. `https://eddn.edcd.io/schemas/journal/1`), o con sufijo `/test` en `SchemaMode::Test` (`TMC_EDDN_TEST_SCHEMAS=1`). Todos los esquemas `https://eddn.edcd.io/schemas/...` son activos; `Settings.GATEWAY_OUTDATED_SCHEMAS` solo lista los antiguos `http://schemas.elite-markets.net/eddn/*`.
  - `header`: `uploaderID` (`hex(sha256(sal_local || FID))[..32]`, sin nombre ni FID en claro), `softwareName` (`"The Multipurpose Companion (TMC)"`), `softwareVersion` (`CARGO_PKG_VERSION`), `gameversion` (de `Fileheader.gameversion`, si falta de `LoadGame.gameversion`, si no `""`), `gamebuild` (de `Fileheader.build` o `LoadGame.build` preservando espacios finales como `"r310123/r0 "`, si no `""`). **Sin `gatewayTimestamp`**.
  - En todos los mensajes, `horizons` y `odyssey` proceden **exclusivamente de `LoadGame`** (nunca de `Fileheader`, que no tiene `Horizons`/`Odyssey` fiables) y se omiten si se desconocen (nunca `null`).

### 3. Respuestas del gateway (`Gateway.py`)

| Estado HTTP | Cuerpo / patrón típico | Acción en TMC |
|---|---|---|
| `200 OK` | `OK` | `Sent`. |
| `400 Bad Request` | `FAIL: Schema Validation: [...]` / `FAIL: JSON parsing: ...` / `FAIL: zlib.error: ...` | `Rejected` (`invalid_event`, sin reintento; pista saneada `<= 120` chars). |
| `413 Payload Too Large` | Límite `MEMFILE_MAX` (`1 MiB`) | `Rejected` (`too_large`, sin reintento). |
| `426 Upgrade Required` | `FAIL: Outdated Schema: ...` | `SchemaOutdated`: pausa ese `$schemaRef` en `kv` (`sync.eddn.paused_schemas`) y muestra aviso hasta nueva versión de TMC. |
| `408`, `503`, otros `5xx`, timeout, error de red | HTML/texto de proxy o caída | `Retry` (`server_error` / `timeout` / `network`) con espera mínima de **60 s** y backoff exponencial hasta **30 min**. |

**Alineación con §7.3:** no se ha detectado ninguna contradicción entre `Gateway.py` y la spec §7.3 (ni respecto a `426`, cabeceras, HTTP/1.1 ni saneado).

### 4. Decisión de validador en tests

- En la tarea `E1-4` se añadirá la crate **`jsonschema`** (licencia **MIT**, compatible y anotada en `THIRD_PARTY_NOTICES.md`) en `[dev-dependencies]` de `tmc-sync` para validar offline Draft-04 contra los 16 esquemas BSD-3-Clause de `tests/fixtures/sync/eddn/_schemas/`, evitando tráfico de red y duplicación manual de reglas de esquema.

### 5. Tabla de saneado, aumentos y elisiones por esquema (16 esquemas)

Reglas transversales a todos los esquemas de diario: (a) eliminar recursivamente cualquier clave acabada en `_Localised`; (b) comprobar coherencia de ubicación (`StarSystem`, `SystemAddress`, `StarPos`) contra el último `Location` / `FSDJump` / `CarrierJump`: ante discrepancia, omitir la fila (`Inconsistent`); (c) barrido local recursivo final contra lista negra de campos personales (`Name`/`CMDR`/FID cuando proceda, `Credits`, `Loan`, `Wanted`, `ActiveFine`, `MyReputation`, chat, `*_Localised`).

| # | `$schemaRef` | Origen | Renombrados / Aumentos obligatorios | Campos eliminados / filtros |
|---|---|---|---|---|
| 1 | `journal/1` | `Docked`, `FSDJump`, `Scan`, `Location`, `SAASignalsFound`, `CarrierJump` | Añadir `StarSystem` (`SAASignalsFound`) y `StarPos` (`Docked`, `Scan`, `SAASignalsFound`) tras cross-check de `SystemAddress`; `horizons`, `odyssey`. | Quitar `ActiveFine`, `CockpitBreach`, `BoostUsed`, `FuelLevel`, `FuelUsed`, `JumpDist`, `Latitude`, `Longitude`, `Wanted`, `IsNewEntry`, `NewTraitsDiscovered`, `Traits`, `VoucherAmount`; dentro de `Factions[]`: `HappiestSystem`, `HomeSystem`, `MyReputation`, `SquadronFaction`; todo `_Localised`. |
| 2 | `commodity/3` | `Market` + `Market.json` | Renombrar `StarSystem->systemName`, `StationName->stationName`, `MarketID->marketId`, `CarrierDockingAccess->carrierDockingAccess`, `Items->commodities`; por item: `Name->name` sin prefijo `$` ni sufijo `_name;` (minúsculas), `MeanPrice->meanPrice`, `BuyPrice->buyPrice`, `Stock->stock`, `StockBracket->stockBracket`, `SellPrice->sellPrice`, `Demand->demand`, `DemandBracket->demandBracket`, `statusFlags` si no vacío; `horizons`, `odyssey`. | Quitar `StationType`, `Producer`, `Rare`, `id`, `Category`, `*_Localised`; no emitir `economies`/`prohibited` si faltan o vacíos; descartar artículos con `Category`/`categoryname == "NonMarketable"` o `Legality` no vacía. |
| 3 | `shipyard/2` | `Shipyard` + `Shipyard.json` | Renombrar `StarSystem->systemName`, `StationName->stationName`, `MarketID->marketId`, `AllowCobraMkIV->allowCobraMkIV`, `PriceList[*].ShipType -> ships` (`uniqueItems`, `minItems: 1`); `horizons`, `odyssey`. | Quitar precios, `ShipPrice`, `*_Localised`; no emitir si `ships` queda vacío. |
| 4 | `outfitting/2` | `Outfitting` + `Outfitting.json` | Renombrar `StarSystem->systemName`, `StationName->stationName`, `MarketID->marketId`, `Items[*].Name -> modules` (`uniqueItems`, `minItems: 1`); `horizons`, `odyssey`. | Conservar solo nombres `^(?i)(Hpt_|Int_|.*_Armour_)` distintos de `Int_PlanetApproachSuite` y con `sku` ausente/null o `"ELITE_HORIZONS_V_PLANETARY_LANDINGS"`; omitir mensaje si `modules` queda vacío. |
| 5 | `navroute/1` | `NavRoute` + `NavRoute.json` | `Route[*]` conserva `{ StarSystem, SystemAddress, StarPos, StarClass }` (`additionalProperties: false`); `horizons`, `odyssey`. | Si `NavRouteClear` o `Route` vacío → no emitir. |
| 6 | `fcmaterials_journal/1` | `FCMaterials` + `FCMaterials.json` | Contenido de `FCMaterials.json` (`MarketID`, `CarrierName`, `CarrierID`, `Items[{id,Name,Price,Stock,Demand}]`); `horizons`, `odyssey`. | Quitar `_Localised`. |
| 7 | `fssdiscoveryscan/1` | `FSSDiscoveryScan` | Aumentar `StarPos` (+ cross-check `SystemAddress`/`SystemName`); `horizons`, `odyssey`. | Quitar `Progress` y claves ajenas al esquema (`additionalProperties: false`), `_Localised`. |
| 8 | `fssallbodiesfound/1` | `FSSAllBodiesFound` | Aumentar `StarPos` (+ cross-check `SystemAddress`/`SystemName`); `horizons`, `odyssey`. | Quitar `_Localised` y claves ajenas (`additionalProperties: false`). |
| 9 | `navbeaconscan/1` | `NavBeaconScan` | Aumentar `StarSystem` y `StarPos` (+ cross-check `SystemAddress`); `horizons`, `odyssey`. | Quitar `_Localised` y claves ajenas (`additionalProperties: false`). |
| 10 | `codexentry/1` | `CodexEntry` | Aumentar `StarPos` y `SystemAddress` si faltan (+ cross-check `System`); `horizons`, `odyssey`; `BodyID`/`BodyName` solo si confirmados por `ApproachBody`/`Location`. | Quitar `IsNewEntry`, `NewTraitsDiscovered`, `_Localised` y claves ajenas (`additionalProperties: false`). |
| 11 | `scanbarycentre/1` | `ScanBaryCentre` | Aumentar `StarPos` (+ cross-check `StarSystem`/`SystemAddress`); `horizons`, `odyssey`. | Conservar solo parámetros orbitales permitidos por el esquema; quitar `_Localised`. |
| 12 | `fssbodysignals/1` | `FSSBodySignals` | Aumentar `StarSystem` y `StarPos` (+ cross-check `SystemAddress`); `Signals[{Type,Count}]`; `horizons`, `odyssey`. | Quitar `Type_Localised` y claves ajenas (`additionalProperties: false`). |
| 13 | `approachsettlement/1` | `ApproachSettlement` | Aumentar `StarSystem` y `StarPos` (+ cross-check `SystemAddress`), conservar `MarketID` cuando exista; `horizons`, `odyssey`. | En `StationFaction` conservar solo `{Name, FactionState}`, en `StationEconomies` `{Name, Proportion}`; quitar `_Localised` y claves ajenas. |
| 14 | `dockinggranted/1` | `DockingGranted` | `timestamp`, `event`, `MarketID`, `StationName`, opcionales `StationType`, `LandingPad`; `horizons`, `odyssey`. | Quitar `_Localised` y claves ajenas (`additionalProperties: false`). |
| 15 | `dockingdenied/1` | `DockingDenied` | `timestamp`, `event`, `MarketID`, `StationName`, `Reason`, opcional `StationType`; `horizons`, `odyssey`. | Quitar `_Localised` y claves ajenas (`additionalProperties: false`). |
| 16 | `fsssignaldiscovered/1` | Racha contigua de `FSSSignalDiscovered` | Agrupar en `signals[]`; `timestamp` del mensaje = primero de la racha; aumentar `StarSystem` y `StarPos` usando la ubicación nueva si cierra con `Location`/`FSDJump`/`CarrierJump` (Odyssey) o la previa (Horizons); `horizons`, `odyssey`. | Descartar señal con `USSType == "$USS_Type_MissionTarget;"` o `SystemAddress` distinto; por señal quitar `event`, `SystemAddress`, `TimeRemaining`, `_Localised`; si `signals` queda vacío no emitir. |

## EDSM

Fecha de verificación: **2026-11-13**. Las páginas HTML oficiales en vivo (`https://www.edsm.net/en/api-journal-v1` y `https://www.edsm.net/en/api-commander-v1`) presentan desafío JS de Cloudflare para clientes sin navegador, por lo que se consultaron sus instantáneas archivadas de Wayback Machine (`2026-03-12` y `2026-03-25`, coincidiendo con las revisiones oficiales del `2022-11-30` y `2019-03-21`), contrastadas con [`community_apis_core.md`](community_apis_core.md) y una **única petición pública `GET https://www.edsm.net/api-journal-v1/discard`** (`200 OK`, `2267` bytes, **141 eventos**, capturada en [`tests/fixtures/sync/edsm/discard.json`](../../../tests/fixtures/sync/edsm/discard.json)). **Ningún POST enviado.**

### 1. Endpoints y petición (`api-journal-v1`)

- **Envío de diario:** `POST https://www.edsm.net/api-journal-v1` (cuerpo JSON o multipart; nunca `GET`).
- **Lista de descarte:** `GET https://www.edsm.net/api-journal-v1/discard` → array JSON de nombres de evento a omitir sin enviar (`141` entradas el `2026-11-13`; se cachea 24 h en `api_cache` y falla cerrado si nunca se obtuvo).
- **Verificación de clave/comandante:** `GET https://www.edsm.net/api-commander-v1/get-ranks?commanderName=...&apiKey=...`.
- **Campos del cuerpo JSON en POST:**
  - `commanderName` (obligatorio; nombre exacto registrado en EDSM, nunca el FID).
  - `apiKey` (obligatorio).
  - `fromSoftware` = `"The Multipurpose Companion (TMC)"` (`[DR-19]`).
  - `fromSoftwareVersion` = versión SemVer (`CARGO_PKG_VERSION`).
  - `fromGameVersion` y `fromGameBuild` (del `Fileheader`, preservando espacios finales como `"r310123/r0 "`; obligatorio para distinguir Live de Legacy — «ONLY THE LIVE VERSION IS ACCEPTED»).
  - `message`: línea del diario sin modificar **o lista (array) JSON de eventos para lotes**. La documentación oficial indica: *«In order to send messages by batch, you can also send a list of message line from the journal»*, adjuntando los seis campos transitorios por entrada mediante la lógica documentada `updateGameStatus(entry)`: `_systemAddress`, `_systemName`, `_systemCoordinates`, `_marketId`, `_stationName`, `_shipId` (como propiedades de cada objeto evento en `message[]`, `null` si se desconocen).
- **Tamaño máximo de lote:** la documentación no publica un límite fijo de elementos por POST ni contradice el máximo de **50 eventos** fijado en §7.2 / `BatchPolicy`; se mantiene `max_rows = 50` (o flush a los 15 s) y objetivo ≤ 1 petición cada 5 s bajo el limitador duro de Cloudflare (10 req / 10 s).

### 2. Formato de respuesta, códigos `msgnum`, tasa y Cloudflare

- **Forma JSON en lote:** objeto raíz `{ "msgnum": <int>, "msg": "<texto>", "events": [ { "msgnum": <int>, "msg": "<texto>", ... }, ... ] }` (en errores globales de autenticación/software/JSON puede omitirse `events`). Si `events.len()` no coincide con el número de eventos enviados en el lote, TMC reintenta todos sin marcar ninguno como enviado.
- **Tabla completa de `msgnum` (`api-journal-v1`):**
  - `100` (`OK`) → `Sent`.
  - `101` (`Message already stored`), `102` (`Message older than the stored one`), `103` (`Duplicate event request`, caché de ~300 s), `104` (`Crew session`) → `Sent` (benigno).
  - `201` (falta `commanderName`), `202` (falta `apiKey`), `203` (`Commander name/API Key not found`) → `CredentialError` (retiene pendientes hasta corregir clave).
  - `204` (`Software/Software version not found`), `205` (`Blacklisted software`) → `SoftwareBlocked` (detiene servicio sin reintentos).
  - `206` (`Cannot decode JSON`) → `Rejected` (`invalid_event`).
  - `207` (`Game/Build version not found`), `208` (`Game/Build version too old`) → `Rejected`.
  - `301` (`Message not found`), `302` (`Cannot decode message JSON`), `303` (`Missing timestamp/event`) → `Rejected`.
  - `304` (`Discarded event`) → `Discarded` + refresco inmediato de la lista de descarte.
  - `401` (`Category unknown`), `402` (`Item unknown`) → `Rejected`.
  - `451` (`System probably non existant`), `452` / `453` (`Same system just before/after visited date`) → `BenignRejected` (sin contar error de servicio).
  - `500`, `501` → `Retry` (`server_error`) con backoff.
- **Límites de tasa y Cloudflare:**
  - Límite Cloudflare (confirmado en `FrontEnd#563`, 2026-08-17): **10 peticiones / 10 s** (bloqueo temporal de 10 s si se supera). HTTP `429` respeta `Retry-After` en primer lugar y como respaldo `X-Rate-Limit-Reset` (además de leer `X-Rate-Limit-Remaining` cuando estén presentes).
  - HTTP `403` con HTML de Cloudflare («Attention Required!» / desafío) → `Retry` con clase `Blocked` y backoff largo (**≥ 15 min**).
  - Respuesta HTML inesperada con HTTP `200 OK` (p. ej. mantenimiento o proxy) → **nunca** se considera `Sent`; clasificada como `Retry` (`Unknown`).

### 3. Contrato `GET /api-commander-v1/get-ranks` (verificación de clave)

- **Respuesta `msgnum: 100`:** devuelve `ranks`, `progress` y `ranksVerbose` (`Combat`, `Trade`, `Explore`, `CQC`, `Federation`, `Empire`) → `KeyCheck::Valid`.
- **Respuesta `msgnum: 207` (`No ranks stored`):** comandante/clave válidos cuando el perfil aún no tiene rangos registrados y se envió `apiKey` → `KeyCheck::Valid`.
- **Respuestas `msgnum: 201 | 202 | 203`:** falta nombre/clave o par inválido en EDSM → `KeyCheck::Rejected`.
- **Cualquier otro código HTTP/red/HTML/403/429:** → `KeyCheck::Unreachable`.

**Alineación con §7.2:** 100 % alineado sin bloqueos ni desviaciones necesarias respecto a la spec §7.2.

### 4. Inventario de fixtures EDSM (`tests/fixtures/sync/edsm/`)

- `discard.json`: instantánea real capturada de `GET /api-journal-v1/discard` (141 eventos, `2026-11-13`).
- `request_batch.json`: ejemplo normalizado de lote POST con `fromSoftware` exacto y los 6 campos transitorios en el evento `FSDJump`.
- `response_ok.json`, `response_mixed.json`: respuestas JSON con `msgnum` global + `events[]`.
- `response_html_403.html`, `response_html_200.html`: páginas HTML defensivas (Cloudflare / mantenimiento).
- `get_ranks_ok.json`, `get_ranks_bad_key.json`: respuestas de verificación de clave.

## Inara

> Investigación realizada el **2026-11-13** leyendo en caché pública de solo lectura (Wayback Machine, capturas `20260308173432id_` y `20260308173440id_` de <https://inara.cz/elite/inara-api-docs/> y <https://inara.cz/elite/inara-api-devguide/>, ya que el host origen presentaba reto Cloudflare `503` a clientes automatizados). **Cero peticiones POST enviadas.**

### 1. Contrato de cabecera, lote y respuestas (InAPI v1)

- **Endpoint:** `POST https://inara.cz/inapi/v1/` con JSON **sin comprimir** (`Content-Type: application/json`).
- **Whitelist oficial:** `"The Multipurpose Companion (TMC)"` exacto en `header.appName`.
- **Página de clave personal del usuario:** <https://inara.cz/elite/cmdr-settings-api/> (en la ficha de ajustes de API de Inara).
- **Cabecera de entrada (`header`):**
  - `appName` (`string`, obligatorio): `"The Multipurpose Companion (TMC)"`.
  - `appVersion` (`string`, obligatorio): versión semántica `x.y.z` de Cargo.
  - `isBeingDeveloped` (`boolean`, obligatorio): `cfg!(debug_assertions)` (`true` en desarrollo, `false` en release). La documentación indica que en `true` Inara omite actualizaciones globales como `setCommunityGoal` para no contaminar con datos de prueba; **en `false` con la app ya en whitelist no requiere gestión adicional** (DR-11, DR-33 verificados sin bloqueo).
  - `APIkey` (`string`, obligatorio): clave personal del comandante (inyectada al enviar; **nunca** persistida en el `payload`).
  - `commanderName` (`string`, recomendado/obligatorio en combate): nombre exacto del diario/cAPI.
  - `commanderFrontierID` (`string`, opcional): formato `'F123456'` o `'123456'` (el `FID` del diario es `'F...'`, compatible directamente).
- **Formato de evento de entrada:**
  - `eventName` (`string`, obligatorio).
  - `eventTimestamp` (`string`, obligatorio): ISO 8601 UTC (`2017-05-02T17:30:49Z`); no debe superar 30 días de antigüedad.
  - `eventCustomID` (`integer`, opcional): asignado a `sync_outbox.id`; **Inara confirma que devuelve `eventCustomID` en el objeto de salida** de cada evento si se envió.
  - `eventData` (`object | array`, obligatorio): campos específicos por evento (se aceptan arrays de objetos en `setCommanderRankPilot`, `setCommanderRankEngineer`, `setCommanderReputationMajorFaction`, `setCommanderReputationMinorFaction`, `setCommanderInventory*`, `resetCommanderInventory`).
- **Tamaño máximo de lote:** la documentación y devguide recomiendan agrupar en lotes («event batching») sin imponer un tope < 100 eventos en InAPI v1; se fija `MAX_EVENTS_PER_BATCH = 100` (sustituyendo el antiguo `50`), **sin truncado silencioso** y sin requerir cambios en el limitador (**2 req / 60 s**, intervalo mínimo **35 s**).
- **Reglas del devguide:** prohibido enviar datos de **Legacy (3.8)** y de versiones **Beta**; enviar solo Live y evitar peticiones unitarias repetitivas (coalescencia previa de `set*`).
- **Respuesta (`output`):**
  - `header`: `{ eventStatus: 200 | 400, eventStatusText?: string, eventData?: { userID, userName } }`. Con `header.eventStatus == 400` se cancela el lote entero (fallo de autorización/cabecera).
  - `events`: array devuelto en el **mismo orden de envío**, cada elemento con `{ eventCustomID?: integer, eventStatus: 200 | 202 | 204 | 400, eventStatusText?: string, eventData?: mixed }`:
    - `200`: aceptado OK → `Sent`.
    - `202`: aviso (p. ej. múltiples coincidencias, todo OK) → `Sent` + aviso diagnóstico.
    - `204`: error blando («sin resultados» / no aplicado pero formato válido) → `Sent` + aviso diagnóstico.
    - `400`: error en propiedades/validación del evento → `Rejected` (permanente).
    - `eventStatus` ausente o cuerpo HTML con HTTP 200 → tratado como `Retry` (nunca asumido como éxito).
- **Verificación de clave (`getCommanderProfile`):**
  - Evento `getCommanderProfile` con `{ "searchName": "<commanderName>" }` (o `{}` con clave personal).
  - `header.eventStatus == 200` y evento `200` (o `204` si el nombre aún no tiene historial pero cabecera autorizada con clave personal válida) → `KeyCheck::Valid`; `header.eventStatus == 400` → `KeyCheck::Rejected`; error de red/HTML/429 → `KeyCheck::Unreachable`.

### 2. Mapeo detallado `eventData` (§7.1) y decisiones [T0]

| Evento Inara | Diario(s) origen | Campos obligatorios (`eventData`) | Campos opcionales relevantes | Notas T0 |
|---|---|---|---|---|
| `addCommanderTravelFSDJump` | `FSDJump` | `starsystemName` | `starsystemCoords` `[x,y,z]`, `jumpDistance`, `shipType`, `shipGameID`, `isTaxiShuttle`, `isTaxiDropship` | Además emite `setCommanderReputationMinorFaction` si hay `Factions[].MyReputation`. |
| `addCommanderTravelCarrierJump` | `CarrierJump` | `starsystemName` | `starsystemCoords`, `stationName`, `marketID`, `shipType`, `shipGameID` | No enviar `jumpDistance` (docs indican omitirlo). |
| `addCommanderTravelDock` | `Docked` | `starsystemName`, `stationName` | `starsystemCoords`, `marketID`, `starsystemBodyName`, `starsystemBodyCoords`, `shipType`, `shipGameID`, `isTaxiShuttle`, `isTaxiDropship` | `Location` **nunca** genera `Dock` (aviso oficial Inara). |
| `addCommanderTravelLand` | `Touchdown`, `DropShipDeploy` | `starsystemName`, `starsystemBodyName` | `starsystemCoords`, `starsystemBodyCoords` `[Lat,Lon]`, `shipType`, `shipGameID`, `isTaxiShuttle`, `isTaxiDropship` | **`Touchdown` (cuando el jugador aterriza, `PlayerControlled != false`) y `DropShipDeploy`** disparan `Land`; `ApproachSettlement` no implica aterrizaje. Si `Touchdown` omite `Body`, usar cuerpo actual del tracker. |
| `setCommanderTravelLocation` | `Location` | `starsystemName` | `starsystemCoords`, `stationName`, `marketID`, `starsystemBodyName`, `starsystemBodyCoords` | Coalescencia `set`: conserva último estado de ubicación. |
| `setCommanderRankPilot` | `Rank`, `Progress` | `rankName` (array de entradas) | `rankValue` `[0..13]` / `[0..14]`, `rankProgress` `[0.0..1.0]` | Incluye `combat`, `trade`, `explore`, `cqc`, `soldier`, `exobiologist`, `federation`, `empire`. Normalizar progreso `/ 100.0`. |
| `setCommanderRankEngineer` | `EngineerProgress` | `engineerName`, `rankStage` | `rankValue` `[1..5]` | Array con los ingenieros conocidos (`Invited`, `Acquainted`, `Unlocked`, `Barred`). |
| `setCommanderRankPower` | `Powerplay` | `powerName`, `rankValue` | `meritsValue` | — |
| `setCommanderReputationMajorFaction` | `Reputation` | `majorfactionName`, `majorfactionReputation` `[-1..1]` | — | Normalizar valor del diario (`/ 100.0`): `"federation"`, `"empire"`, `"alliance"`, `"independent"`. |
| `setCommanderReputationMinorFaction` | `FSDJump`, `Location` (`Factions[]`) | `minorfactionName`, `minorfactionReputation` `[-1..1]` | — | Emite con `sub >= 1` (`MyReputation / 100.0`). |
| `setCommanderCredits` | `LoadGame` | `commanderCredits` | `commanderAssets`, `commanderLoan` | **Solo en `LoadGame`** **[DR-13]** siguiendo aviso oficial de no spammear cambios menores. |
| `setCommanderShip` | `LoadGame`, `ShipyardSwap`, `Loadout`, `SetUserShipName`, `StoredShips` | `shipType`, `shipGameID` | `shipName`, `shipIdent`, `isCurrentShip`, `isHot`, `shipHullValue`, `shipModulesValue`, `shipRebuyCost`, `shipMaxJumpRange`, `shipCargoCapacity`, `starsystemName`, `stationName`, `marketID` | En `StoredShips`, emitir un `setCommanderShip` por nave en `ShipsHere` (ubicación actual) y `ShipsRemote` (si `!InTransit`, con `StarSystem` y `ShipMarketID`; o `setCommanderShipTransfer` si `InTransit` con tiempo). |
| `setCommanderShipLoadout` | `Loadout` | `shipType`, `shipGameID` | `shipLoadout` (`slotName`, `itemName`, `itemValue`, `itemHealth`, `isOn`, `isHot`, `itemPriority`, `itemAmmoClip`, `itemAmmoHopper`, `engineering`) | Modifiers con `name`, `value`, `originalValue`, `lessIsGood`. |
| `addCommanderShip` | `ShipyardNew` (+ `ShipyardBuy` contexto) | `shipType`, `shipGameID` | — | En `ShipyardBuy` manejar antes `StoreOldShip` (`setCommanderShip` docked) o `SellOldShip` (`delCommanderShip`). |
| `delCommanderShip` | `ShipyardSell`, `SellShipOnRebuy`, `ShipyardSwap`/`Buy` con `SellOldShip` | `shipType`, `shipGameID` | — | — |
| `setCommanderShipTransfer` | `ShipyardTransfer` | `shipType`, `shipGameID`, `starsystemName`, `stationName`, `transferTime` | `marketID` | `starsystemName` y `stationName` son destino actual del comandante (donde se solicita). |
| `setCommanderInventoryMaterials` | `Materials` + deltas | `[{ itemName, itemCount }]` | — | Conjunto completo mantenido en `TransientTracker` **[DR-12]**. |
| `setCommanderInventoryCargo` | `Cargo` / `Cargo.json` | `[{ itemName, itemCount }]` | `isStolen`, `missionGameID` | Conjunto completo (incluye drones/limpets). |
| **`setCommanderInventory`** | `ShipLocker` / `ShipLocker.json` | `[{ itemName, itemCount, itemType, itemLocation: "ShipLocker" }]` | — | **Evento Odyssey confirmado:** `setCommanderInventory` con `itemLocation: "ShipLocker"` y `itemType` ∈ `{'Item', 'Component', 'Data', 'Consumable'}` (equivalentes a `Items`, `Components`, `Data`, `Consumables` en diario). Si una categoría queda vacía, usar `resetCommanderInventory` con `{ itemType, itemLocation: "ShipLocker" }` (o antes de reescribir si vacía). |
| `addCommanderMission` | `MissionAccepted` | `missionName`, `missionGameID`, `starsystemNameOrigin`, `stationNameOrigin` | `missionExpiry`, `influenceGain`, `reputationGain`, `minorfactionNameOrigin`, `starsystemNameTarget`, `stationNameTarget`, `minorfactionNameTarget`, `commodityName`, `commodityCount`, `targetName`, `targetType`, `killCount`, `passengerType`, `passengerCount`, `passengerIsVIP`, `passengerIsWanted` | Requiere ubicación de origen conocida en tracker. |
| `setCommanderMissionCompleted` | `MissionCompleted` | `missionGameID`, `donationCredits`, `rewardCredits` | `rewardPermits`, `rewardCommodities`, `rewardMaterials`, `minorfactionEffects` | `donationCredits`/`rewardCredits` a `0` si ausentes en el diario. |
| `setCommanderMissionFailed` / `…Abandoned` | `MissionFailed` / `MissionAbandoned` | `missionGameID` | — | — |
| `addCommanderCombatDeath` | `Died` | `starsystemName` | `opponentName`, `wingOpponentNames` (`isPlayer` omitido si el diario no lo indica) | Si colisión sin atacante (`KillerName`/`Killers` vacíos), omitir envío porque Inara lo ignora. |
| `addCommanderCombatInterdicted` | `Interdicted` | `starsystemName`, `opponentName`, `isPlayer` | `isSubmit` | Si falta `Interdictor`, usar `Faction` o `Power`. |
| `addCommanderCombatInterdiction` | `Interdiction` | `starsystemName`, `opponentName`, `isPlayer` | `isSuccess` | Si falta `Interdicted`, usar `Faction` o `Power`. |
| `addCommanderCombatInterdictionEscape` | `EscapeInterdiction` | `starsystemName`, `opponentName`, `isPlayer` | — | Si falta `Interdictor`, usar `Faction` o `Power`. |

### 3. Inventario de fixtures Inara (`tests/fixtures/sync/inara/`)

- `response_ok.json`, `response_mixed.json`, `response_header_400.json`, `response_no_status.json`, `response_html_200.html`, `profile_ok.json`.
- `events/<evento>.json`: 28 fixtures (`{ "journal": ... | [...], "inara": { "eventName": ..., "eventTimestamp": ..., "eventData": ... } }`) cubriendo todos los eventos de la tabla §7.1 (incluido `ShipLocker` → `setCommanderInventory` y `StoredShips` → `setCommanderShip`).

**Alineación con §7.1:** 100 % alineado sin bloqueos ni desviaciones.

