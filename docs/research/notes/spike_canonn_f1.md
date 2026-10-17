# Spike 3 de F1: Canonn (verificado con peticiones reales)

Fecha de las pruebas: 2026-10-17 (entre 21:00 y 21:10 UTC). Cliente: `curl` con `User-Agent: TMC/0.1.0 (spike)`, una petición cada 1 s como máximo, sin paralelismo. Solo datos públicos de la galaxia (Merope, HIP 1727). No se envió ningún nombre de comandante.

Contexto: spec F1 §5.3 (capa 3: refinamiento con Canonn en línea, activada por defecto con opt-out, respuestas solo en `api_cache`) y §6; `spike_licencias_f1.md` (datos de Canonn GPL-3.0: no incorporables al repo, solo consumo por red); `specialized_apis_static_data.md` §3.

## 1. Fixtures: sintéticos

Por decisión del propietario, **no se ha guardado ninguna respuesta real de Canonn en el repo** (datos GPL-3.0). `tests/fixtures/net/canonn_body_ok.json` y `canonn_body_empty.json` los he escrito a mano: reproducen solo la **forma** (nombres de campos y estructura) de `getSystemPoi` con valores inventados (sistema «Sistema Sintetico de Prueba», `entryid` `9900001..9900003`, especies «Ejemplo Sintetico …», coordenadas arbitrarias). El único campo con «cmdr» es `cmdrName: null`.

## 2. Resumen

Base: `https://us-central1-canonn-api-236217.cloudfunctions.net/query` (Google Cloud Functions, `Google Frontend`, `access-control-allow-origin: *`). El CAPIv2 (`api.canonn.tech`) **no conectó** (HTTP 000), coherente con la investigación previa (repos sin cambios desde 2021).

| Función | Endpoint | Estado |
|---|---|---|
| Observaciones por cuerpo (variante vista, con lat/long) | `GET /getSystemPoi?system=<nombre>[&odyssey=Y]` | Funciona. Por **sistema**, no por cuerpo; hay que filtrar por `body` |
| Estadística por variante (propiedades) | `GET /biostats` | Funciona. 1 051 entradas (816 de Biología), todo el catálogo en una respuesta |
| Referencia de códex (id -> nombre) | `GET /codex/ref?category=Biology` | Funciona. 503 KB, 1,7 s |
| Volcado CSV por variante | `https://storage.googleapis.com/canonn-downloads/dumpr/Biology/<entryid>.csv` | Funciona (0,2 s). Solo sistema y coordenadas, sin cuerpo |
| `nearest/codex` | | HTTP 500 (investigación previa); no se usa |

No hay cabeceras de límite (`RateLimit-*`, `Retry-After`) ni de caché (`ETag`, `Cache-Control`). No se probó ningún límite agresivo.

## 3. Por cuerpo: `getSystemPoi`

`GET /getSystemPoi?system=HIP%201727&odyssey=Y` -> 200, 98 545 B, 3,96 s (Merope, sin biología: 29 516 B, 1,98 s). Requiere `system` (sin él: 500 con texto plano «I'm sorry Dave I'm afraid I can't do that»). `cmdr` es opcional: **no se envía** (privacidad); con ello `cmdrName` sale `null` y `scanned` siempre `"false"`. Sistema desconocido: 200 con `{"cmdrName":null,"odyssey":null,"system":"..."}` y sin más claves (forma de `canonn_body_empty.json`).

Forma (ver `canonn_body_ok.json`):

```json
{ "SAAsignals": [ { "body": "AB 1 h a", "count": 6, "english_name": "Biology", "hud_category": "Biology" } ],
  "codex": [ { "body": "A 1", "english_name": "<especie y variante>", "entryid": 2100407, "hud_category": "Biology",
               "index_id": null, "latitude": "-37.75388", "longitude": "-87.99676", "scanned": "false" } ],
  "cmdrName": null, "odyssey": 1, "system": "<nombre>" }
```

- `codex[]`: una fila por **observación** (HIP 1727 devolvió 164 de Biología; una misma variante se repite con distinta posición). `body` es el nombre **sin el nombre del sistema** (`"A 1"`, `"AB 1 h a"`), no el `BodyID`; puede ser `null`. `latitude`/`longitude` son **cadenas**. `hud_category`: `Biology`, `Geology`, `Thargoid`, `Cloud`, `Guardian`, `Anomaly`...
- `SAAsignals[]`: nº de señales por cuerpo (`count`), equivalente al `Signals` del diario; no nombra especies. Mezcla mayúsculas/minúsculas en `body` (`"2 a"`, `"2 B"`).
- `english_name` trae **género, especie y variante** juntas (`"Prasinum Bioluminescent Anemone"`); `entryid` identifica la variante. Género y especie se separan con la tabla propia de TMC (`entryid` -> género/especie) o con `codex/ref`.

### Mapeo a `RemoteBodyBio { hints: Vec<RemoteSpeciesHint { genus, species, weight }> }` (Task 6)

1. Consulta por nombre de sistema (el servicio no acepta id64). Una petición por **sistema**, cacheada en `api_cache` y reutilizada por todos sus cuerpos.
2. Filtrar `codex[]` con `hud_category == "Biology"` y `body ==` nombre del cuerpo sin el nombre del sistema (comparación sin distinguir mayúsculas).
3. Agrupar por `entryid` y contar; `weight = recuento / total` del cuerpo. `genus` y `species` salen de `english_name` (descartando el color) o por `entryid`; las variantes de una misma especie se suman.
4. Cuerpo sin filas -> `hints` vacío -> `NetOutcome::NoData` (manda la capa 1/2).

Es **evidencia observada** (alguien vio esa especie ahí), más fuerte que una probabilidad, pero solo existe para cuerpos ya visitados por otros comandantes con el plugin de Canonn.

## 4. Por propiedades: `biostats`

`GET /biostats` -> 200. **Con el UA de TMC y `Accept-Encoding: gzip` la respuesta llegó sin comprimir: 9,57 MB (3,3 s)**; con el UA de navegador que recomienda el README del servicio y `Accept-Encoding: gzip, deflate` llegó comprimida a 1,69 MB (2,8 s). El servidor solo comprime según el cliente: el adaptador debe pedir gzip y tolerar ambos casos.

Forma: objeto `entryid` (cadena) -> entrada con `name` (`"Bacterium Aurasus - Turquoise"`), `fdevname`, `hud_category`, `count`, `reward`, `bodies[]`, `atmosphereType[]`, `atmosComposition[]`, `volcanism[]`, `materials[]`, `primaryStars[]`, `localStars[]`, `regions[]`, rangos `ming/maxg` (gravedad), `mint/maxt` (temperatura), `minp/maxp` (presión), `mind/maxd` (distancia de llegada) e `histograms` (`atmos_types`, `body_types`, `gravity`, `temperature`, `pressure`, `distance`, `materials`, `primary_stars`, `local_stars`, `volcanic_body_types`, `system_bodies`). Los histogramas de rangos son listas de `{min,max,value}` con huecos `{}`.

Sirve para predicción **por propiedades** (variantes cuyo rango contiene gravedad, temperatura, atmósfera, tipo de cuerpo, estrella y vulcanismo del cuerpo; ponderar por `count`). Es un catálogo global: se baja una vez, se guarda en `api_cache` con TTL largo y se evalúa en local. Se solapa con lo que ya hacen las capas 1-2, así que es un refinamiento, no una dependencia.

## 5. Licencia y condiciones de uso

- Datos y código de Canonn: **GPL-3.0** (`Canonn-GCloud` confirmado: `LICENSE` GPL v3; resto en `spike_licencias_f1.md`). Consumo por red sí; copiar al repo no (AGENTS.md §6). Respetado: solo hay fixtures sintéticos.
- **Condición de uso explícita** (README de `Canonn-GCloud/query`, «Usage»): para usar esas funciones en software propio *«we require that you make a formal request to Canonn and that if appropriate you also include code in your application to capture the data from your users»*. No hay clave ni registro técnico (el servicio responde sin autenticación), pero **sin esa solicitud formal el uso por TMC no tiene permiso expreso**. Es una acción externa del propietario (AGENTS.md §9).
- «Capturar datos de los usuarios» chocaría con la privacidad de TMC (F1 solo consulta, no envía). Debe aclararse en la solicitud.
- No hay límite publicado. Política propia: 1 petición/s global, una consulta por sistema con caché en `api_cache` (TTL sugerido: 24 h `getSystemPoi`, 7 días `biostats`), retroceso ante 5xx y UA identificable.
- Los CSV de Dumpr declaran que «pueden usarse por terceros», pero no traen el cuerpo: no sirven para la capa 3.

## 6. Resultado del criterio de decisión

Criterio del brief: la capa 3 se desactiva si no hay endpoint usable por cuerpo o por propiedades, **o** si la licencia no permite ni la consulta.

- Hay endpoint usable por cuerpo (`getSystemPoi`, filtrando) y por propiedades (`biostats`). La licencia GPL-3.0 no impide la consulta por red.
- Pero hay un requisito de permiso sin cumplir: la solicitud formal a Canonn. Decisión propuesta: **la capa 3 es técnicamente viable y se implementa (Task 6/10), pero queda desactivada hasta que el propietario envíe la solicitud formal y lo confirme**. Mientras tanto el adaptador devuelve `NetOutcome::Disabled` para `NetQuery::BodyBio` (el comportamiento que prevé el criterio) y F1 sigue con las capas 1 y 2 sin bloqueo (spec §11).
- Es una **desviación** de la spec §5.3 («activada por defecto»): el valor por defecto pasa a «desactivada» hasta la confirmación. Debe anotarse en el plan (Desviaciones) y en `status.md` (Bloqueos y pendientes del propietario) por quien coordine.

## 7. Huecos y riesgos

- Servicio comunitario sin contrato: sin SLA, latencia de 1-4 s, 500 con texto plano ante peticiones inválidas. Parser tolerante: JSON inválido o campos ausentes -> `NetOutcome::Failed`/`NoData`, nunca pánico.
- El cuerpo se identifica por **nombre sin prefijo**, no por `BodyID`; los sufijos (`AB 1 h a`) pueden dar falsos emparejamientos si se compara mal. Hace falta un test con nombres que se solapan (`A 1` frente a `A 1 a`).
- `getSystemPoi` incluye posiciones individuales (lat/long) de otros jugadores: se descartan; solo se usa el recuento por variante.
- Límites de tamaño y tiempo en el cliente: ~2 MB para `getSystemPoi`, ~16 MB sin comprimir para `biostats`, 20 s de espera.
- Endpoints no versionados ni documentados como estables: parsers tolerantes a claves desconocidas.
- No se evaluó la calidad de la predicción frente a diarios reales; queda para `tmc-headless eval` en las tareas de predicción.
