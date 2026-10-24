# Spike de red de F4: comerciantes de materiales y brókeres de tecnología

Pruebas reales el 2026-10-21 con `curl`, 15 peticiones separadas >= 1 s, solo con Sol (`10477373803`) como sistema de referencia. Ningún dato de comandante. Los fixtures son sintéticos (nombres y valores ficticios) con la forma real de la respuesta; los guiones de prueba viven solo en el scratchpad.

## 1. Ardent: `nearest/{servicio}`

`GET https://api.ardent-insight.com/v2/system/address/{addr}/nearest/{svc}?minLandingPadSize=` (mismo patrón que `interstellar-factors` de F2).

| Segmento probado | Resultado |
|---|---|
| `material-trader` | **200**, 20 estaciones |
| `technology-broker` | **200**, 20 estaciones |
| `raw-material-trader`, `manufactured-material-trader`, `encoded-material-trader` | 404 `{"error":"Not Found","message":"Service unknown"}` |
| `guardian-technology-broker`, `human-technology-broker` | 404 `Service unknown` |

Coincide con el código fuente de Ardent (servicios soportados: `interstellar-factors`, `material-trader`, `technology-broker`, `black-market`, `universal-cartographics`, `refuel`, `repair`, `shipyard`, `outfitting`, `search-and-rescue`; ver `community_apis_core.md`).

**Ardent NO distingue el tipo** de comerciante (Raw/Manufactured/Encoded) ni de bróker (Guardian/Human). La respuesta es el mismo array que `nearest/interstellar-factors` (fixture `ardent_nearest_if.json`): campos planos `marketId`, `stationName`, `stationType`, `distanceToArrival`, `maxLandingPadSize` (1/2/3), `systemAddress`, `systemName`, `systemX/Y/Z`, `distance` (ly, entero), `updatedAt` y banderas 0/1 `materialTrader` / `technologyBroker` (más las demás). El único dato de tipo es la bandera booleana. Ojo: una estación con bróker también suele traer `materialTrader: 1` (un bróker puede ser además comerciante), pero eso no indica el tipo de comerciante.

## 2. Spansh: `POST /api/stations/search`

Sí distingue el tipo. Cuerpo (probado):

```json
{"filters":{"material_trader":{"value":["Raw"]},"distance":{"min":0,"max":40}},
 "sort":[{"distance":{"direction":"asc"}}],"size":3,"page":0,"reference_system":"Sol"}
```

- Valores de filtro (`GET /api/stations/field_values/...`): `material_trader` = `Encoded` (445), `Manufactured` (813), `Raw` (373); `technology_broker` = `Guardian` (906), `Human` (2066).
- Probado en Sol a 40-60 ly: Raw 19 resultados, Manufactured 13, Encoded 28, bróker Guardian 44, bróker Human 32; el filtro realmente discrimina.
- Para un bróker: `{"technology_broker":{"value":["Guardian"]}}` (o `Human`).
- Respuesta: `{count, from, reference{id64,name,x,y,z}, search, search_reference, size, results[]}`. Cada estación trae `name`, `market_id`, `type` («Orbis Starport», «Outpost»...), `system_name`, `system_id64`, `system_x/y/z`, `distance` (ly, f64 ordenado ascendente), `distance_to_arrival`, `large_pads`/`medium_pads`/`small_pads` (conteos; **no hay «tamaño máximo»**, se deriva: `large_pads>0` = L, si no `medium_pads>0` = M, si no S), `has_large_pad`, `market_updated_at`/`updated_at` (ISO 8601), y el campo del tipo: **`material_trader`: "Raw"|"Manufactured"|"Encoded"** o **`technology_broker`: "Guardian"|"Human"** (solo presente si la estación lo tiene).
- Peso: ~100 KB por estación (incluye `market`, `modules`, `ships`, `services`...). Pedir `size` pequeño (3-5) y ordenar por distancia; el adaptador debe ignorar los campos que no use. No se probó ningún parámetro para recortar campos.
- Los portaaviones aparecen como estación normal; filtrar con `tmc_model::market::is_fleet_carrier` por `market_id`/`type`.

## 3. Decisión (criterio del brief)

- `NearbyService` nuevos para `ardent::nearest_url`: `material-trader` y `technology-broker` (segmentos exactos) solo cuando no importa el tipo; **no existen** segmentos por tipo.
- Para filtrar por tipo (Raw/Manufactured/Encoded, Guardian/Human) **hace falta el respaldo de Spansh** (Task 7), que es la única fuente con el tipo. Propuesta: Spansh como fuente principal cuando se pide un tipo concreto; Ardent para «el comerciante más cercano, de cualquier tipo» y «bróker más cercano, de cualquier tipo». Si Spansh falla y se pidió un tipo, «sin datos» (no inferir el tipo desde la economía: la wiki da Raw=Refinery/Extraction, etc., pero es heurística y puede fallar).
- La interfaz `NetQuery::NearestService` no cambia: el tipo viaja como parámetro adicional de la variante de servicio.
- Pad mínimo: Ardent filtra en servidor (`minLandingPadSize`); Spansh se re-comprueba en local con los conteos de plataformas.

## 4. Licencia y límites

- Ardent es AGPL: uso **por red** permitido; no se ha copiado código suyo (solo observada la forma de la API y su lista de servicios, un dato). Spansh: API pública usada por herramientas de terceros; mantener `User-Agent` de TMC y sondeo moderado (<= 1 petición/s, sin ráfagas); caché de resultados.
- Fixtures: `tests/fixtures/net/ardent_nearest_material_trader.json`, `ardent_nearest_tech_broker.json`, `spansh_stations_material_trader.json` (este último recortado: sin `market`/`modules`/`ships`/listas de mercancías; los campos presentes conservan nombres y tipos reales).
