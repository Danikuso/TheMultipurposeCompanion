# Spike 1 de F1: Spansh (verificado con peticiones reales)

Fecha de las pruebas: 2026-10-17 (entre 20:47 y 20:54 UTC). Base: `https://spansh.co.uk/api`. Cliente: `curl` con `User-Agent: TMC/0.1.0 (spike)`, como mucho 1 petición/s en las altas, sondeos cada 1-4 s y un trabajo de ruta a la vez (salvo un solapamiento puntual de dos trabajos de galaxia al sondear parámetros). Solo se usaron sistemas públicos (Sol `10477373803`, Colonia `3238296097059`, Maia `8216113749`). Las respuestas grabadas están en `tests/fixtures/net/spansh_*.json` (recortadas; sin datos de comandantes: `grep -i cmdr` sin resultados).

Contexto previo: `docs/research/notes/specialized_apis_static_data.md` §1. Este documento fija lo que usarán `tmc_net::spansh` (Task 10) y la tabla de mapeo a `RemoteSystem` / `PlannedHop` (Task 6).

## 1. Resumen

| Función | Endpoint | Estado |
|---|---|---|
| Ficha por id64 | `GET /api/system/{id64}` (envuelto en `record`) y `GET /api/dump/{id64}` (envuelto en `system`) | Funciona. Trae política y Powerplay |
| Ruta de neutrones | `POST /api/route` (form) + `GET /api/results/{job}` | Funciona |
| Ruta de galaxia | `POST /api/generic/route` (form) + `GET /api/results/{job}` | Funciona; exige 7 parámetros de nave |
| Autocompletado | `GET /api/systems/field_values/system_names?q=` | Funciona |
| Límites | Sin cabeceras `RateLimit-*` ni `Retry-After` | Se mantiene el límite conservador propio |

Decisión (criterio del brief): la ficha por id64 **sí** trae política y Powerplay, así que **no hace falta EDSM para esos campos**. EDSM queda solo como respaldo si Spansh falla o responde 404. No se probó EDSM en este spike.

Dos campos **no existen** en Spansh: `discovered` (de `RemoteSystem`) y la clase de estrella de cada salto (`star_class` de `PlannedHop`). Son `Option` en la forma interna, así que no hay desviación de interfaz.

Todas las respuestas (incluidos errores y trabajos pendientes) fueron JSON (`application/json;charset=UTF-8`); nunca HTML ni desafío. Servidor `nginx/1.31.0`; cabeceras vistas: `Server`, `Date`, `Content-Type`, `Content-Length`, `Connection`, `Vary`, `Strict-Transport-Security`, `X-Frame-Options`. No hay ninguna de límite de uso ni de caché.

## 2. Ficha de sistema por id64

`GET https://spansh.co.uk/api/system/10477373803` -> 200, 51 036 B, 1,09 s (primera petición). Fixture: `spansh_system_ok.json` (recortado a 3 cuerpos, 1 estación y 2 facciones; el real tiene 40 cuerpos, 67 estaciones y 6 facciones).

Campos de `record` a nivel de sistema: `id64`, `name`, `x`,`y`,`z`, `region`, `allegiance`, `government`, `controlling_minor_faction`, `controlling_minor_faction_state`, `controlling_power`, `power` (lista), `power_state`, `power_state_control_progress`, `power_state_reinforcement`, `power_state_undermining`, `primary_economy`, `secondary_economy`, `security`, `population`, `body_count`, `bodies[]`, `minor_faction_presences[]` (`name`, `allegiance`, `government`, `influence`, `state`), `stations[]`, `estimated_scan_value`, `estimated_mapping_value`, `landmark_value`, `needs_permit`, `known_permit`, `is_colonised`, `is_being_colonised`, `thargoid_war_*`, `updated_at` (ISO UTC).

Cada cuerpo de `bodies[]`: `id`/`id64`, `name`, `type` (`Star`/`Planet`), `subtype` (p. ej. `"G (White-Yellow) Star"`), `is_main_star` (`true` o `null`), `distance_to_arrival`, `estimated_scan_value`, `estimated_mapping_value`, `terraforming_state` (planetas), `landmarks[]` opcional. Es un resumen: sin mineralogía ni biológicos.

`GET /api/dump/{id64}` -> 200, 2 070 862 B (2 MB), 1,24 s: volcado completo en camelCase (`bodyCount`, `controllingFaction{name,allegiance,government,state,activeStates[]}`, `controllingPower`, `powerState`, `powers[]`, `primaryEconomy`, `coords{x,y,z}`, `date`, `timestamps`, `factions[]`, cuerpos completos con `spectralClass`, etc.). **No usarlo para la tira**: pesa unas 40 veces más. Usar `/api/system/`.

Error: `GET /api/system/1` -> 404, 0,32 s, cuerpo `{"error":"Could not find record with id64 1"}` (fixture `spansh_system_404.json`). Un 404 significa «Spansh no conoce el sistema»: se traduce a `NetOutcome::NoData` (y permite probar EDSM).

### Mapeo a `RemoteSystem`

| Campo interno | Campo de Spansh (`/api/system`) | Notas |
|---|---|---|
| `id64` | `id64` | |
| `name` | `name` | |
| `discovered: Option<bool>` | no existe | Siempre `None`. Que exista el registro solo indica que alguien lo envió a Spansh. Manda lo local (`WasDiscovered`) |
| `body_count: Option<u32>` | `body_count` | Cuerpos **conocidos por Spansh**, no el total real |
| `primary_star: Option<String>` | `bodies[]` con `is_main_star == true` -> `subtype` | Texto tipo `"G (White-Yellow) Star"`. Si no hay cuerpos o ninguno es principal -> `None` |
| `allegiance` | `allegiance` | |
| `government` | `government` | Texto legible (`"Democracy"`), no el `$government_..;` del diario |
| `controlling_faction` | `controlling_minor_faction` | |
| `faction_state` | `controlling_minor_faction_state` | |
| `economy` | `primary_economy` | Texto legible (`"Refinery"`) |
| `second_economy` | `secondary_economy` | |
| `security` | `security` | `"High"`, etc. |
| `population: Option<u64>` | `population` | |
| `powers: Vec<String>` | `controlling_power` si existe; si no, `power` | En Sol `power` trae las 6 potencias que compiten y `controlling_power` la que controla. Propuesta: `powers = [controlling_power]` y, si falta, `power` tal cual. Decidir en Task 10 |
| `powerplay_state: Option<String>` | `power_state` | `"Stronghold"`, etc. |

Presentes y no usados hoy por `RemoteSystem`: `power_state_control_progress`, `estimated_scan_value`, `estimated_mapping_value`, `needs_permit`, `thargoid_war_*`, `minor_faction_presences`.

Frescura: `updated_at` del sistema; el volcado trae un sello por bloque en `timestamps` (`factions`, `powers`, `powerState`, `controllingPower`).

## 3. Ruta de neutrones

Alta: `POST https://spansh.co.uk/api/route`, `Content-Type: application/x-www-form-urlencoded`, cuerpo `from=Sol&to=Colonia&range=50&efficiency=60&supercharge_multiplier=4`. Respuesta **202**, 0,3 s: `{"job":"1F2B690E-CA6C-11F1-B0E5-AD2C2EDDEC53","status":"queued"}` (fixture `spansh_route_job.json`).

Parámetros: `from` y `to` por **nombre** de sistema (no id64), `range` (ly), `efficiency` (1-100, 60 por defecto), `supercharge_multiplier` (4 ó 6), `via` (opcional, repetible).

Errores de alta (400, JSON):
- `range` pequeño: `{"error":"range must be greater than 10 LY"}`. (Probado enviando un valor no numérico en la query; el mensaje es el de rango mínimo. La investigación previa documentó además que el cliente web exige > 20.)
- Sistema destino inexistente (`NoSuchSystemXyz`): `{"error":"Could not find finishing system"}`.
- Faltan campos (investigación previa): `{"error":"from, to and range are required"}`.

Sondeo: `GET /api/results/{job}`.
- Pendiente: **HTTP 202** con `status:"queued"`, `state:"started"`, `created_at`, `updated_at`, `parameters` y **sin** `result`. Cada sondeo ~0,3 s.
- Terminado: **HTTP 200**, `status:"ok"`, `state:"completed"`, con `result`. Sol -> Colonia (22 000 ly, 128 saltos): `created_at` 20:48:35, `updated_at` 20:48:49 (~14 s); la respuesta final tardó 3,7 s.
- Trabajo inexistente: 404 `{"error":"Not Found"}` (visto con un trabajo nunca creado en esta sesión; la investigación previa vio `{"error":"Unable to find route"}` con un GUID a ceros). Tratar ambos como 404.

Forma del resultado (fixture `spansh_route_neutron_result.json`, recortado a 4 puntos + el último):

```json
"result": {
  "source_system": "Sol", "destination_system": "Colonia",
  "distance": 22000.47, "range": "50", "efficiency": "60", "via": [], "total_jumps": 128,
  "system_jumps": [
    { "system": "PSR J1752-2806", "id64": 22712681061, "x": -10.96875, "y": -6.84375, "z": 407.28125,
      "jumps": 10, "distance_jumped": 407.49, "distance_left": 21629.39, "neutron_star": true }
  ]
}
```

Importante: la lista son **puntos de paso**, no saltos sueltos. `jumps` = nº de saltos desde el punto anterior y `distance_jumped` = distancia del **tramo** (varios saltos). `neutron_star` indica que se supercarga allí. El primer elemento (`jumps: 0`) es el origen. No hay campo de repostaje, clase de estrella ni combustible.

### Mapeo a `PlannedHop` (neutrones)

| Campo interno | Spansh | Notas |
|---|---|---|
| `name` | `system` | |
| `id64` | `id64` | siempre `Some` |
| `star_pos` | `[x, y, z]` | |
| `distance_ly` | `distance_jumped` | De **tramo**, no de salto. Pérdida de información: `jumps` (saltos del tramo) no tiene campo interno. Opcional: añadir `jumps: Option<u32>` a `PlannedHop` en Task 6; si no, se descarta sin afectar a la copia automática |
| `neutron` | `neutron_star` | |
| `refuel` | no existe | `false` |
| `star_class` | no existe | `None` |
| `PlannedRoute.total_ly` | `result.distance` | |

Descartar el primer elemento (`jumps == 0`, el origen) al construir los hops.

## 4. Ruta de galaxia («exacta»)

Alta: `POST https://spansh.co.uk/api/generic/route` (form). Respuesta 202 `{"job":"...","status":"queued"}`; sondeo igual que en neutrones.

Origen y destino por **id64** (`source`, `destination`). Sin ellos: 400 `{"error":"source and destination are required"}` (1,6 s la primera vez). Con id64 pero sin nave: 400 `{"error":"ship details are invalid"}`.

Pruebas de omisión (quitando de uno en uno cada parámetro de un juego válido; las siete respuestas fueron 400 `ship details are invalid`, 0,3 s): son **obligatorios siete parámetros**:

| Parámetro | Significado | Valor de prueba |
|---|---|---|
| `optimal_mass` | masa óptima del FSD (t) | 1052 |
| `base_mass` | masa de la nave en vacío (t) | 300 |
| `tank_size` | depósito principal (t) | 32 |
| `internal_tank_size` | depósito interno (t) | 2 |
| `max_fuel_per_jump` | combustible máximo por salto (t) | 5.2 |
| `fuel_power` | exponente de combustible del FSD | 2.45 |
| `fuel_multiplier` | multiplicador de combustible del FSD | 0.012 |

Un intento con `tank_size=1`, `internal_tank_size=0`, `max_fuel_per_jump=0.5` también dio `ship details are invalid`; no se aisló si el motivo fue el `0` o el depósito mínimo. Con `tank_size=4`, `internal_tank_size=2`, `max_fuel_per_jump=0.5` se aceptó (y el trabajo terminó con ruta).

Opcionales aceptados (eco en `parameters`): `cargo` (t), `range_boost` (ly), `algorithm` (`optimistic` por defecto; `fuel` verificado; según el cliente web también `fuel_jumps`, `guided`, `pessimistic`), `use_injections`, `use_injections_when_required`, `use_supercharge`, `is_supercharged`, `exclude_secondary`, `refuel_every_scoopable`, `max_time`, `ship_build`, `supercharge_multiplier`, `injection_multiplier`. `ship_build` no se probó (aparece como `null`).

Observaciones:
- `reserve_size=0.63` se aceptó pero el servidor lo devolvió como `0` en `parameters`: **parece ignorado** (probado dos veces). Para respetar la reserva hay que ajustar uno mismo `tank_size`.
- `range_boost`, `cargo`, `use_injections`, `refuel_every_scoopable` y `algorithm` sí se reflejan.
- Enviar booleanos como `1`/`0`.
- Tiempos: Sol -> Colonia (607 saltos, depósito de 32 t, dos trabajos en paralelo) ~60 s hasta `completed`; Sol -> Maia (383 ly, 105 saltos, `algorithm=fuel`, `refuel_every_scoopable=1`, `range_boost=10.5`, `cargo=100`, `use_injections=1`) ~8 s (último sondeo 200 en 3,9 s). Con `max_time=5` el trabajo siguió ~50 s y devolvió ruta: no hay que fiarse de `max_time` como tope.

Resultado (fixture `spansh_route_galaxy_result.json`, recortado a 3 saltos + el último):

```json
"result": { "jumps": [
  { "name": "Core Sys Sector QT-R b4-5", "id64": 11665802339769, "x": -8.40625, "y": -4.84375, "z": 36.625,
    "distance": 37.89, "distance_to_destination": 21963.66, "fuel_in_tank": 32, "fuel_used": 5.16,
    "is_scoopable": true, "must_refuel": true, "has_neutron": false, "must_inject": 0 }
] }
```

Aquí cada elemento **es un salto**. El primero (`distance: 0`) es el origen. Con `use_injections=1` salió `must_inject: 1` en los primeros saltos de la prueba corta; sin él `0`. No hay clase de estrella, ni `total_jumps`, ni distancia total.

### Mapeo a `PlannedHop` (galaxia)

| Campo interno | Spansh | Notas |
|---|---|---|
| `name` | `name` | |
| `id64` | `id64` | |
| `star_pos` | `[x, y, z]` | |
| `distance_ly` | `distance` | de un salto |
| `neutron` | `has_neutron` | |
| `refuel` | `must_refuel` | `is_scoopable` solo dice que se puede repostar |
| `star_class` | no existe | `None` |
| `PlannedRoute.total_ly` | `jumps[0].distance_to_destination` | o la suma de `distance` |

`must_inject` y `fuel_in_tank`/`fuel_used` no tienen campo interno hoy (se pierden).

### Parámetros de nave: Loadout -> Spansh

| Spansh | Origen | ¿Disponible? |
|---|---|---|
| `base_mass` | `UnladenMass` del Loadout | Sí |
| `tank_size` | `FuelCapacity.Main` | Sí |
| `internal_tank_size` | `FuelCapacity.Reserve` | Parcial: la semántica exacta en Spansh no está confirmada y 0 puede rechazarse. Pendiente de verificar con una ruta real; hasta entonces, mandar `Reserve` y, si fuese 0, un valor mínimo positivo, anotándolo |
| `reserve_size` | `FuelCapacity.Reserve` | Sí, pero el servidor lo ignora |
| `cargo` | capacidad o carga actual (`Cargo.json`/`CargoCapacity`) | Sí |
| `optimal_mass` | módulo FSD del Loadout (`OptimalMass`, con ingeniería) | Sí si el módulo la trae; si no, tabla estática |
| `max_fuel_per_jump` | módulo FSD (`MaxFuelPerJump`) | Ídem |
| `fuel_power` | exponente del FSD | **Falta** en el Loadout: tabla estática por clase/calificación (datos de juego) |
| `fuel_multiplier` | multiplicador del FSD | **Falta**: ídem |
| `range_boost` | Guardian FSD Booster instalado | Sí (módulo del Loadout: `GuardianFSDBooster` por tamaño) |
| `use_injections`, `refuel_every_scoopable`, `algorithm`, `is_supercharged` | opciones del usuario | n/a |
| `Loadout.MaxJumpRange` | alcance actual | Spansh **no** lo pide; se usa para `range` en neutrones |

**Conclusión del criterio de decisión:** el planificador de galaxia exige datos que el `Loadout` no da directamente (`fuel_power`, `fuel_multiplier`, y en duda `internal_tank_size`). F1 entrega «Galaxia» completando esos valores con una tabla estática del FSD (clase y calificación) a incluir en el adaptador (Task 10), y se anota como **desviación**; si el FSD de la nave no está en la tabla, el modo «Galaxia» se desactiva con explicación. La interfaz interna (`NetQuery`/`NetOutcome`) no cambia: `GalaxyRouteParams` debe llevar `fuel_power` y `fuel_multiplier`. Neutrones solo necesita `range` (alcance del Loadout) y funciona hoy.

## 5. Autocompletado de nombres

`GET https://spansh.co.uk/api/systems/field_values/system_names?q=Sol` -> 200, 0,31 s (fixture `spansh_autocomplete.json`):

```json
{"min_max":[{"id64":10477373803,"name":"Sol","x":0.0,"y":0.0,"z":0.0}, {"id64":5059379007779,"name":"Solitude","x":-9497.65625,"y":-911.0,"z":19807.625}],
 "values":["Sol","Solitude","Sollaro","Solibamba","Solati"]}
```

Hasta 5 sugerencias por prefijo, con la coincidencia exacta primero. Sin resultados: 200 con `{"min_max":[],"values":[]}` (`q=Zzzzqq`, no hay 404). `min_max` trae `id64` y coordenadas, así que **proporciona el id64** que necesita el planificador de galaxia sin otra petición. `parse_autocomplete -> Vec<String>` usa `values`.

Alternativa descartada: `GET /api/search?q=Colonia` -> 200, 0,65 s, 151 KB (fichas completas, `count` 10 000); demasiado pesada.

## 6. Huecos y riesgos

- `discovered` no existe en Spansh (`None`); `star_class` tampoco; `refuel` no existe en neutrones.
- En neutrones `distance_jumped` es por tramo, no por salto.
- Galaxia: `fuel_power` y `fuel_multiplier` obligatorios y fuera del Loadout. `reserve_size` ignorado. Semántica de `internal_tank_size` por confirmar.
- Formato de fallo de un trabajo **no observado** (no se consiguió provocar `status:"failed"`). El adaptador debe tratar también `status == "failed"` o un campo `error` en la respuesta de sondeo como `RoutePoll::Failed`, según el cliente de referencia de la investigación previa, con tests de cuerpos sintéticos.
- No hay límite publicado ni cabeceras: se mantiene 1 petición/s y una ruta a la vez. La primera petición tardó ~1 s (conexión); las siguientes ~0,3 s.
- Un trabajo de galaxia largo (22 000 ly) tarda ~60 s: tope de espera razonable (p. ej. 120 s, como la UI web) con mensaje de «calculando».
- Los endpoints distintos de los 4 GET documentados oficialmente no son estables: parsers tolerantes a claves desconocidas.
