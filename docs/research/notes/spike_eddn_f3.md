# Spike EDDN para F3 (Task 0)

Fecha: 2026-10-19. Solo recepción (relay `tcp://eddn.edcd.io:9500`, SUB, JSON comprimido con zlib). No se publicó nada. Código del spike desechable, fuera del repositorio. No se anotan nombres de comandante, sistemas ni posiciones reales; solo agregados.

## Biblioteca

| Candidata | Versión | Licencia | Resultado |
|---|---|---|---|
| `zeromq` (zmq.rs, Rust puro) | 0.6.0 | MIT | Conecta a `eddn.edcd.io:9500`, SUB con `subscribe("")` y recibe mensajes sin problemas. Binario release del spike completo (tokio + serde_json + flate2 + zeromq): 2,1 MB. Expone PUB/SUB/XPUB (tabla de compatibilidad de sockets en `src/lib.rs`), así que sirve para el relay falso local; el PUB local no se ejercitó en el spike, se verificará en el Task 11. |
| `zmq` (libzmq) | no probada | MPL-2.0 | No hizo falta: `zeromq` funciona. |
| `flate2` (descompresión zlib) | 1.1.x | MIT OR Apache-2.0 | OK. |

**Decisión: `zeromq` 0.6.0 (MIT) + `flate2`.** Sin GPL/LGPL. Anotar ambas en `THIRD_PARTY_NOTICES.md` en el Task 11 (cuando entren como dependencia).

## Mediciones (10 min de flujo real, 600 s)

Volumen por esquema (mensajes/s; tamaño comprimido p50/p99/máx. y descomprimido p50/p99/máx., en bytes):

| Esquema | msg/s | Comprimido | Descomprimido |
|---|---|---|---|
| journal/1 | 8,97 | 770 / 1118 / 1515 | 1342 / 2908 / 4237 |
| fsssignaldiscovered/1 | 1,73 | 534 / 2945 / 4493 | 1144 / 17796 / 64909 |
| fssbodysignals/1 | 1,29 | 433 / 464 / 467 | 651 / 716 / 747 |
| dockinggranted/1 | 0,97 | 357 / 389 / 395 | 519 / 574 / 585 |
| commodity/3 | 0,93 | 3624 / 9429 / 9591 | 18190 / 58907 / 60015 |
| fssdiscoveryscan/1 | 0,78 | 380 / 397 / 400 | 559 / 578 / 584 |
| navroute/1 | 0,59 | 427 / 1262 / 3432 | 678 / 3758 / 14393 |
| scanbarycentre/1 | 0,48 | 494 / 512 / 515 | 734 / 754 / 755 |
| outfitting/2 | 0,44 | 2720 / 5239 / 5275 | 20351 / 35688 / 36821 |
| fssallbodiesfound/1 | 0,40 | 379 / 390 / 391 | 544 / 561 / 561 |
| shipyard/2 | 0,35 | 481 / 616 / 624 | 752 / 1094 / 1109 |
| dockingdenied/1 | 0,24 | 349 / 376 / 384 | 520 / 566 / 573 |
| approachsettlement/1 | 0,19 | 641 / 801 / 803 | 1099 / 1542 / 1576 |
| codexentry/1 | 0,13 | 480 / 517 / 530 | 779 / 838 / 857 |
| outfitting/3 | 0,06 | 9833 / 16038 / 16038 | 69128 / 105302 / 105302 |
| scanorganic/1, fcmaterials_*, navbeaconscan/1 | < 0,04 | n/d | n/d |

Total aproximado: 18-19 mensajes/s. `commodity/3`, el que importa: 559 mensajes en 600 s (0,93/s).

Consumo del proceso del spike (descompresión + parseo JSON completo con `serde_json::Value` + filtro por esquema + descarte; muestreo cada 10 s con `Get-Process`; equipo de 12 núcleos lógicos):

| Métrica | Valor |
|---|---|
| CPU acumulada a 591 s | 2,20 s |
| CPU media | 0,37 % de un núcleo (0,03 % del total de la máquina) |
| CPU en régimen estable (t=300 s a 591 s) | 0,39 % de un núcleo |
| RAM (working set) | 8,5 MB al inicio; 9,2 MB al final (sin fugas visibles) |

Nota: el spike parsea todo el JSON de todos los mensajes. Un filtro barato (leer `$schemaRef` sin parsear el resto y descartar lo no deseado) reduciría el CPU; no se midió.

### Coordenadas

| Medida | Resultado |
|---|---|
| `commodity/3` dentro de 200 al de la última posición del propietario (diario más reciente; solo el %) | 30,8 % (172/559) |
| `commodity/3` con coordenadas conocidas por `journal/1` en frío (mapa construido solo durante la ventana, evaluado al llegar el mensaje) | 91,8 % (513/559) |
| Ídem sin arranque en frío (mapa al final de la ventana, ~1020 sistemas) | 95,0 % (531/559) |

Con la cobertura en frío muy por encima del 50 % **no se activa la siembra obligatoria** de coordenadas; el Task 12 la mantiene como mejora (historial propio y `system_coords`), sin cambiar interfaces. Los mensajes sin coordenadas se tratan como «posición desconocida».

### `journal/1` `Docked`

590 mensajes `Docked`: 100 % con `LandingPads` (`{"Small","Medium","Large"}`) y 100 % con `StationType`. Valores vistos de `StationType`: Coriolis, Orbis, Ocellus, Dodec, Outpost, CraterOutpost, CraterPort, SurfaceStation, AsteroidBase, MegaShip, OnFootSettlement, FleetCarrier, SpaceConstructionDepot, PlanetaryConstructionDepot. Claves de `Docked` observadas: timestamp, event, StationName, StationType, Taxi, Multicrew, StarSystem, SystemAddress, MarketID, StationFaction, StationGovernment, StationServices, StationEconomy, StationEconomies, StationAllegiance, StationState, DistFromStarLS, LandingPads, Body, BodyType, StarPos, horizons, odyssey.

## Formas (recortadas)

`commodity/3`: no lleva `StationType` ni `StarPos`. Claves de `message`: `timestamp`, `systemName`, `stationName`, `marketId`, `commodities`, `horizons`, `odyssey`; cada commodity: `name`, `meanPrice`, `buyPrice`, `stock`, `stockBracket`, `sellPrice`, `demand`, `demandBracket`. Cabecera: `uploaderID`, `softwareName`, `softwareVersion`, `gameversion`, `gamebuild`, `gatewayTimestamp`. Fixture sintético: `tests/fixtures/eddn/commodity_ok.json`.

`journal/1` `Docked`: ver `tests/fixtures/eddn/journal_docked.json` (datos ficticios). `StarPos` es un array `[x, y, z]` de `f64` y viene en todos los `journal/1` (`FSDJump`, `Location`, `Docked`...).

## Regla de carriers

Con `Docked`: 130 carriers (`StationType == "FleetCarrier"`) frente a 460 no carriers. MarketID mínimo de carriers: 3 700 277 504; MarketID máximo de no carriers: 4 398 041 859.

**El umbral `marketId >= 3_700_000_000` es necesario pero NO suficiente**: lo cumplieron 445 de 559 `commodity/3`, y los depósitos de construcción (colonización) y estaciones nuevas tienen MarketID por encima. Regla para el Task 10:

1. `journal/1` `Docked` con `StationType == "FleetCarrier"`: carrier (y se recuerda su `MarketID`).
2. `commodity/3` (no trae `StationType`): carrier si el `marketId` está entre los carriers recordados, o si `marketId >= 3_700_000_000` **y** `stationName` cumple `^[A-Z0-9]{3}-[A-Z0-9]{3}$` (formato de indicativo; 104/559 `commodity/3` coincidieron con el patrón).
3. Si no se cumple ninguna, no es carrier.

## Decisión `EddnMode`

Criterio del plan: CPU en reposo con filtro > 0,3 % implica `OnDemand`.

Medido: 0,37-0,39 % de un núcleo (> 0,3 %) con parseo completo, por tanto **`EddnMode::OnDemand`** (decisión conservadora y literal). Para el propietario: en términos de la máquina completa (12 núcleos) son 0,03 %, muy por debajo del 1 % total, y un filtro previo al parseo bajaría el CPU; si la spec §10.13 se mide como % del total, `Always` también sería defendible. Se adopta `OnDemand` por prudencia; cambiar a `Always` no altera interfaces.
