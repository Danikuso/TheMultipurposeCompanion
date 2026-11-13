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

_Pendiente (tarea T0-EDSM)._

## Inara

_Pendiente (tarea T0-Inara)._
