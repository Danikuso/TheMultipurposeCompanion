# Spike de red de F3: Spansh comercio, Ardent exportaciones y coordenadas

Pruebas reales el 2026-10-19 con `curl`, `User-Agent: TMC/0.1 (spike)`, 14 peticiones en total separadas >= 1 s. Solo sistemas públicos (Sol / Abraham Lincoln, Shinrarta Dezhra). Los fixtures (`tests/fixtures/net/spansh_trade_job.json`, `spansh_trade_result.json`, `ardent_nearby_exports_gold.json`, `coords_by_name.json`) son **sintéticos** con la forma real (nombres, ids y precios ficticios). Comparar con `spike_spansh_f1.md` y `spike_ardent_f2.md`.

## 1. Spansh: ruta comercial

- Alta: `POST https://spansh.co.uk/api/trade/route`, formulario (`application/x-www-form-urlencoded`), como las rutas de F1. Responde **202** `{"job":"<UUID>","status":"queued"}`: **`parse_job` de F1 sirve tal cual**.
- Sondeo: `GET /api/results/{job}`, el mismo `results_url`. Mientras corre: 202 con `status:"queued"` y sin `result`; al terminar: 200 con `status:"ok"`, `state:"completed"` y `result`. Tardó unos 15-20 s con `max_hops=2`. El sondeo de F1 (Pending si no hay `result`) vale, pero **`parse_route_result` de F1 no**: aquí `result` es un **array**, no un objeto con `system_jumps`/`jumps` (devolvería `Malformed`). Hace falta `parse_trade_result` propio y que el despachador distinga por tipo de ruta, no por forma.
- Parámetros aceptados (probados; el trabajo ecoa en `parameters` los que usa): `system`, `station` (nombre; Spansh lo resuelve a `starting_station_id`), `starting_capital`, `max_cargo`, `max_hops`, `max_hop_distance`, `max_system_distance`, `max_price_age` (segundos; 86400 = 1 día), `requires_large_pad` (0/1), `allow_planetary` (0/1), `allow_player_owned` (0/1), `permit` (0/1), `unique` (0/1). Con 0 algunos no se ecoan (`requires_large_pad`, `allow_*`, `permit`), con 1 sí; tratarlos como opcionales y enviar siempre `0`/`1`.
- **No existe parámetro de tamaño de plataforma M**, solo `requires_large_pad`.

### Forma del resultado

`result`: array de saltos. Cada salto:

| Campo | Tipo | Notas |
|---|---|---|
| `source` / `destination` | objeto | `system`, `system_id64`, `station`, `market_id`, `x`,`y`,`z`, `distance_to_arrival` (Ls), `market_updated_at` (epoch s, entero) |
| `distance` | f64 | ly del salto |
| `commodities` | array | una o varias por salto |
| `commodities[].name` | str | nombre legible ("Fruit And Vegetables"), no el `Type` del diario |
| `commodities[].amount` | u32 | toneladas |
| `commodities[].profit` | i64 | por tonelada |
| `commodities[].total_profit` | i64 | `amount * profit` |
| `commodities[].source_commodity` | `{buy_price, sell_price, supply, demand}` | se **compra** en origen a `buy_price` |
| `commodities[].destination_commodity` | idem | se **vende** en destino a `sell_price` |
| `total_profit` | i64 | suma del salto |
| `cumulative_profit` | i64 | acumulado hasta el salto |

El destino de un salto es el origen del siguiente. La respuesta **no trae tamaño de plataforma ni tipo de estación**. La antigüedad del mercado es `market_updated_at` en epoch; hay que convertirla con un «ahora» inyectado (como en Ardent F2).

### Decisión de plataforma (criterio del brief)

Spansh solo acepta `requires_large_pad`. El adaptador envía `requires_large_pad = (pad == Large)`; para `Small`/`Medium` envía 0. Como el resultado no incluye el tamaño de plataforma, el filtro local de paradas sin plataforma compatible **no puede aplicarse sobre la respuesta de Spansh** (para M habría que consultar el tamaño por estación, p. ej. con Ardent; no verificado aquí). Propuesta: en F3 el filtro local se limita a lo que Spansh garantiza (large) y se documenta como límite; el filtrado fino por M queda opcional vía Ardent. La interfaz (`TradeRouteParams` con `pad`) no cambia.

## 2. Ardent: `nearby/exports`

`GET https://api.ardent-insight.com/v2/system/address/{systemAddress}/commodity/name/{commodity}/nearby/exports?maxDistance=50&fleetCarriers=false&minVolume=100&maxDaysAgo=3`

- Misma estructura que `nearby/imports` de F2. Array de mercados con: `commodityName`, `marketId`, `stationName`, `stationType`, `distanceToArrival`, `maxLandingPadSize` (1/2/3), `bodyId`/`bodyName` (nulables), `systemAddress`, `systemName`, `systemX/Y/Z`, `buyPrice` (lo que pagas al comprar), `sellPrice`, `stock` (oferta), `demand`, `meanPrice`, `updatedAt` (ISO 8601), `distance` (ly, entero).
- Para exportaciones lo relevante es `buyPrice` y `stock`. No se verificó con otro valor de `minVolume` si filtra por `stock`; re-filtrar en local.
- **`minLandingPadSize` se ignora** (con y sin el parámetro: 209 resultados, pads 2 y 3): el filtro de plataforma es local, igual que en imports. Incluye `OnFootSettlement` y puede incluir carriers: reutilizar `excluded()`/`is_carrier_name` de F2.
- `maxDaysAgo` puede no respetarse: re-filtrar por `updatedAt` como en F2.
- 209 resultados a 50 ly para `gold` desde Sol: limitar a `MAX_MARKETS`.

## 3. Coordenadas por nombre

| Fuente | Petición | Resultado |
|---|---|---|
| Ardent | `GET /v2/system/name/{name}` | 200 `{systemAddress, systemName, systemX, systemY, systemZ, ...}`; inexistente: **404** `{"error":"Not Found","message":"System not found"}` |
| EDSM | `GET https://www.edsm.net/api-v1/system?systemName=&showCoordinates=1&showId=1` | 200 `{name,id,id64,coords:{x,y,z},coordsLocked}`; inexistente: **200 `[]`** |
| Spansh | `GET /api/systems/search?q=` | 400 `Invalid request`; descartada (sin endpoint simple verificado) |

**Elegida: Ardent** (mismo host y adaptador que F2, trae además `systemAddress`, 404 claro). **Respaldo: EDSM** (`[]` = desconocido; respetar 10 peticiones/10 s). Si fallan ambas, `SystemCoords` queda ausente y la UI ordena sin distancia.

## 4. Mapeo a Task 7

- `TradeRouteParams` -> formulario: `system`, `station`, `starting_capital`, `max_cargo`, `max_hops`, `max_hop_distance`, `max_system_distance`, `max_price_age` (segundos), `requires_large_pad=(pad==Large)`, `allow_planetary`, `allow_player_owned`, `permit`, `unique`, flags 0/1.
- `TradeRoutePlan`: saltos con origen/destino (sistema, estación, market_id, id64, coords, Ls), `distance_ly`, mercancías (`name`, `amount`, `buy_price`=source.buy_price, `sell_price`=destination.sell_price, `profit`, `total_profit`), antigüedad desde `market_updated_at`.
- `RemoteExport` desde Ardent: `buyPrice`, `stock`, `maxLandingPadSize`, `updatedAt` + estación/sistema/coords.
- `SystemCoords { x, y, z, id64? }` desde Ardent `systemX/Y/Z` o EDSM `coords`.

## 5. Dudas

- El trabajo tardó ~15-20 s: el sondeo de comercio necesita un tope de tiempo mayor que el de F1.
- Sin tamaño de plataforma en la respuesta de Spansh (ver §1).
- No se verificó si Spansh filtra de verdad con `allow_planetary=0`.
