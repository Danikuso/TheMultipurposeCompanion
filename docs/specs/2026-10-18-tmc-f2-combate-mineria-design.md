# TMC · Especificación de F2 — Cazarrecompensas/combate (incl. AX) y minería

- **Estado:** borrador pendiente de aprobación del propietario
- **Fecha:** 2026-10-18
- **Depende de:** [`2026-10-17-tmc-arquitectura-y-f0-design.md`](2026-10-17-tmc-arquitectura-y-f0-design.md) y la infraestructura de F1 ([`2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md`](2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md)): `tmc-net` (limitador, salud, caché `api_cache`, opt-out), `ModuleCtx::request`/`store_task`, disposiciones y relevancia, i18n por módulo.
- **Maquetas aprobadas (2026-10-18):** `f2-combate-v2.html` y `f2-mineria-v2.html` (sesión de diseño en `.superpowers/brainstorm/`). La descripción escrita es la referencia.

---

## 1. Intención

El propietario quiere que, mientras caza o mina, la segunda pantalla le diga **qué tiene delante, cómo está su nave, cuánto lleva ganado y qué hacer después** (dónde cobrar, dónde vender), sin mirar paneles del juego.

Decisiones del propietario en el diseño:

- Propuesta de paneles aceptada; **fuera de F2**: minería de superficie (Rhino) y las reglas de *Law & Disorder* (F9). **Dentro**: combate contra thargoides (AX).
- «Mi nave» con más peso visual: **pips como en el juego** (3 columnas de 4 unidades con medios pips), casco y calor en barras.
- Subsistema apuntado muy claro; aviso **LIMPIO · NO DISPARAR** / **BUSCADO** arriba a la derecha y muy visible.
- Minería más visual y completa (composición, núcleo, valor por material, veredicto, histórico, refinería).
- **Limitación aceptada (2026-10-18):** el juego no expone el calor en % ni el casco en tiempo real (investigado: diario, `Status.json`, CAPI; leer memoria u OCR del HUD descartados por términos de uso y presupuesto). El calor se muestra como **indicador de tres estados** y el casco como **barra por escalones** (último valor conocido). Si Frontier añade los campos, se sustituyen por barras reales.

## 2. Objetivos, no objetivos y antiobjetivos

**Objetivos**

1. **Objetivo actual** (`ShipTargeted`): nave, piloto, rango, facción, estado legal, recompensa en la jurisdicción, escudos y casco del objetivo, **subsistema apuntado** con su integridad; LIMPIO/BUSCADO destacado.
2. **Mi nave en combate**: escudos (arriba/abajo), casco por escalones, calor de tres estados con tiempo sobrecalentado, **pips** exactos, alertas (casco bajo, escudos caídos, sobrecalentamiento, bajo ataque, interdicción).
3. **Sesión de caza**: bajas, asistencias, créditos y créditos/hora, **vales sin cobrar** (en riesgo si mueres) por facción emisora con dónde cobrarlos y el coste en Interstellar Factors (−25 %), progreso del rango de combate.
4. **Zona de conflicto**: bando, bajas y bonos por facción.
5. **AX**: objetivo thargoide (tipo y casco según escaneo), bajas y bonos AX por tipo, aviso de **presencia thargoide** en el sistema.
6. **Prospector**: contenido, núcleo, composición, valor estimado por material, **veredicto** «merece la pena» según la lista de interés, restante, histórico de los últimos 8 asteroides.
7. **Sesión de minería**: toneladas refinadas por mineral, t/h, créditos estimados y créditos/hora, refinería por depósitos, limpets (lanzados por tipo y restantes), carga.
8. **Dónde vender**: mejores estaciones cercanas para la carga actual (precio, distancia, demanda, plataforma, antigüedad del dato) y venta estimada.

**No objetivos (F2)**: minería de superficie/Rhino; reglas de crimen de *Law & Disorder*; avisos de comandantes peligrosos (servicios de terceros); combate a pie; envío de datos a EDDN/Inara (F6); localizador de *hotspots* (posible en una fase posterior).

**Antiobjetivos**: romper el presupuesto (< 150 MB, < 1 % CPU en reposo, máx. 10 Hz) en combate intenso; mostrar como real lo estimado (calor, valores de minería, precios antiguos); que un servicio caído bloquee los paneles locales.

## 3. Experiencia de usuario

### 3.1 Disposiciones

- **«Combate»**: fila superior con Objetivo (grande) · Sesión de caza · Mi nave; fila inferior con los paneles AX (solo si hay actividad AX o presencia thargoide) y Zona de conflicto (solo en una ZC).
- **«Minería»**: Prospector (grande) · Sesión de minería · Dónde vender.
- Activación automática por relevancia (spec base §5.3): `combat` Active con objetivo fijado, `UnderAttack`, hardpoints desplegados en una ZC/RES o actividad AX reciente; `mining` Active al lanzar un prospector/recolector o con `ProspectedAsteroid` reciente.

### 3.2 Combate (según maqueta v2)

- **Objetivo**: cabecera con nave (nombre localizado), piloto, rango y facción; **insignia grande arriba a la derecha**: BUSCADO · <recompensa> (rojo) o LIMPIO · NO DISPARAR (verde) según `LegalStatus`/`Bounty` del último `ShipTargeted` (`ScanStage` ≥ 3); antes del escaneo completo, «escaneando…». Barras de escudos y casco del objetivo (datos de `ShipTargeted`). **Recuadro del subsistema** apuntado con nombre grande, icono y barra de integridad. Al perder el objetivo, el panel se vacía a los 10 s.
- **Mi nave**: escudos con indicador; **casco** barra por escalones (`Loadout.HullHealth`, `HullDamage.Health`, reparaciones); **calor**: estados *normal* / *sobrecalentado (>100 %)* (flag `OverHeating`, `HeatWarning`) / *daño por calor* (`HeatDamage`), con cronómetro de tiempo sobrecalentado; **pips** 3 columnas × 4 unidades con medios pips (`Status.Pips`, en medios pips 0–8); alertas.
- **Sesión de caza**: desde el inicio de sesión del juego (o desde que el usuario pulsa «reiniciar sesión»). Vales sin cobrar = `Bounty`/`FactionKillBond`/`CapShipBond` no canjeados (`RedeemVoucher`) desde la última muerte; reconstruidos desde el historial al arrancar. «Dónde cobrar»: estación de la facción emisora; si el sistema actual tiene un IF conocido (dato remoto opcional), se indica el coste del −25 %.
- **Zona de conflicto**: bando (del primer `FactionKillBond`/`Comms` disponible), bajas y bonos por facción.
- **AX**: objetivo thargoide (tipo según `Ship` de `ShipTargeted`), bonos AX (`FactionKillBond` con facción víctima thargoide), bajas por tipo, presencia thargoide a partir del bloque `ThargoidWar` de `FSDJump`/`Location` y de estados de facción thargoides. Lo que el **Spike AX** (§9) no confirme se omite sin bloquear el resto.

### 3.3 Minería (según maqueta v2)

- **Prospector**: del último `ProspectedAsteroid`: contenido (bajo/medio/alto), núcleo (`MotherlodeMaterial`), **barra de composición apilada**, tabla de materiales (nombre localizado, %, valor estimado, barra), **veredicto grande** («✔ merece la pena · ≈ X» / «✖ no compensa»), restante (`Remaining`), histórico de los 8 últimos asteroides por valor estimado.
- Valor estimado por material = `% × rendimiento típico × mejor precio conocido` (marcado «≈»; el rendimiento típico es un dato propio con procedencia y `verified`, ajustable con el historial: toneladas refinadas por asteroide).
- **Lista de interés** configurable (minerales que el usuario quiere; por defecto los de mayor valor) y **umbral** del veredicto (por defecto 1 000 000 cr estimados o núcleo de la lista).
- **Sesión de minería**: refinados (`MiningRefined`) por mineral, t/h, créditos estimados (precio de venta conocido) y créditos/hora, refinería por depósitos (estimada desde `MiningRefined` y carga), limpets (`LaunchDrone` por tipo; restantes desde `Cargo`), carga/capacidad.
- **Dónde vender**: para cada mineral de la carga, mejores mercados cercanos (servicio online, §5) con precio, distancia, demanda, plataforma compatible con la nave y antigüedad; venta estimada total en la mejor estación; fuente y antigüedad visibles.

## 4. Arquitectura

- Dos módulos nuevos en `crates/tmc-modules/src/` y `ui/src/modules/`: **`combat`** (objetivo, nave, sesión de caza, ZC, AX) y **`mining`** (prospector, sesión, dónde vender). Registro en `all_modules()` y en el registro de la UI.
- **`tmc-model`** tipa los eventos nuevos: `ShipTargeted` (nombre real del diario; tolerar la errata histórica si existe), `Bounty`, `FactionKillBond`, `CapShipBond`, `RedeemVoucher`, `ShieldState`, `HeatWarning`, `HeatDamage`, `EscapeInterdiction`, `PVPKill`, `FighterDestroyed`, `Promotion`, `ProspectedAsteroid`, `AsteroidCracked`, `MiningRefined`, `LaunchDrone`, `EjectCargo`, `MarketSell`, y el bloque `ThargoidWar` en `LocationInfo`. Eventos no tipados siguen yendo a `Other`.
- **`tmc-net`**: adaptador **Ardent** (`api.ardent-insight.com/v2`) para precios de venta cercanos; misma infraestructura de F1 (limitador, salud, reintentos, `api_cache`, opt-out, nunca en Legacy). Nuevo `NetQuery::CommodityImportsNearby { system_address, commodity, max_distance_ly, min_pad }`.
- **Datos estáticos propios** (`crates/tmc-modules/data/`): rendimiento típico por contenido de asteroide (`mining_yield.json`), nombres de subsistemas/naves para localización si el diario no trae `_Localised`; con procedencia y `verified` (mismas reglas de licencia que F1).
- Persistencia: tablas de módulo solo si hace falta (p. ej. histórico de asteroides de la sesión no se persiste; vales sin cobrar se **derivan** de la tabla `event`).

## 5. Servicios externos

| Servicio | Uso | Límite | Caché |
|---|---|---|---|
| **Ardent** | Mejores mercados de venta cercanos por mineral | Conservador: máx. 1 petición/s | 15 min por (sistema, mineral) |

- Solo se envían el `SystemAddress`/nombre del sistema actual y el nombre del mineral; nada del comandante.
- Respeta el ajuste «Consultar servicios online»; sin servicio, «Dónde vender» muestra «sin datos» y el resto funciona.
- Licencia: Ardent es AGPL-3.0 — **uso por red permitido**; sus respuestas solo van a `api_cache`, nunca al repositorio (fixtures sintéticos o recortes mínimos de datos públicos de mercado, según el spike).

## 6. Rendimiento y robustez

- En combate los eventos llegan en ráfagas: los topics se recalculan como mucho a 10 Hz y solo si cambian; el cronómetro de sobrecalentamiento se actualiza por el difusor (sin `requestAnimationFrame` continuo).
- Ningún evento raro/ausente tumba la app; objetivos sin escanear muestran «escaneando…».
- `measure-footprint.ps1` al cerrar la fase con las disposiciones «Combate» y «Minería» visibles.

## 7. i18n

Todas las etiquetas en `locales/{es,en,fr,de,pt}/{combat,mining}.ftl`; nombres de naves, subsistemas, minerales y facciones desde `_Localised` del diario, con respaldo propio cuando falte.

## 8. Criterios de aceptación de F2

1. Checks de Rust y UI en verde (fmt, clippy `-D warnings`, tests, `npm run check/test/build`). La CI de GitHub sigue deshabilitada por decisión del propietario; los checks se ejecutan en local.
2. Con un fixture de combate, el panel de objetivo muestra BUSCADO/LIMPIO correcto, el subsistema apuntado y escudos/casco del objetivo; p95 evento→UI < 500 ms.
3. «Mi nave»: pips exactos (incluidos medios pips), casco por escalones y los tres estados de calor según flags/eventos del fixture.
4. Vales sin cobrar = suma de `Bounty`/`FactionKillBond`/`CapShipBond` desde el último canje/muerte, igual a lo canjeado en `RedeemVoucher` del fixture (±0); se reconstruyen tras reiniciar.
5. Con un fixture de minería, el prospector muestra composición, núcleo y veredicto según la lista de interés; t/h y refinados coinciden con `MiningRefined` del fixture.
6. «Dónde vender» lista mercados de un servicio falso con precio, distancia, plataforma compatible y antigüedad; con servicios desactivados, «sin datos» sin errores.
7. AX: con un fixture AX (real o sintético según el spike), bajas/bonos AX por tipo y aviso de presencia thargoide.
8. Todas las cadenas nuevas en es/en/fr/de/pt (test de claves).
9. `measure-footprint.ps1` < 150 MB y < 1 % CPU en reposo con las disposiciones nuevas.
10. `THIRD_PARTY_NOTICES.md` y datos estáticos con procedencia; nada GPL/AGPL incorporado.

## 9. Riesgos y spikes previos

| Riesgo | Mitigación |
|---|---|
| Mecánicas AX en 2026 sin confirmar (eventos, facción del bono, tipos) | **Spike AX**: revisar diarios del propietario (solo agregados) y notas de actualización 4.x; documentar qué eventos y campos llegan. Lo no confirmado se omite. |
| API de Ardent (formas exactas, frescura, plataforma) | **Spike Ardent**: peticiones reales a `/nearby/imports`, forma de respuesta, campos de plataforma y fecha; documentar en `docs/research/`. |
| Rendimiento típico por asteroide sin fuente primaria | Dato propio `verified:false`, ajustado con el historial (`tmc-headless eval` ampliado). |
| El propietario no tiene sesiones de combate/minería en el historial local | Fixtures sintéticos anonimizados; validación con sesiones reales pendiente como en F1. |
| Ráfagas de eventos en combate | Recalcular por evento con dirty-only y difusor a 10 Hz; prueba de rendimiento. |

## 10. Decisiones del redactor (a revisar por el propietario)

Precisiones tras la revisión de un builder:

- **Vales en riesgo:** la muerte es el evento `Died`. `RedeemVoucher` resta por tipo (`bounty` / `CombatBond`) y por facción cuando el evento la detalla (`Factions[]`/`Faction`, con `BrokerPercentage` si se cobró en un IF); si no detalla facción, resta el importe de ese tipo empezando por el más antiguo. Los bonos de ZC son un tipo aparte de las recompensas y se muestran separados.
- **Dónde cobrar:** se indica el **sistema donde se ganó** cada vale (los vales se cobran en estaciones de la facción emisora) y el **Interstellar Factors más cercano** vía Ardent (`/v2/system/address/{id64}/nearest/interstellar-factors`, ya documentado en la investigación). Nuevo `NetQuery::NearestService { system_address, service }`; caché 1 h.
- **Spikes:** el plan se escribe con interfaces internas fijas y adaptadores que se ajustan al resultado de los spikes (mismo enfoque que F1); los spikes van primero.
- **Plataforma compatible:** tabla propia de tamaño de plataforma por nave (`ship_pads.json`, hechos del juego con procedencia y `verified`).
- **Precio de respaldo** para el valor estimado: último precio de venta conocido del historial local (`MarketSell` y `Market.json`); si no hay ninguno y no hay servicio online, el valor se muestra «sin datos» y el veredicto se decide por la lista de interés (núcleo o material de la lista con contenido alto).
- **Ajustes:** lista de interés, umbral del veredicto y radio de búsqueda de venta se guardan en `config.toml` como los ajustes de F1 (`PUT /api/config`, en caliente). «Reiniciar sesión» reinicia solo la sesión del módulo en el que se pulsa.

Decisiones iniciales:

1. Dos módulos (`combat` con AX dentro, `mining`).
2. Ardent como fuente de precios (no Spansh), caché 15 min.
3. Umbral del veredicto de minería: 1 000 000 cr estimados o núcleo en la lista de interés.
4. Sesión de caza/minería: desde el inicio de sesión del juego, con botón «reiniciar sesión».
5. El panel de objetivo se vacía 10 s después de perder el objetivo.
6. Paneles AX y Zona de conflicto solo visibles cuando aplican.

## 11. Ramas y entregas

- Rama `fase/f2-combate-mineria` desde `main` (con F1 fusionada).
- Bloques sugeridos: 0) spikes AX y Ardent; 1) eventos tipados + Ardent en `tmc-net`; 2) combate (objetivo, nave, sesión, ZC, AX) + UI; 3) minería (prospector, sesión, dónde vender) + UI; 4) disposiciones, ajustes, fixtures, eval y cierre.
