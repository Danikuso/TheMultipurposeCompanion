# TMC · Especificación de F1 — Navegación, exploración y exobiología

- **Estado:** ✅ aprobada por el propietario (2026-10-17)
- **Fecha:** 2026-10-17
- **Depende de:** [`2026-10-17-tmc-arquitectura-y-f0-design.md`](2026-10-17-tmc-arquitectura-y-f0-design.md) (arquitectura, protocolo, módulos, rendimiento). Esta spec no cambia la arquitectura salvo donde lo dice explícitamente (§4).
- **Maquetas aprobadas:** generadas en la sesión de diseño del 2026-10-17 (tira de ruta v3, exobiología, cuerpos del sistema). Se describen en §3; la descripción escrita es la referencia.

---

## 1. Intención

El propietario quiere que TMC sea **«un compañero útil, no solo un dashboard»**: que en la segunda pantalla aparezca, sin que lo pida, lo que necesita para decidir **en ese momento** y que le acompañe mientras actúa.

Lo que pidió, con sus palabras:

- «En la segunda pantalla estaría bien ver información de la **siguiente estrella en la ruta**» → para decidir si repostar, si parar a explorar o si hay peligro: «Todo eso. Ver qué tipo de sistema es por superpotencia, estado político, si está descubierto… toda la información relevante».
- «De exobiología, ver los **posibles descubrimientos del planeta en el que estoy**», mostrado **al aterrizar** («si he aterrizado es porque he detectado señales biológicas»), y que le acompañe durante el muestreo (muestras tomadas, distancia a la anterior).
- Meter también en F1 el **valor de los cuerpos al escanear** y el **planificador de rutas con Spansh**, que debe usar **la información de la nave para saber la distancia de salto**.

## 2. Objetivos, no objetivos y antiobjetivos

**Objetivos**

1. **Tira de ruta**: siguiente estrella con ficha completa + tres siguientes resumidas, siempre al día tras cada salto.
2. **Cuerpos del sistema**: valor de cada cuerpo (escaneado y si se mapea), señales, marcas y consejo («Mapear», «Aterrizar», «No compensa») mientras se escanea el sistema actual.
3. **Exobiología**: al aterrizar en un cuerpo con señales biológicas, especies posibles ordenadas por valor y acompañamiento del muestreo (3 muestras, distancia mínima, dirección a la anterior, valor sin vender).
4. **Planificador**: rutas con Spansh (neutrones y galaxia) usando el alcance real de la nave; copia automática del siguiente sistema al portapapeles en cada salto.
5. **Alcance de salto** calculado con los datos reales de la nave (FSD, ingeniería, masa, combustible, carga).

**No objetivos (F1)**

- Envío de datos a EDDN/EDSM/Inara (F6). En F1 solo se **consultan** servicios.
- Planificador de fleet carrier, rutas de comercio o de minería (F2/F3/F5).
- Geología, sitios guardianes/thargoides, POI de Canonn, mapa 3D de la galaxia.
- Voz y overlay (F7/F8).

**Antiobjetivos** (sería un fracaso aunque «funcione»)

- Romper el presupuesto de consumo (< 150 MB, < 1 % CPU en reposo) o la latencia p95 < 500 ms por culpa de llamadas de red o listas grandes.
- Que un servicio caído (EDSM, Spansh) deje paneles rotos o vacíos sin explicación: siempre se muestra lo local y «sin datos» en lo remoto.
- Incorporar datos o código GPL/AGPL (p. ej. criterios de SrvSurvey).
- Presentar como seguro lo que es estimado: los valores estimados y las probabilidades se marcan como tales.

## 3. Experiencia de usuario

### 3.1 Disposición

- La **tira de ruta** ocupa **todo el ancho** de la rejilla (las 4 columnas en 2560×1080) en la parte superior de la disposición «Exploración».
- Debajo, un panel de contexto: **Cuerpos del sistema** mientras se está en el espacio; **Exobiología** al aterrizar (`Touchdown`) o al bajar a pie/SRV en un cuerpo con señales biológicas, y mientras haya un muestreo en curso en ese cuerpo.
  - «Cuerpo con señales biológicas» = `FSSBodySignals` o `SAASignalsFound` con señal biológica > 0 para ese `BodyID`, o un `ScanOrganic` en ese cuerpo. Sin DSS basta con `FSSBodySignals`. Si no hay ninguna de esas señales (p. ej. aterrizaje sin escaneo previo), el panel no se activa solo, pero se puede abrir a mano y se activa con el primer `ScanOrganic`. *Decisión del redactor.*
- El cambio entre paneles usa la activación por relevancia de la spec de arquitectura §5.3 (exobiología `Active` al aterrizar en un cuerpo con señales biológicas; exploración `Active` con `GuiFocus` = FSS o en un sistema con cuerpos sin escanear).
- *Decisión del redactor:* la disposición «Exploración» se crea por defecto con estos paneles; el usuario puede moverlos como cualquier otro panel.
- Tipografía: tamaño **mayor que el de F0** en estos paneles (petición del propietario sobre la maqueta: ~17 px de texto, ~24 px en el nombre principal a 2560×1080).

### 3.2 Tira de ruta

**Siguiente estrella (bloque grande, ~3/4 del ancho), en cuatro columnas:**

| Columna | Datos | Origen |
|---|---|---|
| Salto | Nombre, distancia del salto (al), saltos restantes, distancia total al destino, clase de estrella, si es **recogible** (KGBFOAM), combustible estimado tras el salto (%) | Local (ruta, coordenadas, nave) |
| Exploración | Descubierto (sí/no/sin datos), cuerpos conocidos, **tus visitas** (nº), si es estrella de neutrones o enana blanca | Remoto + historial local |
| Política | Superpotencia (allegiance), gobierno, facción controladora y su **estado** (guerra, disturbios, elecciones…; guerra/disturbios/brote resaltados) | Remoto |
| Sistema | Economía (primaria/secundaria), seguridad, población, Powerplay (potencia y estado) | Remoto |

**Tres siguientes (bloques pequeños):** nombre, clase y si es recogible (o NEUTRONES/ENANA BLANCA resaltado), distancia del salto, descubierto, superpotencia, gobierno y estado.

- La ruta es la del **planificador** si hay una activa; si no, la de `NavRoute.json` del juego; si no hay ninguna, la tira muestra el **sistema actual** con la misma ficha.
- Tras cada `FSDJump` la tira avanza. Reglas de la ruta activa del planificador (*decisión del redactor*):
  - si el sistema alcanzado está en el plan (aunque se salten sistemas intermedios), el índice avanza hasta él;
  - si no está en el plan, se marca «fuera de ruta» y se ofrece recalcular desde el sistema actual; **el plan no se descarta** hasta que el usuario recalcule o lo borre;
  - trazar otra ruta en el juego no cambia el plan; `NavRoute.json` solo manda cuando no hay plan activo;
  - «copiar siguiente sistema» (manual y automático) usa siempre el siguiente sistema del plan activo.
- Lo remoto que no haya llegado o haya fallado se muestra como «sin datos» (nunca bloquea lo local).
- Combustible tras el salto: estimado con la fórmula de consumo del FSD (§5.1); si falta información, se oculta.

### 3.3 Cuerpos del sistema

- Cabecera: sistema, insignia descubierto/no descubierto, barra de progreso del FSS (`FSSDiscoveryScan.BodyCount` vs cuerpos escaneados; completo con `FSSAllBodiesFound`), valor actual del sistema y valor si se mapean los recomendados.
- Tabla ordenada por **valor si se mapea** (desc.), una fila por cuerpo escaneado: nombre corto, tipo, distancia a la llegada (ls), valor escaneado, valor si se mapea (— para estrellas), señales (Bio n · Geo n), marcas (terraformable, aterrizable, recogible, primer descubrimiento, ya mapeado, mapeado por ti), **consejo**:
  - «★ Mapear (n sondas)» si el valor de mapeo supera el umbral (configurable; por defecto 500 000 cr) — n = estimación de sondas para el objetivo de eficiencia;
  - «Aterrizar (exobiología)» si hay señales biológicas y es aterrizable; combinable con «Mapear»;
  - «No compensa» en el resto.
- Pie: cuerpos aún sin identificar en el FSS y **datos de exploración sin vender** (se pierden si te destruyen).
- Se resaltan las filas con consejo «★».

### 3.4 Exobiología

- Cabecera: cuerpo, tipo, gravedad, temperatura, atmósfera, nº de señales biológicas, insignia «SIN REGISTRAR · ×5» cuando aplique el bonus de primer registro (estimado, §5.4).
- **Posibles especies** (izquierda), ordenadas por valor: especie, valor base, valor con ×5, distancia mínima entre muestras (CCR del género), **probabilidad** (Alta/Media/Baja), estado (— · ▶ n/3 · ✓ 3/3). Debajo, géneros confirmados por el DSS (`SAASignalsFound.Genuses`) y nº completados de nº de señales.
  - Antes del DSS: candidatos por predicción. Tras el DSS: solo géneros confirmados. Tras el primer `ScanOrganic`: la especie exacta sustituye a sus candidatas del mismo género.
- **Muestreo en curso** (derecha): especie, puntos de muestra (n/3), **distancia a la muestra anterior** frente a la mínima (barra), lo que falta y **dirección** de la anterior (rumbo relativo: «a tu espalda (SO)»), y **valor sin vender** total con lo que sumará al completar la especie actual.
- Al completar las 3 muestras la fila pasa a ✓ y el bloque de muestreo queda a la espera del siguiente `ScanOrganic`.

### 3.5 Planificador de rutas

- Panel con: origen (por defecto el sistema actual), destino (con autocompletado de nombres), modo **Neutrones** o **Galaxia**, eficiencia (neutrones), «usar nave actual» (por defecto sí) y, en Galaxia, opciones básicas (repostar en estrellas recogibles, usar inyecciones FSD no).
- Resultado: lista de saltos (sistema, distancia, neutrón sí/no, repostar sí/no) y totales (saltos, distancia). Al aceptarla se convierte en **la ruta activa** de la tira.
- **Copia automática**: tras cada `FSDJump` sobre la ruta activa, el siguiente sistema se copia al portapapeles (opción activada por defecto) y se emite un aviso discreto. La acción manual «copiar siguiente sistema» de F0 usa también la ruta activa.
- La ruta activa persiste entre reinicios (por comandante) hasta llegar al destino o borrarla.
- *Decisión del redactor:* no hay maqueta del planificador; el diseño visual sigue el estilo HUD de las demás y se valida con el propietario durante la ejecución (tarea de UI del plan).

## 4. Arquitectura

Se mantienen la arquitectura y reglas de dependencia de F0. Cambios:

1. **Nuevo crate `tmc-net`** (ya previsto en la spec de arquitectura §3.1; depende solo de `tmc-model`): clientes HTTP de Spansh y EDSM, limitador de peticiones por servicio, reintentos con retroceso y degradación. Cliente `reqwest` con `rustls` (sin OpenSSL), `User-Agent: TMC/<versión>`.
2. **Acceso de los módulos a la red** sin romper el actor: `ModuleCtx` gana la capacidad de **solicitar** una consulta (`ctx.request(NetRequest)`); la app compone un servicio `NetService` (en `tmc-app`) que ejecuta la petición fuera del actor, guarda en `api_cache` y devuelve el resultado al actor como `SourceEvent::Net(NetResponse)`. `tmc-core` sigue sin depender de `tmc-net` (los tipos `NetRequest`/`NetResponse` viven en `tmc-model`).
   - Cada `NetRequest` lleva un **id de correlación** y la **clave de lo consultado** (p. ej. `SystemAddress`). La respuesta se guarda siempre en caché, pero el módulo solo la aplica si esa clave sigue siendo relevante (está en la tira o es el sistema actual); las respuestas tardías o duplicadas no deshacen datos más nuevos. Peticiones idénticas en curso se agrupan en una sola. *Decisión del redactor.*
3. **Tres módulos nuevos** en `crates/tmc-modules/src/` y `ui/src/modules/`:
   - `navigation`: ruta activa, tira de ruta, planificador, alcance de salto, copia automática;
   - `exploration`: cuerpos del sistema, valores, datos sin vender;
   - `exobiology`: predicción, muestreo, valores sin vender.
4. **Datos estáticos propios** en `crates/tmc-modules/data/` (embebidos con `include_str!`, JSON): tabla de FSD (§5.1), valores y CCR de especies (§5.3), reglas de predicción (§5.3). Cada archivo declara su **fuente y licencia** en una cabecera y en `THIRD_PARTY_NOTICES.md` si procede.
5. **`tmc-model`** tipa los eventos nuevos: `Scan`, `FSSDiscoveryScan`, `FSSAllBodiesFound`, `FSSBodySignals`, `SAAScanComplete`, `SAASignalsFound`, `ScanOrganic`, `SellOrganicData`, `SellExplorationData`, `MultiSellExplorationData`, `CodexEntry`, `JetConeBoost`, `Embark`/`Disembark`, y amplía `Loadout`/`LoadoutModule` con `Engineering` (modificadores) y `Status` con los campos de planeta ya existentes (latitud, longitud, rumbo, `PlanetRadius`) si faltan. Los eventos no tipados siguen yendo a `Other` sin error.

## 5. Datos y cálculos

### 5.1 Alcance de salto y combustible

- Fórmula estándar del FSD: `alcance = (masa_óptima / masa_total) · (1000 · combustible / coef_clase_rating)^(1/exponente_clase)` + bonus del **Guardian FSD Booster** si lo hay; `masa_total = masa sin carga + combustible + carga`.
- `masa_óptima` y `combustible máximo por salto` salen de la **tabla de FSD** propia (clase/rating → masa óptima, máx. combustible por salto, coeficiente, exponente; incluye FSD SCO y Mk II) **modificada por la ingeniería** del `Loadout` (`Modifiers` con `FSDOptimalMass`, `MaxFuelPerJump`, etc.).
- **Validación:** el alcance calculado para «sin carga y depósito lleno» debe coincidir con `Loadout.MaxJumpRange` del juego con tolerancia ±0,05 al; si no coincide (o falta la tabla para ese FSD), se usa `MaxJumpRange` **escalado por masa** y se registra un aviso en el log. Este respaldo es aceptable en producción, de modo que el planificador nunca queda bloqueado por la tabla; el criterio §10.3 mide la fórmula completa. *Decisión del redactor.*
- Se recalcula con cada `Loadout`, cambio de combustible (`Status`) y cambio de carga (`Cargo`).
- **Supercarga** (`JetConeBoost`) se refleja en la ruta (neutrón ×4, enana blanca ×1,5; FSD SCO Mk II: ×6 / ×3 según `docs/research/notes/player_roles_domain.md`).
- Combustible tras el salto: `combustible − f(distancia, masa)` con la inversa de la fórmula; si excede el máximo por salto, el salto se marca como no realizable.

### 5.2 Valor de los cuerpos

- Fórmulas de valor de la comunidad (vigentes desde 3.3) recogidas en `docs/research/notes/player_roles_domain.md` §1: valor base por tipo (k) y masa, multiplicadores de primer descubrimiento (×2,6), mapeo, primer mapeo, eficiencia (×1,25) y bonus de Odyssey. Implementación propia a partir de la fórmula (la fórmula es un hecho; no se copia código).
- «Valor escaneado» = lo que se cobra solo con FSS; «valor si se mapea» = con DSS y eficiencia, aplicando primer descubrimiento/mapeo según `WasDiscovered`/`WasMapped`.
- **Sondas estimadas** para el objetivo de eficiencia: aproximación por radio del cuerpo; se marca «≈». Tras `SAAScanComplete` se usa `EfficiencyTarget`/`ProbesUsed` reales.
- **Datos sin vender:** suma de valores de los cuerpos escaneados/mapeados desde la última venta (`SellExplorationData`/`MultiSellExplorationData`) o muerte (`Died` los pone a cero). Se reconstruye desde el historial al arrancar.
- **Tolerancia:** los valores calculados deben coincidir con lo cobrado en `SellExplorationData` del historial del propietario dentro de ±5 % en el agregado (criterio de aceptación §10).

### 5.3 Exobiología

- **Tabla de especies propia:** género, especie, valor base, CCR del género (valores en `player_roles_domain.md` §2). Valor real confirmado con `SellOrganicData` (Value/Bonus) del historial cuando exista.
- **Predicción** (columna «Probabilidad»):
  1. **Reglas propias por género/especie** escritas por TMC a partir de hechos públicos (tipo de cuerpo, atmósfera, gravedad, temperatura, presión, vulcanismo, clase de la estrella principal, distancia a nebulosas cuando aplique). Prohibido copiar `bio-criteria` de SrvSurvey.
  2. **Estadística del propio historial**: especies encontradas por el comandante en cuerpos con propiedades similares (consulta sobre la tabla `event`), que ajusta Alta/Media/Baja.
  3. **Canonn como refuerzo online** (decisión del propietario, 2026-10-17: unificar opciones A y C). Con servicios online activados, al detectar señales biológicas en un cuerpo o al aterrizar, se consulta Canonn sobre ese cuerpo o sobre cuerpos con propiedades similares y se ajusta la probabilidad. La respuesta va a `api_cache` y **nunca se incorpora al repositorio**, salvo que la licencia de sus datos lo permita explícitamente. Sin conexión, o si Canonn falla, las capas 1 y 2 funcionan solas.
  4. Las reglas son **datos versionados** (JSON) para poder afinarlas sin tocar código; cada regla con su fuente.
  5. La UI indica el **origen** de cada probabilidad (reglas · tu historial · Canonn) para que el usuario sepa cuánto fiarse.
- Si no hay reglas para un género, se muestra «desconocida» en lugar de inventar.
- **Muestreo:** máquina de estados por (cuerpo, especie): `Log` → `Sample` → `Analyse` (`ScanOrganic.ScanType`); posiciones guardadas desde `Status` (lat/long) en el momento de cada muestra. Distancia por **haversine** con `PlanetRadius`; dirección = rumbo a la muestra anterior relativo a `Heading`.
- Cambiar de especie a mitad de muestreo descarta el progreso de la anterior (comportamiento del juego) y la UI lo refleja.
- **Bio sin vender:** suma de especies completadas desde la última `SellOrganicData` o muerte.

### 5.4 Bonus ×5 (primer registro)

- Se marca como **estimado**: aplica si el cuerpo no tiene huella previa (`Scan.WasFootfalled == false`, si el campo existe) o, en su defecto, si el sistema no estaba descubierto. Se confirma a posteriori con `SellOrganicData.Bonus` y se ajusta la heurística. *Decisión del redactor: heurística a verificar con el historial del propietario durante la ejecución.*

## 6. Servicios externos

| Servicio | Uso en F1 | Límite | Caché (`api_cache`) |
|---|---|---|---|
| **Spansh** | Planificador (neutrones y galaxia, trabajos asíncronos con sondeo del resultado); ficha de sistema por `SystemAddress` (id64): cuerpos, facciones, Powerplay; autocompletado de nombres | Conservador: máx. 1 petición/s, una ruta a la vez | Ficha de sistema 24 h; política 1 h; rutas no se cachean |
| **Canonn** | Refuerzo de la predicción de especies (§5.3) | Conservador: máx. 1 petición/s | Por cuerpo 30 días |
| **EDSM** | Alternativa para ficha/política/«descubierto» si Spansh falla o no tiene el sistema | 10 peticiones / 10 s (Cloudflare) | Igual que Spansh |

- **Qué se consulta:** solo la siguiente estrella y las tres siguientes de la ruta, el sistema actual y las rutas pedidas. Nunca el historial completo.
- **Qué se envía:** nombres/ids de sistemas y parámetros de nave para la ruta. **Nada del comandante.** Nunca se consulta nada en sesiones de la galaxia **Legacy** (no aplica envío, pero se omite igual por coherencia).
- **Opt-out visible:** ajuste «Consultar servicios online (Spansh, EDSM, Canonn)», **activado por defecto**, en Ajustes. Desactivado, la tira y los paneles muestran solo lo local; el planificador se desactiva con explicación. *Decisión del redactor.*
- **Degradación:** error/timeout (5 s) → «sin datos» + reintento con retroceso (30 s, 2 min, 10 min); tres fallos seguidos marcan el servicio como caído 10 min y se prueba la alternativa.
- Los endpoints exactos de Spansh (ficha por id64, rutas, autocompletado) **se verifican en un spike previo** (§11); la investigación actual no los re-verificó.

## 7. Persistencia

Tablas nuevas (migraciones con prefijo de módulo):

- `nav_route_plan` (cmdr, creado, origen, destino, modo, parámetros, saltos JSON, índice actual).
- `exo_sample` (cmdr, system_address, body_id, especie, tipo de muestra, lat, lon, ts) para reanudar un muestreo tras reiniciar.
- Visitas por sistema y valores sin vender se **derivan** de la tabla `event` (no se duplican); se pueden materializar en una tabla de resumen si la consulta supera 50 ms en el historial del propietario.

## 8. Protocolo, i18n y ajustes

- **Topics nuevos** (valor completo, pequeños): `navigation.route` (siguiente + 3, ficha), `navigation.plan` (estado del planificador), `navigation.jump` (alcance actual, combustible), `exploration.system` (cabecera + filas), `exploration.unsold`, `exobiology.body`, `exobiology.sampling`, `exobiology.unsold`.
- **Acciones nuevas** (`POST /api/actions/{name}` y WS): `navigation.plan` (destino, modo, opciones), `navigation.accept-plan`, `navigation.clear-plan`, `navigation.recalculate`, `navigation.autocomplete` (vía REST paginado).
- **i18n:** todas las cadenas en `locales/{es,en,fr,de,pt}/` (nuevos `navigation.ftl`, `exploration.ftl`, `exobiology.ftl`); nombres de especies/géneros desde los campos `_Localised` del diario y, si no existen, nombre latino de la tabla propia de especies. La tabla propia de nombres en cinco idiomas queda aplazada para después de F1 (aprobación explícita del propietario, 2026-10-18).
- **Ajustes nuevos:** servicios online on/off, copia automática on/off, umbral de «Mapear» (cr), eficiencia por defecto del planificador.

## 9. Rendimiento y robustez

- Presupuesto de F0 intacto: < 150 MB, < 1 % CPU en reposo; se pasa `scripts/measure-footprint.ps1` al cerrar la fase con los tres paneles visibles.
- Las tablas grandes (sistemas con > 100 cuerpos) se virtualizan; la tabla de especies y la de FSD se cargan una vez.
- Ninguna respuesta remota malformada, campo ausente o evento nuevo tumba la app (tests con respuestas corruptas).
- Las peticiones de red nunca bloquean el actor; el difusor sigue a 10 Hz máx.

## 10. Criterios de aceptación de F1

1. `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, `npm run check`, `npm test`, `npm run build` y CI en verde.
2. Con `tmc-replay` de un fixture de exploración (anonimizado), la **tira de ruta** avanza en cada salto mostrando la ficha local en < 500 ms (p95) y la remota cuando llega; con servicios desactivados o caídos muestra «sin datos» sin errores.
3. El **alcance de salto** calculado coincide con `Loadout.MaxJumpRange` (±0,05 al) en al menos 5 naves/loadouts del historial del propietario, incluidas una con FSD de ingeniería y una con Guardian FSD Booster.
4. Los **valores de cuerpos** agregados coinciden con lo cobrado en `SellExplorationData`/`MultiSellExplorationData` del historial del propietario dentro de ±5 %.
5. Con un fixture de exobiología, el panel aparece al aterrizar, muestra candidatos, sigue 3 muestras con distancia correcta (±1 m frente a cálculo de referencia en test) y suma el valor sin vender igual al `SellOrganicData` posterior.
6. En el historial del propietario, la especie finalmente muestreada está entre los candidatos «Alta» o «Media» en al menos el **70 %** de los cuerpos con señales biológicas. *(Umbral propuesto por el redactor; ajustable tras medirlo.)*
7. El **planificador** calcula una ruta de neutrones real con Spansh usando el alcance de la nave actual, la acepta como ruta activa y la copia automática deja el siguiente sistema en el portapapeles tras cada salto.
8. Todas las cadenas nuevas existen en es/en/fr/de/pt (test de claves).
9. `measure-footprint.ps1` sigue cumpliendo < 150 MB y < 1 % CPU en reposo con la disposición «Exploración» visible.
10. `THIRD_PARTY_NOTICES.md` recoge las fuentes y licencias de los datos estáticos nuevos; ningún dato GPL/AGPL incorporado.

### 10.1 Cómo se verifican los criterios con el historial real

Los criterios 3, 4 y 6 se miden con un subcomando nuevo **`tmc-headless eval`** (decisión del propietario, 2026-10-17: en la app headless para no cambiar las dependencias de §3.1) que se ejecuta **en local** sobre la carpeta de diarios del propietario (o la base `tmc.db` ya importada) y solo imprime **métricas agregadas** (nº de casos, aciertos, error medio y máximo), sin nombres de comandante, sistemas ni datos personales. Nada se sube ni se guarda en el repo. Un agente puede ejecutarlo en la máquina del propietario; los tests de CI usan fixtures anonimizados pequeños con los mismos cálculos. *Decisión del redactor.*

## 11. Riesgos y spikes previos

| Riesgo | Mitigación |
|---|---|
| Endpoints de Spansh no documentados o cambiantes | **Spike 1** al inicio del plan: verificar ficha por id64, rutas (neutrones/galaxia) y autocompletado con peticiones reales; documentar en `docs/research/`. |
| Licencia de la tabla de FSD (p. ej. coriolis-data) y de valores de especies | **Spike 2**: confirmar licencia de cada fuente antes de incorporarla; si no es compatible, construir la tabla desde hechos públicos y verificarla contra `MaxJumpRange`/`SellOrganicData` del historial. |
| Predicción de especies poco precisa sin SrvSurvey | Reglas propias + estadística del historial + refuerzo de Canonn; criterio §10.6 medible; reglas en datos versionados. |
| API de Canonn no verificada (existencia, endpoints, licencia de los datos) | **Spike 3**: verificar con peticiones reales qué ofrece Canonn por cuerpo/especie y bajo qué licencia; documentar en `docs/research/`. Si no sirve, F1 sigue con las capas 1 y 2 (sin bloqueo). |
| Heurística del bonus ×5 | Marcada como estimada; se valida con `SellOrganicData.Bonus`. |
| EDSM inestable (Cloudflare/403) | Spansh como fuente principal; EDSM solo de respaldo. |
| Fixtures: faltan diarios con exobiología/exploración | Extraer del historial del propietario con `tmc-replay anonymize` + revisión manual. |

## 12. Decisiones del redactor (a revisar por el propietario)

1. Spansh como fuente principal y EDSM como respaldo.
2. Servicios online **activados por defecto** con opt-out visible.
3. Copia automática del siguiente sistema activada por defecto.
4. Umbral «Mapear» por defecto 500 000 cr.
5. Disposición «Exploración» por defecto: tira arriba a todo el ancho; debajo, cuerpos del sistema o exobiología según contexto.
6. Ruta del planificador prioritaria sobre `NavRoute.json` cuando está activa.
7. Umbral de precisión de la predicción: 70 %.
8. Tres módulos (`navigation`, `exploration`, `exobiology`) en vez de uno.

## 13. Ramas y entregas

- Rama `fase/f1-navegacion-exploracion`, creada desde `main` **después** de fusionar `fase/f0-cimientos`.
- *Propuesta:* el plan puede entregar por bloques verificables (1. spikes + `tmc-net` + eventos; 2. navegación; 3. exploración; 4. exobiología; 5. cierre), cada bloque con la app funcionando.
