# TMC · Especificación de F3 — Comercio, misiones y pasajeros; recepción de EDDN

- **Estado:** 📝 borrador para aprobación del propietario
- **Fecha:** 2026-10-19
- **Depende de:** [`2026-10-17-tmc-arquitectura-y-f0-design.md`](2026-10-17-tmc-arquitectura-y-f0-design.md), F1 ([`2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md`](2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md): rutas Spansh, alcance de salto, portapapeles, `api_cache`, opt-out) y F2 ([`2026-10-18-tmc-f2-combate-mineria-design.md`](2026-10-18-tmc-f2-combate-mineria-design.md): Ardent, `PriceBook`, `ship_pads.json`, `Bounty` tipado, `core.clock`, disposiciones por relevancia).
- **Maquetas aprobadas (2026-10-19):** `f3-comercio.html` y `f3-misiones.html` (sesión de diseño en `.superpowers/brainstorm/`). La descripción escrita es la referencia.

---

## 1. Intención

TMC tiene que ser un **compañero útil** también fuera del combate: que el comerciante sepa qué comprar aquí y dónde venderlo sin abrir webs, que quien hace misiones no pierda ninguna por caducidad ni dé vueltas de más, y que el piloto de pasajeros sepa al instante si una misión de turismo compensa. Para eso F3 añade además una fuente de precios **fresca** (EDDN en recepción) que mejora también «Dónde vender» de minería.

## 2. Objetivos, no objetivos y antiobjetivos

**Objetivos**
- Disposición **«Comercio»**: mercado actual, mi carga, ruta comercial (Spansh), sesión de comercio y buscar mercancía.
- Disposición **«Misiones»**: misiones activas agrupadas por destino, pila de masacre, avisos, recompensas, pasajeros, turismo (ruta óptima y créditos/año luz) y objetivos comunitarios.
- **Recepción de EDDN** como fuente de precios en vivo, guardada en local.

**No objetivos (otras fases)**
- Enviar datos a EDDN/EDSM/Inara (F6). Powerplay, BGS, fleet carrier, colonización (F5). Law & Disorder (F9).
- Consultar el tablón de misiones antes de aceptarlas (el juego no lo escribe en el diario; ver §3.3).

**Antiobjetivos (sería un fracaso aunque funcione)**
- Que EDDN dispare el consumo: el límite < 150 MB y < 1 % CPU en reposo se mantiene.
- Mostrar precios viejos como si fueran actuales: siempre se indica la antigüedad, y en «Mercado actual» solo cuentan ventas vistas en las **últimas 48 h**.
- Perder una misión por un aviso que no llegó, o contar mal una pila de masacre.

## 3. Experiencia de usuario

### 3.1 Disposiciones y relevancia

- **«Comercio»** se activa sola al atracar en una estación con mercado o al llevar carga comercial (no de misión ni robada). **«Misiones»** se activa sola con misiones activas mientras estás atracado o en supercrucero.
- Prioridad de selección automática (ampliando F2): manual > combate activo/urgente > minería activa > misiones > comercio > exploración. La elección manual dura hasta que cambia el resultado automático (regla de F2).

### 3.2 Comercio (según maqueta)

- **Mercado actual** (al atracar, desde `Market.json`): las 5 mejores compras de la estación con su **mejor venta conocida en las últimas 48 h** — estación, distancia, plataforma compatible con tu nave, beneficio por tonelada y antigüedad del dato. Mercancías raras marcadas («RARA», con distancia de venta recomendada).
- **Mi carga**: por mercancía, toneladas, precio pagado (`MarketBuy`; media ponderada si compraste varias veces), precio aquí, mejor venta cercana compatible y beneficio total estimado, con sugerencia «VENDER AQUÍ» / «LLEVAR». Carga de misión y robada marcadas y fuera del cálculo de venta (la robada: «solo mercado negro»). Barra de bodega ocupada/capacidad.
- **Ruta comercial (Spansh)**: con capacidad de carga, créditos disponibles, plataforma y alcance de salto reales; N paradas «compra X en A → vende en B» con beneficio por tonelada, beneficio por parada, distancia, saltos y antigüedad de los datos; total de la ruta. **Avanza sola** al atracar en la siguiente parada y copia el siguiente sistema al portapapeles (como F1). Botón «Recalcular».
- **Sesión de comercio**: duración, beneficio y créditos/hora, toneladas compradas/vendidas, número de operaciones, mejor operación, progreso del rango de comercio; botón «reiniciar sesión» (como F2).
- **Buscar mercancía**: campo de texto con autocompletado de mercancías conocidas → dónde comprarla cerca (distancia, precio, oferta, plataforma compatible, antigüedad).

### 3.3 Misiones y pasajeros (según maqueta)

- **Misiones activas**: reconstruidas al arrancar con el evento `Missions` y los detalles de `MissionAccepted` del historial; actualizadas con `MissionAccepted/Completed/Failed/Abandoned/Redirected` y `CargoDepot`. **Agrupadas por destino** (sistema · estación), ordenadas por distancia desde tu posición, con la **siguiente parada** destacada y botón «a la ruta» (planificador de F1 / portapapeles). Por misión: tipo, facción, progreso (entregas, recogidas, bajas, pasajeros), caducidad con cuenta atrás en hora del diario y recompensa.
- **Pila de masacre**: por facción objetivo, barras por facción emisora (bajas hechas / requeridas, sumando sus misiones), bajas que faltan para completar toda la pila y recompensa total. Regla del juego: cada baja de la facción objetivo cuenta para **una** misión de **cada** facción emisora a la vez (la más antigua de esa facción).
- **Avisos**: caduca en < 1 h; redirigida (nuevo destino); fallida; carga de misión que falta para entregar.
- **Recompensas de la sesión**: créditos, misiones completadas/fallidas, materiales y reputación/influencia por facción.
- **Pasajeros**: cabinas por clase (económica, business, primera, lujo) con plazas ocupadas/totales, pasajeros a bordo por destino con marcas VIP y BUSCADO, y aviso visible si hay un pasajero buscado (riesgo en escaneos en zonas de seguridad).
- **Turismo — ruta óptima y rentabilidad (decisión del propietario: opción B)**: el juego no escribe el tablón en el diario, así que el cálculo se hace **automáticamente al aceptar** una misión de turismo (`MissionAccepted`): orden óptimo de las paradas desde tu posición (y regreso si la misión lo exige), comparado con el orden del tablón, distancia total, saltos estimados con tu alcance real, **créditos por año luz** y créditos/hora estimados, y un veredicto «RENTABLE / NO COMPENSA» frente a tu media de misiones de pasajeros. Si no compensa, el panel indica que puedes **abandonarla ahora**, mientras sigues atracado (con la penalización de reputación que aplique el juego).
- **Objetivos comunitarios** (`CommunityGoal`): tu aportación, nivel/percentil, tiempo restante y recompensa prevista.

## 4. Arquitectura

- Dos módulos nuevos en `crates/tmc-modules/src/` y `ui/src/modules/`: **`trade`** (mercado actual, carga, ruta, sesión, búsqueda) y **`missions`** (misiones, masacre, avisos, recompensas, pasajeros, turismo, objetivos comunitarios). Registro en `all_modules()` y en el registro de la UI.
- **Crate nuevo `tmc-eddn`**: suscriptor del relay de EDDN (ZeroMQ SUB, mensajes zlib + JSON), filtro por esquema (`commodity/3`) y por distancia a tu posición, y entrega de mercados al núcleo como `SourceEvent` nuevo. Se respeta la tabla de dependencias de la arquitectura (§3.1): `tmc-eddn` depende de `tmc-model`; lo compone `tmc-app`.
- **`tmc-store`**: tabla de **precios de mercado** (estación, mercancía, compra, venta, oferta, demanda, fecha, origen `local|eddn|ardent`) con limpieza de datos de más de 7 días. Alimentada por `Market.json` propio, EDDN y respuestas de Ardent. «Mercado actual» consulta solo lo de las últimas 48 h.
- **`tmc-model`** tipa: `Market`/`MarketBuy` (ya existe `MarketSell`), `Missions`, `MissionAccepted`, `MissionCompleted`, `MissionFailed`, `MissionAbandoned`, `MissionRedirected`, `CargoDepot`, `Passengers`, `CommunityGoal`, `Rank`/`Progress` (comercio) si faltan. Eventos no tipados siguen en `Other`.
- **`tmc-net`**: adaptadores **Spansh trade route** (job asíncrono como la ruta de F1) y **Ardent** para «dónde comprar» (exports cercanos) y mejores ventas por mercancía; coordenadas de sistemas para el turismo con la consulta de sistemas ya existente de F1. Misma infraestructura: limitador, salud, `api_cache`, opt-out, nunca en Legacy ni en replay.
- **Datos estáticos propios** (`crates/tmc-modules/data/`): mercancías raras (origen, distancia de venta recomendada), clases de cabina por módulo de pasajeros; con procedencia y `verified` (reglas de licencia de F1).
- Reutiliza: `PriceBook`/`rank_markets` y `ship_pads.json` de F2 (compatibilidad de plataforma), `core.clock` para cuentas atrás, alcance de salto de F1.

## 5. Servicios externos

| Servicio | Uso | Límite | Caché |
|---|---|---|---|
| **EDDN** (relay, solo recepción) | Mercados en vivo de estaciones cercanas | Flujo entrante; filtrado local | Tabla de precios (7 días) |
| **Spansh** | Ruta comercial | Como F1 (1 trabajo a la vez) | 15 min por parámetros |
| **Ardent** | Dónde comprar / mejores ventas cercanas | 1 petición/s (F2) | 15 min |

- No se envía nada del comandante: solo sistema/estación actuales, mercancía y parámetros de la nave (capacidad, créditos, alcance, plataforma) a Spansh. EDDN es solo recepción.
- Todo respeta «Consultar servicios online»; sin servicio, los paneles muestran «sin datos» con los precios locales que haya.
- Licencias: Spansh y Ardent por red (Ardent AGPL — uso por red permitido); datos de EDDN solo a la base local, nunca al repositorio. La biblioteca ZeroMQ elegida debe tener licencia compatible (ver spike).

## 6. Rendimiento y robustez

- **EDDN**: descomprimir y filtrar en un hilo aparte; descartar pronto lo que no es `commodity/3` o está lejos; escribir en SQLite por lotes; reconexión con retroceso si el relay cae. Si el spike muestra que no cabe en < 1 % CPU en reposo, solo se escucha mientras «Comercio» o «Minería» están activas.
- Topics a 10 Hz como máximo y solo si cambian; cuentas atrás por `core.clock` (sin `requestAnimationFrame`).
- Ningún mensaje corrupto de EDDN, evento raro o servicio caído tumba la app.

## 7. i18n

Etiquetas en `locales/{es,en,fr,de,pt}/{trade,missions}.ftl`; nombres de mercancías, facciones, tipos de misión y estaciones desde `_Localised` del diario, con respaldo propio. Rango de comercio en inglés (decisión del propietario para rangos).

## 8. Criterios de aceptación de F3

1. Checks de Rust y UI en verde (fmt, clippy `-D warnings`, tests, `npm run check/test/build`, e2e), en local.
2. Con un fixture de comercio y precios sintéticos, «Mercado actual» muestra las 5 mejores compras con su mejor venta de las últimas 48 h (una venta de hace 49 h no aparece) y plataforma compatible.
3. «Mi carga» calcula el precio pagado medio y el beneficio por mercancía igual al fixture; la carga de misión y la robada quedan fuera del cálculo.
4. Ruta comercial con un Spansh falso: paradas, beneficios y avance automático al atracar en la parada siguiente; portapapeles con el siguiente sistema.
5. Sesión de comercio: beneficio = Σ(`MarketSell`) − coste medio pagado de lo vendido (±0 en el fixture).
6. Misiones: con un fixture, la lista coincide con `Missions` tras reiniciar, agrupada por destino; redirigida, fallida y caducidad < 1 h generan aviso.
7. Masacre: con un fixture de 5 misiones de 3 facciones y bajas `Bounty`, las cuentas por facción y las bajas que faltan coinciden con el cálculo manual.
8. Turismo: al aceptar una misión de turismo del fixture, la ruta óptima es ≤ que el orden del tablón y los créditos/año luz coinciden con el cálculo manual (distancias de un servicio falso).
9. EDDN: con un relay falso, los mercados cercanos se guardan y aparecen en «Mercado actual»/«Dónde vender»; mensajes corruptos no rompen nada; con servicios desactivados no se conecta.
10. `measure-footprint.ps1` < 150 MB y < 1 % CPU en reposo con EDDN activo y las disposiciones nuevas.
11. Todas las cadenas nuevas en los 5 idiomas; `THIRD_PARTY_NOTICES.md` y datos estáticos con procedencia; nada GPL/AGPL incorporado.

## 9. Riesgos y spikes previos

| Riesgo | Mitigación |
|---|---|
| CPU/RAM de EDDN (volumen del flujo) | **Spike EDDN**: medir mensajes/s y CPU con filtrado; elegir biblioteca ZeroMQ (licencia y Windows). Plan B: escuchar solo con «Comercio»/«Minería» activas. |
| API de ruta comercial de Spansh (parámetros, job, forma) | **Spike Spansh trade**: peticiones reales, documentar en `docs/research/`. |
| Formato real de misiones de turismo, masacre y `CargoDepot` | **Spike misiones**: revisar diarios del propietario (solo agregados) y documentación; fixtures sintéticos para lo que no haya. |
| Distancias para turismo sin coordenadas locales | Coordenadas de los sistemas de la misión vía consulta de sistemas de F1 (Spansh/EDSM) y caché. |

## 10. Decisiones del redactor (a revisar por el propietario)

1. Dos módulos (`trade`, `missions`) y un crate `tmc-eddn` aparte.
2. «Mejor venta conocida» = máximo de la tabla de precios en 48 h (cualquier origen) con plataforma compatible; si no hay ninguna, «sin datos».
3. Turismo: orden óptimo por búsqueda exacta hasta 8 paradas (heurística por encima); «tu media» = créditos/año luz medios de tus misiones de pasajeros completadas en el historial (si no hay, se compara con 100 000 cr/al como referencia inicial, ajustable en ajustes).
4. Sesión de comercio desde el inicio de sesión del juego (`LoadGame`), con «reiniciar sesión» (como F2).
5. Radio de EDDN 200 al alrededor de tu posición, ajustable.
6. «Siguiente parada» = destino de misión más cercano; empate → la que caduca antes. Sin posición propia o sin coordenadas del destino, los grupos se ordenan por caducidad.

Precisiones tras la revisión de un builder:

7. **Paradas del turismo:** `MissionAccepted` de las misiones de turismo trae varios destinos en `DestinationSystem` separados por los marcadores `$MISSIONUTIL_MULTIPLE_…` del juego; el spike de misiones lo confirma con diarios reales o documentación. Si no vinieran, el panel muestra la misión sin cálculo de ruta, con el aviso «paradas no disponibles en el diario», y se anota como desviación (no se vuelve a la opción A sin el propietario).
8. **Veredicto del turismo:** se calcula al aceptar, se mantiene mientras la misión esté activa y se recalcula si la misión se redirige.
9. **«Tu media» de pasajeros:** el módulo guarda en su propia tabla, al aceptar cada misión de pasajeros, la recompensa y la distancia de la ruta calculada; la media sale de las completadas. No se hacen consultas masivas del historial antiguo; sin datos se usa la referencia de 100 000 cr/al.
10. **Pila de masacre — regla exacta:** cuenta solo `Bounty` (y bonos de ZC si la misión lo admite según el spike) cuya facción víctima sea la facción objetivo, ocurridos **después** de aceptar la misión. Cada baja se asigna, por cada facción emisora, a su misión activa más antigua con esa facción objetivo (por hora de aceptación; empate → `MissionID` menor) que aún no esté completa. Misiones con distinta facción objetivo son pilas distintas. `MissionRedirected` marca la misión como completa aunque el recuento local no cuadre (el juego manda).
11. **Plataforma desconocida:** las estaciones sin dato de plataforma se incluyen con aviso (`pad_ok` desconocido), como en F2; los fleet carriers se excluyen de «mejor venta» y de la ruta comercial.
12. **ZeroMQ:** preferencia por una biblioteca en Rust puro con licencia permisiva; se acepta libzmq (MPL-2.0) si la pura no funciona con el relay. Nada GPL/LGPL. EDDN solo ofrece ZeroMQ, no hay transporte alternativo.
13. **Plan B de EDDN (si el spike lo exige):** se conecta al activarse «Comercio» o «Minería» y se desconecta 60 s después de que dejen de estar activas; la tabla de precios persiste (7 días) entre conexiones. El criterio 10 se mide en el modo que resulte del spike.

## 11. Ramas y entregas

- Rama `fase/f3-comercio-misiones` desde `main` (con F2 fusionada).
- Bloques sugeridos: 0) spikes EDDN, Spansh trade y misiones; 1) eventos tipados, tabla de precios y `tmc-eddn`; 2) comercio + UI; 3) misiones, masacre, pasajeros y turismo + UI; 4) disposiciones, ajustes, fixtures, eval y cierre.
