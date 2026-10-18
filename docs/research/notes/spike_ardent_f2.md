# Spike Ardent para F2 (2026-10-18)

Peticiones reales con `curl`, User-Agent `TMC/0.1.0 (spike)`, ≤1 req/s, solo datos públicos (Sol, `10477373803`). Ardent es AGPL: los fixtures de `tests/fixtures/net/ardent_*.json` son **sintéticos** (forma real, datos inventados). No se ha copiado ninguna respuesta real.

## Rutas (base `https://api.ardent-insight.com/v2`)

| Uso | Ruta | Resultado |
|---|---|---|
| Compradores de mineral | `GET /system/address/{id64}/commodity/name/{c}/nearby/imports?maxDistance=&fleetCarriers=false&minVolume=&maxDaysAgo=` | 200, array (hasta 1000), ordenado por `sellPrice` desc |
| Factores interestelares | `GET /system/address/{id64}/nearest/interstellar-factors?minLandingPadSize=` | 200, 20 resultados, ordenado por `distance` |
| Catálogo de commodities | `GET /commodities` | 200, `commodityName` en minúsculas |
| Sistema desconocido | cualquier ruta con id64 inexistente | 404 `{"error":"Not Found","message":"System not found"}` |
| Sin resultados | filtros muy estrictos o nombre de mineral erróneo | 200 `[]` |

La ruta por `address` existe: no hace falta `name/{nombre}`.

## Decisión de filtro de plataforma

- `nearby/imports` **NO filtra por `minLandingPadSize`** (con `minLandingPadSize=3` devolvió 250 estaciones de pad 2 y 1 de pad 1 entre 1000). Filtro **local** con `data::ship_pads`; `NetQuery.min_pad` se conserva y el adaptador de imports lo ignora.
- `nearest/interstellar-factors` **SÍ lo filtra en servidor** (`=3` → solo pad 3; `=1` → mezcla 1/2/3). Se envía el parámetro (NearestService lleva `min_pad`) y además se re-comprueba localmente.

## Mapeo campo Ardent → RemoteMarket / RemoteStation

Imports (`RemoteMarket`): `marketId`, `stationName`, `stationType`, `distanceToArrival` (ls), `maxLandingPadSize` (1=S, 2=M, 3=L), `systemAddress`/`systemName`/`systemX/Y/Z`, `distance` (ly, entero), `sellPrice` = lo que paga la estación al jugador, `demand` (unidades), `demandBracket`, `updatedAt` (frescura). `buyPrice`/`stock` valen 0 en imports.

Nearest (`RemoteStation`): `marketId`, `stationName`, `stationType` (puede ser `null`), `distanceToArrival`, `maxLandingPadSize`, `systemAddress`/`systemName`/coords, `distance`, `interstellarFactors` (0/1), `updatedAt`, `bodyName`, `latitude`/`longitude` (null en órbita).

## Nombres de mineral

El `{c}` es `commodityName` de Ardent en minúsculas; el `Type` del diario coincide en los casos comprobados (`platinum`, `painite`, `lowtemperaturediamond`, `opal`, `alexandrite`, `benitoite`, `grandidierite`, `monazite`, `musgravite`, `rhodplumsite`, `serendibite`, `bromellite`, `tritium`, `osmium`, `palladium`). Con `Low Temperature Diamonds` o `lowtemperaturediamonds` devuelve `[]` (no 404). El catálogo también tiene `void opal` (con espacio) además de `opal`; el catálogo mezcla duplicados con y sin espacios (`advanced catalysers`/`advancedcatalysers`): usar la forma sin espacios y minúsculas, y en Task 6 mantener una tabla `journal_type → ardent_name` con `void opal` verificado en ejecución.

## Demanda y frescura

- `demand`: entero (unidades). `demandBracket`: **entero** (0-3) en las 1000 filas observadas; el deserializador debe tolerar también texto (`"High"`, caso incluido en el fixture) y no depender de él: el filtro usa `demand` y `minVolume`.
- Frescura: `updatedAt` (ISO-8601 UTC con milisegundos). `maxDaysAgo` lo aplica el servidor, pero la investigación previa indica que puede no respetarse en rutas nearby: **re-filtrar localmente** por `updatedAt`.
- Defaults elegidos: `maxDistance=100` ly (máx. 500), `fleetCarriers=false`, `minVolume=100`, `maxDaysAgo=3`.

## Tipos de estación, carriers y obras

`stationType` observados: Coriolis, Outpost, Orbis, Ocellus, CraterOutpost, OnFootSettlement, AsteroidBase, CraterPort, Dodec, MegaShip, SurfaceStation, FleetCarrier (solo con `fleetCarriers=true`) y `null` en alguna estación (usar `Option`). Con `fleetCarriers=false` no aparecen carriers (los carriers traen `stationName` tipo `XXX-000`). No aparecieron obras de construcción en las consultas; heurística no verificada: nombre con `Construction Site` o `ColonisationShip` y tipo `SurfaceStation`/`null`; excluir en local. Los `OnFootSettlement` figuran con pad 1 o 3 pero no admiten naves normales: excluirlos en minería.

## Latencia y cabeceras

0,5-0,9 s por petición (imports con 1000 filas ≈ 0,86 s). `Cache-Control: public, max-age=60, s-maxage=300`, `Ardent-Api-Version: 6.4.1`, sin cabeceras de límite de tasa. Cachear por (sistema, mineral, parámetros) ≥ 5 min.

## Fixtures sintéticos

- `ardent_nearby_imports_platinum.json`: hub L, asentamiento a pie con `demandBracket` texto, outpost M, obra de construcción, carrier.
- `ardent_nearby_imports_empty.json`: `[]`.
- `ardent_nearest_if.json`: orbis L, outpost solo M, estación sin tipo y pad 1.
