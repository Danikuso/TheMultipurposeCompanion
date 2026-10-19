# Estado del desarrollo de TMC

> Lee primero [`AGENTS.md`](AGENTS.md). Este archivo indica el punto **exacto** del desarrollo. Actualízalo según las reglas de `AGENTS.md` §4.

- **Última actualización:** 2026-10-20 00:03 (Europe/Madrid) · Codex
- **Fase actual:** F3 — Comercio, misiones y pasajeros; recepción de EDDN (🔄 en ejecución). F2 ✅ fusionada en `main` (`e069a14`); F1 ✅ (`b96fecf`); F0 ✅ (`2b953e6`).
- **Spec activa:** [`docs/specs/2026-10-19-tmc-f3-comercio-misiones-design.md`](docs/specs/2026-10-19-tmc-f3-comercio-misiones-design.md) — ✅ aprobada (2026-10-19); base: [`2026-10-17-tmc-arquitectura-y-f0-design.md`](docs/specs/2026-10-17-tmc-arquitectura-y-f0-design.md)
- **Plan activo:** [`docs/plans/2026-10-19-tmc-f3-comercio-misiones.md`](docs/plans/2026-10-19-tmc-f3-comercio-misiones.md) — 🔄 17/35 hechas (aprobado al ordenar «escribe el plan y ejecútalo»). Anteriores: F2 ✅ 29/29, F1 ✅ 33/33, F0 ✅ 19/19
- **Rama de trabajo:** `fase/f3-comercio-misiones`

## Puntero actual

- **Tarea:** PRIORIDAD DEL PROPIETARIO: ampliar los logs de TODA la aplicación para diagnosticar pruebas y cuerpos ausentes en exploración. F3 Task 17 pausada con tests/stubs sin commit.
- **Siguiente acción:** concretar diseño/plan de instrumentación general sobre el subsistema `tracing` existente (arquitectura F0 §9) y la petición expresa «Añádelo ahora», e implementarlo sobre main estable para pruebas; después integrar el cambio en F3 y retomar Task 17. Checkout main reutilizable: `C:/Users/danik/.codex/worktrees/prueba-main/EDPyCompanion`, HEAD 9154d41, limpio. F3 HEAD 7b6671c; sin commit `trade/{route.rs,mod.rs}` y este status. `.claude/` y `.tmp/` locales ajenos. No incluir payloads privados, tokens ni chats ni enviar logs fuera del equipo.

## Fases

| Fase | Contenido | Estado |
|---|---|---|
| F0 | Cimientos (lectura, estado, SQLite, servidor, UI base, panel general, instalador) | ✅ completada y fusionada (2b953e6) |
| F1 | Navegación, exploración, exobiología | ✅ completada y fusionada (b96fecf) |
| F2 | Cazarrecompensas/combate (incl. AX), minería | ✅ completada y fusionada (e069a14) |
| F3 | Comercio, misiones y pasajeros, recepción EDDN | 🔄 en ejecución |
| F4 | Ingenieros y materiales | ⬜ |
| F5 | Fleet carrier, colonización, Powerplay 2.0, BGS, a pie | ⬜ |
| F6 | Cuentas: envío EDDN, EDSM, Inara, CAPI | ⬜ |
| F7 | Voz (KokoroTTS; alemán sin voz) | ⬜ |
| F8 | Overlay ligero e integrado | ⬜ |
| F9 | Crimen y piratería (Law & Disorder, dic. 2026) | ⬜ |
| F10 | Atajos, Stream Deck, VoiceAttack, build Linux | ⬜ |

## Tareas de F3

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 0 | Spike EDDN | ✅ hecha | 26d7da8 | EDDN OnDemand (0,38 % de un núcleo); zeromq MIT; carriers; coords 91,8 % |
| 1 | Spike red — Spansh trade, Ardent exports, coordenadas | ✅ hecha | 1786893 | Spansh trade (requires_large_pad), Ardent exports, coords Ardent+EDSM |
| 2 | Spike misiones | ✅ hecha | 61f06c6 | Turismo verificado; masacre; cabinas agregadas |
| 3 | Aprobaciones del propietario tras los spikes | ✅ hecha | (este commit) | Resultados de spikes como rulings del controlador en Desviaciones |
| 4 | `tmc-model` — eventos de comercio | ✅ hecha | 299fc8c, ad46673 | Eventos de comercio tolerantes; max_pad, has_market |
| 5 | `tmc-model` — eventos de misiones, pasajeros y objetivos comunitarios | ✅ hecha | 0463c11, ad46673 | Eventos de misiones/pasajeros/CG tolerantes; destinos múltiples |
| 6 | `tmc-model::market` y `tmc-store` — tabla de precios, estaciones y coordenadas | ✅ hecha | b510911, c1214e2 | Tablas de precios/estaciones/coords; marcas de tiempo explícitas; retención 7/30 d |
| 7 | `tmc-model` — consultas de red nuevas y ajustes de F3 | ✅ hecha | b89c415 | NetQuery/NetOutcome nuevas y ajustes eddn_radius_ly/tourism_reference_cr_ly |
| 8 | `tmc-net` — ruta comercial de Spansh y coordenadas por nombre | ✅ hecha | 84131b6, 5580336 | Spansh trade (requires_large_pad), coords Ardent→EDSM, filas parciales |
| 9 | `tmc-net` — exportaciones cercanas de Ardent | ✅ hecha | 84131b6, 5580336 | Ardent exports; regla de carriers común (también en F2) |
| 10 | `tmc-eddn` — decodificación y filtro | ✅ hecha | 9d058d5, 63feee3 | tmc-eddn: decodificación acotada (1 MiB), prefiltro, carriers en decode, índice de coords |
| 11 | `tmc-eddn` — suscriptor, reconexión y lotes | ✅ hecha | 273f12b, ea0616f | Suscriptor EDDN: OnDemand, retroceso, vigilancia 60 s, tope 256 KiB, lotes sin bloqueo; PUB/SUB probado en Windows |
| 12 | `tmc-app` — servicio EDDN, `SourceEvent::Eddn`, limpieza y configuración | ✅ hecha | c7840f5, 3334f69 | Replay explícito; recuperación tras lag; coords tras importar y offline sin congelación; spec/calidad aprobadas; 874 Rust y 144 UI PASS |
| 13 | datos estáticos `rare_goods.json` y `cabins.json` | ✅ hecha | 440c5d9 | OnceLock, parser tolerante con avisos y recuperación por fila; catálogo inicial 1 rara/4 cabinas; spec/calidad aprobadas; 878 Rust PASS |
| 14 | i18n — `trade.ftl` y `missions.ftl` | ✅ hecha | 7f6a9a9 | 125 claves de paneles por idioma y ajustes/disposiciones; rangos ingleses; spec/calidad aprobadas; 880 Rust y 144 UI PASS |
| 15 | `trade` — coste pagado y «Mi carga» | ✅ hecha | 28e205b | Media ponderada, coste parcial, exclusión misión/robada, SRV protegido y reconstrucción paginada; spec/calidad aprobadas; 893 Rust PASS |
| 16 | `trade` — «Mercado actual» y mejor venta en 48 h | ✅ hecha | 3a33097 | Top5, mezcla fresca, consultas acotadas compatibles y marcador de catálogo completo; spec/calidad aprobadas, 924 Rust PASS |
| 17 | `trade` — ruta comercial con avance automático | 🔄 en curso | — | PAUSADA por prioridad de logs del propietario; route.rs con tests/stubs y mod.rs sin commit; RED aún sin confirmar |
| 18 | `trade` — sesión de comercio y buscar mercancía | ⬜ pendiente | — |  |
| 19 | `trade` — módulo, topics, relevancia, red, almacén y registro | ⬜ pendiente | — |  |
| 20 | `mining` — «Dónde vender» con la tabla de precios | ⬜ pendiente | — |  |
| 21 | UI — «Mercado actual» y «Mi carga» | ⬜ pendiente | — |  |
| 22 | UI — «Ruta comercial», «Sesión» y «Buscar mercancía» | ⬜ pendiente | — |  |
| 23 | `missions` — libro de misiones y reconstrucción | ⬜ pendiente | — |  |
| 24 | `missions` — pila de masacre | ⬜ pendiente | — |  |
| 25 | `missions` — avisos, recompensas de la sesión y objetivos comunitarios | ⬜ pendiente | — |  |
| 26 | `missions` — pasajeros y cabinas | ⬜ pendiente | — |  |
| 27 | `missions` — turismo: ruta óptima, cr/al y veredicto | ⬜ pendiente | — |  |
| 28 | `missions` — módulo, coordenadas, topics, relevancia y registro | ⬜ pendiente | — |  |
| 29 | UI — «Misiones activas», «Masacre» y «Avisos» | ⬜ pendiente | — |  |
| 30 | UI — «Pasajeros», «Turismo», «Recompensas» y «Objetivos comunitarios» | ⬜ pendiente | — |  |
| 31 | UI — disposiciones «Comercio» y «Misiones», prioridad y ajustes | ⬜ pendiente | — |  |
| 32 | fixtures sintéticos y pruebas de extremo a extremo | ⬜ pendiente | — |  |
| 33 | `tmc-headless eval` — conciliación de coste pagado y de masacre | ⬜ pendiente | — |  |
| 34 | avisos, consumo con EDDN, verificación de los 11 criterios y cierre | ⬜ pendiente | — |  |

## Tareas de F2

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 0 | Spike AX | ✅ hecha | a2276dd | Sin datos AX en los 7 diarios; AX desde fuentes secundarias (sin verificar); BrokerPercentage: Amount tal cual; sin asistencias. Revisión aprobada. |
| 1 | Spike Ardent | ✅ hecha | e9937cd | Rutas por address OK; filtro de plataforma local; nombres = Type del diario; fixtures sintéticos. |
| 2 | Aprobaciones del propietario tras los spikes | ✅ hecha | (este commit) | Registradas en Bloqueos; no bloquean. |
| 3 | `tmc-model` — eventos de combate | ✅ hecha | ecce185 | ShipTargeted (alias ShipTargetted), Bounty, bonos, RedeemVoucher, calor, escudos… Revisión aprobada. |
| 4 | `tmc-model` — eventos de minería y mercado, `Cargo` con inventario, reparaciones | ✅ hecha | ac3eacb | ProspectedAsteroid, MiningRefined, LaunchDrone, MarketSell, Cargo con inventario, reparaciones. Revisión aprobada. |
| 5 | `tmc-model` — bloque `ThargoidWar` y facciones en `LocationInfo` | ✅ hecha | 99d0f19, a76ba18 | Bloque ThargoidWar y facciones tolerantes a tipos inesperados; thargoid_presence. Revisión aprobada. |
| 6 | `tmc-model` — consultas de Ardent, `PadSize` y ajustes de minería | ✅ hecha | eac8347 | NetQuery CommodityImportsNearby/NearestService (con min_pad), NetSource::Ardent, PadSize, ajustes de minería. Revisión aprobada. |
| 7 | `tmc-net` — adaptador Ardent | ✅ hecha | 934b660 | Adaptador Ardent (mercados cercanos con filtro local de plataforma, fecha, obras y carriers; IF más cercano), límite 1/s, caché 15 min/1 h. Revisión aprobada. |
| 8 | configuración nueva en `config.toml` y `PUT /api/config` | ✅ hecha | d022ecd | Ajustes de minería en config.toml con validación y en caliente; caché de Ardent en NetService. Revisión aprobada. |
| 9 | datos estáticos `ship_pads.json` y `mining_yield.json` | ✅ hecha | cad0c42, d3529c5 | ship_pads.json (47 naves, alias, tamaño conservador si hay duda) y mining_yield.json, todo verified:false hasta eval. Revisión aprobada tras 1 ronda. |
| 10 | i18n — `combat.ftl` y `mining.ftl` | ✅ hecha | 740fa37, 9e05655, a89d18d | combat.ftl y mining.ftl en 5 idiomas (63 claves), plurales correctos en fr/pt. Revisión aprobada tras 1 ronda. |
| 11 | `combat` — objetivo actual | ✅ hecha | ecd1a8f, a1ef396 | Seguimiento del objetivo: estado legal (BUSCADO/LIMPIO/…), escaneando, subsistema, vaciado a los 10 s; constante thargoide no verificada. Revisión aprobada. |
| 12 | `combat` — mi nave | ✅ hecha | 7d87eea, 872f9d6 | Escudos, casco por escalones (Hull/All/RepairAll), calor de 3 estados con cronómetro, pips en medios pips, alertas. Revisión aprobada tras 1 ronda. |
| 13 | `combat` — libro de vales | ✅ hecha | 78e6e49, 46ce565 | Libro de vales: recompensas y bonos por facción, canje por tipo/facción con resto a vales sin facción, muerte a cero, sistema de origen. Revisión aprobada tras 1 ronda. |
| 14 | `combat` — sesión de caza y reconstrucción | ✅ hecha | 0f3368c, 7e7ecdf | Sesión de caza (bajas, créditos/h, rango) y reconstrucción de vales desde el historial; paginador de historial compartido con F1; sin asistencias. Revisión aprobada tras 1 ronda. |
| 15 | `combat` — zona de conflicto y AX | ✅ hecha | 4934f4f, 6b9f97a | Zona de conflicto (vista por zona) y AX (tipos tolerantes, objetivo pendiente que caduca, presencia thargoide), todo AX sin verificar. Revisión aprobada. |
| 16 | `combat` — módulo, topics, relevancia, red y registro | ✅ hecha | a0c1c19, 8c74cae | Módulo Combat registrado: 5 topics, relevancia, IF más cercano (Ardent), reconciliación exacta del libro, render solo cuando cambia. Revisión aprobada tras 1 ronda. |
| 17 | UI — paneles «Objetivo» y «Mi nave» | ✅ hecha | f8e0a72, 1171e5d | Paneles Objetivo (insignia BUSCADO/LIMPIO, subsistema destacado) y Mi nave (casco por escalones, calor de 3 estados con cronómetro, pips 3×4), accesibles; 103 tests UI. Revisión aprobada tras 1 ronda. |
| 18 | UI — «Sesión de caza», «Zona de conflicto» y «AX» | ✅ hecha | d42a6d9, 8a4567d | Paneles Sesión de caza, Zona de conflicto y AX (visibles solo cuando aplican), tipos AX localizados. Revisión aprobada tras 1 ronda. Nombres de rangos en es/de/pt por confirmar con el juego. |
| 19 | `mining` — prospector | ✅ hecha | 2274422, cb9cfeb | Prospector; fix precios parciales y sin dedup; 364 tests tmc-modules |
| 20 | `mining` — sesión de minería | ✅ hecha | 73915e4, df3e786 | Sesión de minería; Cargo sin inventario conserva limpets |
| 21 | `mining` — `PriceBook` y «Dónde vender» | ✅ hecha | 570cdcb, 99365d2 | PriceBook y Dónde vender; Market.json soportado; clear_remote al cambiar nave |
| 22 | `mining` — módulo, topics, relevancia, red y registro | ✅ hecha | 846516e, 2c98f49 | Módulo de minería; clave con plataforma; reinicio por cmdr/LoadGame; 408 tests |
| 23 | UI — panel «Prospector» | ✅ hecha | 3c8e204, 650bfc6 | Panel prospector |
| 24 | UI — «Sesión de minería» y «Dónde vender» | ✅ hecha | 928ad55, 650bfc6 | Sesión y Dónde vender; hora del diario (as_of), nombres localizados |
| 25 | UI — disposiciones «Combate» y «Minería», relevancia y ajustes | ✅ hecha | f0c843a, 38568cf | Disposiciones Combate/Minería, vuelta a auto, ajustes de minería |
| 26 | fixtures sintéticos y pruebas de extremo a extremo | ✅ hecha | 05663ac, b50630b | Fixtures sintéticos + e2e (Playwright 5/5); ráfaga p95 ~311 ms |
| 27 | `tmc-headless eval` — vales y rendimiento de asteroides | ✅ hecha | 996fae3, 81352c0, 606fcc1 | Eval vales/rendimiento; historial propietario: vales INSUFICIENTE (canje incluye vales previos al primer diario), minería sin datos |
| 28 | avisos, consumo, verificación de los 10 criterios y cierre | ✅ hecha | 9b2f5bf, fdcbfca, c54a541, cb484d8 | 10/10 criterios PASS (750+ tests Rust, 144 vitest, 5 e2e). Footprint con ventana: combate 112,7 MB, minería 111,5 MB, CPU en reposo 0 %. Revisión final de rama: «Ready to merge» tras corregir acciones reset de combate/minería (no llegaban al núcleo) y reinicio por comandante. |

## Tareas de F1

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 0 | Spike 1 — Spansh | ✅ hecha | 886c597 | Ficha `/api/system/{id64}`, neutrones `/api/route`, galaxia `/api/generic/route`, autocompletado; política/Powerplay desde Spansh; «descubierto» vía EDSM. Revisión aprobada. |
| 1 | Spike 2 — licencias FSD y especies | ✅ hecha | bdaea77, 6ecb663 | Todos los datos estáticos se construyen desde hechos con procedencia y `verified`; ninguna fuente externa incorporable; Canonn solo por red. Revisión aprobada tras 1 ronda. |
| 2 | Spike 3 — Canonn | ✅ hecha | 722500c, 6297f2f | Viable técnicamente; permiso formal pendiente → capa 3 desactivada por ahora. Revisión aprobada tras 1 ronda. |
| 3 | Aprobaciones previas del propietario | ✅ hecha | (este commit) | Bloqueos registrados (Canonn, nombres de especies). |
| 4 | `tmc-model` — eventos de exploración y exobiología | ✅ hecha | 4eec257 | 48 tests tmc-model; workspace, fmt y clippy PASS; fixture `exploration_session.log`. Revisión aprobada. |
| 5 | `tmc-model` — Loadout con ingeniería, política, superficie | ✅ hecha | c272ae9 | 55 tests tmc-model; snapshots F0 intactos; `LocationInfo` en `Box`. Revisión aprobada. |
| 6 | `tmc-model` — tipos de red, `ModuleSettings`, nuevas `SourceEvent` | ✅ hecha | ae32e35 | 58 tests tmc-model; `PlannedHop` con `jumps` y `refuel: Option`. Revisión aprobada. |
| 7 | `tmc-core` — peticiones de red, tareas de SQLite, migraciones | ✅ hecha | 69ec17a, 2fe3b63 | API pública de SQL y `api_cache` en tmc-store; worker FIFO único para StoreTasks; 27 core / 21 store PASS. Revisión aprobada tras 1 ronda. |
| 8 | i18n — varios `.ftl` por idioma | ✅ hecha | 9649357, adb1d19 | Rust y UI cargan todos los `*.ftl`; degradación por archivo en ejecución; 12 i18n + 11 Vitest PASS. Revisión aprobada tras 1 ronda. |
| 9 | `tmc-net` — limitador, salud y política de caché | ✅ hecha | 407de8d | Crate nuevo (solo depende de tmc-model); 7 tests deterministas. Revisión aprobada. |
| 10 | `tmc-net` — clientes Spansh/EDSM/Canonn y `NetClient` | ✅ hecha | f87f9fb, 1303436 | reqwest 0.13 + rustls; licencias permisivas (incl. Unicode-3.0) en THIRD_PARTY_NOTICES; Canonn devuelve `Disabled`; autocompletado con id64; 56 tmc-net / 245 workspace PASS. Revisión aprobada tras 1 ronda. |
| 11 | `tmc-app` — `NetService`, configuración y ajustes en caliente | ✅ hecha | 9e8f009, abaa34e | Caché ficha 24 h / política 1 h, agrupación, reintentos con retroceso, semáforo de 4, opt-out revalidado; 21 tmc-app PASS. Revisión aprobada tras 1 ronda. |
| 12 | `tmc-server` — acciones con argumentos y autocompletado | ✅ hecha | 9960190, 61ef707 | `POST /api/actions/{name}` con JSON, WS `action` con `args`, `GET /api/navigation/autocomplete` paginado (20) con `{name,id64}`; seguridad LAN aplicada; 12 Vitest PASS. Revisión aprobada. **Seguridad:** protección CSRF en peticiones que cambian estado (Origin/Sec-Fetch-Site, JSON obligatorio), fallo preexistente de F0 detectado por análisis automático. |
| 13 | `navigation` — alcance de salto y combustible | ✅ hecha | 1ef1621, cf191fa | `fsd.json` propio (procedencia, `verified:false`), validación contra MaxJumpRange con respaldo escalado, `star_flags` explícito; 30 tmc-modules PASS. Revisión aprobada tras 1 ronda. |
| 14 | `navigation` — ruta activa y reglas de avance | ✅ hecha | 0ec3604, 6a7361c | Avance con saltos omitidos, retroceso a hops anteriores, fuera de ruta sin descartar plan, prioridad plan > NavRoute > actual; 300 workspace PASS. Revisión aprobada tras 1 ronda. |
| 15 | `navigation` — tira de ruta (ficha local + remota) | ✅ hecha | 8393242, 76b4c38 | Módulo `Navigation` registrado; topics `navigation.route` y `navigation.jump`; peticiones con caducidad y reintento; visitas desde el historial; 30 claves `strip-*` en 5 idiomas; 74 tmc-modules PASS. Revisión aprobada tras 1 ronda. |
| 16 | `navigation` — planificador Spansh y persistencia | ✅ hecha | aea8686, ec87d82 | Acciones plan/accept/clear/recalculate, topic `navigation.plan` (resumen ≤ 50 saltos), tabla `nav_route_plan` por comandante, validación de argumentos; 78 navigation PASS. Revisión aprobada tras 1 ronda. |
| — | Arreglo CI Linux (`apps/desktop`) | ✅ hecha | f68abb0 | Import solo de Windows bajo `cfg`; el fallo venía de F0 y **sigue en `main`** hasta fusionar F1. |
| 17 | `tmc-app` — copia automática y manual desde la ruta activa | ✅ hecha | 0b60442, 9d66ac4 | Navigation decide copia y aviso (`nav-auto-copied`) solo en saltos en vivo con el ajuste activo; ajuste en caliente vía Settings; portapapeles compartido con respaldo en memoria; 27 tmc-app PASS. Revisión aprobada tras 1 ronda. |
| 18 | UI — disposiciones, ancho completo, tipografía y tira de ruta | ✅ hecha | 468f00b, a3d53ed | `RouteStrip`/`SystemCard`/`JumpPanel` según maqueta aprobada; fuente 17/24 px ≥ 2400 px; números con Intl según idioma; tira adaptable; aviso de peligro no solo por color; 31 tests UI PASS. Revisión aprobada tras 1 ronda. |
| 18b | Ajuste de la tira (cabecera «Siguiente salto») | ✅ hecha | 9815cce | Verificado en una demo en vivo con datos reales de Spansh. |
| 19 | UI — panel del planificador | ✅ hecha | 34d3b9c, a15b5ee | Formulario, autocompletado con id64, eficiencia por defecto de Ajustes, resultado y aceptar/borrar; 46 tests UI PASS. Revisión aprobada tras 1 ronda. Validación visual del propietario: ✅ aprobada (2026-10-18). |
| 20 | `exploration` — valor de cuerpos y sondas | ✅ hecha | fe26c90, 123b335, 7cfb305 | Fórmulas propias (FSS, mapeo con eficiencia, primeros, Odyssey), gigantes gaseosos mapeables, nombres reales del diario, `probes.json` con `verified:false`; 18 exploration PASS. Revisión aprobada tras 1 ronda. |
| 21 | `exploration` — cuerpos del sistema, FSS y consejo | ✅ hecha | 9940901, c25b238, 3984224 | P2 de payload y Terraforming corregidos; marks compacto documentado para Task 23; 392 Rust/46 UI PASS, revisión aprobada. |
| 22 | `exploration` — datos sin vender | ✅ hecha | ffa3433 | Libro compartido, reconstrucción cronológica paginada y conciliación/retry; 413 Rust/46 UI PASS, revisión aprobada. |
| 23 | UI — cuerpos del sistema | ✅ hecha | b669a77 | Tabla virtualizada ≤30 filas, cinco idiomas, visual 2560/1366 px; 413 Rust/56 UI PASS, revisión aprobada. |
| 24 | Exobiología — especies y geodesia | ✅ hecha | 458fc84, c1857be | Geodesia + tabla propia de 21 géneros/117 especies (CCR y valores, `verified:false`, a verificar con eval). Revisión aprobada. |
| 25 | `exobiology` — predicción (reglas + historial + Canonn) | ✅ hecha | 73261c4, 2f44b17 | 90 reglas propias (15 géneros Odyssey) con procedencia por género y nota `docs/research/notes/bio_rules_f1.md`; historial en SQL (200k Scan ≈ 0,4 s); Canonn soportado pero desactivado. Revisión aprobada tras 1 ronda. |
| 26 | `exobiology` — muestreo y `exo_sample` | ✅ hecha | 5225eb9, e13064a | Máquina Log→Sample→Analyse, posiciones persistidas y reanudables, distancia y rumbo al punto previo más cercano, replay sin borrados. Revisión aprobada tras 1 ronda. |
| 27 | `exobiology` — módulo, bonus ×5, bio sin vender y relevancia | ✅ hecha | 0d228b4 | Módulo `Exobiology` registrado (4 módulos); topics body/sampling/unsold; ×5 estimado; reconstrucción idempotente al arrancar; 236 tmc-modules PASS. Revisión aprobada. |
| 28 | UI — panel de exobiología | ✅ hecha | 9578414, 1b824dd | Candidatos con probabilidad y origen, muestreo con barra/dirección/sin vender, tipo y atmósfera localizados; 69 tests UI PASS. Revisión aprobada tras 1 ronda. |
| 29 | `tmc-headless eval` (criterios 3, 4 y 6) | ✅ hecha | 98f93e9, cdc921e, d16ebb0, e734e5f | Eval local con solo agregados. Tus 7 diarios: C3 15/15 loadouts dentro de ±0,05 al pero INSUFICIENTE (sin ingeniería ni Guardian); C4 y C6 INSUFICIENTE (sin ventas cubiertas ni exobiología). Corrección de §5.1 (ver Bloqueos). |
| 30 | Fixtures anonimizados y pruebas de extremo a extremo | ✅ hecha | 1ee4697 | `real_explo_01.log` (87 líneas, anonimizado y revisado); tira p95 310,8 ms con juego en marcha; exobiología e2e con fixture sintético (sin datos reales). Revisión aprobada. |
| 31 | Disposición Exploración y ajustes F1 | ✅ hecha | 7878cdf | 515 Rust/81 UI, fmt/clippy/check/build y smoke real PASS; 3 P2 corregidos, revisión aprobada. |
| 32 | Consumo y cierre F1 | ✅ hecha | a87c3a0 | `measure-footprint.ps1 -ShowWindow` (`max_mb=108.01` < 150, `avg_cpu_idle=0.00%` < 1% PASS), instalador NSIS 5,79 MiB, 526 Rust / 89 UI / E2E PASS y 10/10 criterios documentados. |

## Tareas de F0

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 0 | Entorno de desarrollo (Build Tools, rustup, Tauri CLI) | ✅ | — | VS BuildTools 17.14, rustc 1.99.1 MSVC, tauri-cli 2.12.3 |
| 1 | Workspace, licencia y CI | ✅ | 69b4be8, 83201c9 | job `ui` de CI añadido en Task 15 |
| 2 | `tmc-model` — Status.json | ✅ | 65c72d9 | |
| 3 | `tmc-model` — eventos del diario y archivos compañeros | ✅ | d24e813 | |
| 4 | `tmc-model` — GameState y SourceEvent | ✅ hecha | 70e4999, 177eac1, 2ec0abe | Revisión aprobada; 35 tests PASS, fmt y clippy del workspace PASS. |
| 5 | `tmc-journal` — localizar carpeta y listar diarios | ✅ hecha | e8e6cc8 | TDD 9 FAIL → 9 PASS; revisión aprobada; workspace 44 tests, fmt y clippy PASS. |
| 6 | `tmc-journal` — lector incremental de líneas | ✅ hecha | 7634de8 | RED 11 FAIL → GREEN 20 journal PASS; revisión y checks workspace PASS. |
| 7 | `tmc-journal` — Status, compañeros, proceso, HUD y watcher | ✅ hecha | 2cc55b7 | Revisión final aprobada; 37 journal y 72 workspace PASS, fmt/clippy PASS. |
| 8 | `tmc-store` — SQLite, escritor e importación | ✅ hecha | 9fb90b7 | 18 tests store y 90 workspace PASS; revisión, fmt y clippy aprobados. |
| 9 | `tmc-core` — actor, módulos, topics y alertas | ✅ hecha | 5565fbb | Revisión aprobada; 17 core/107 workspace PASS, fmt/clippy PASS. |
| 10 | `tmc-i18n` y traducciones | ✅ hecha | f56862d | 67 claves/5 idiomas; 8 i18n y 115 workspace PASS, revisión y checks aprobados. |
| 11 | `tmc-modules` — registro y `dashboard` | ✅ hecha | 0ee2585 | RED 11 FAIL → GREEN 12 modules y 127 workspace PASS, revisión y checks aprobados. |
| 12 | `tmc-server` — HTTP, WS, seguridad, emparejamiento | ✅ hecha | d1e12be | RED 10 FAIL → GREEN 12 server y 139 workspace PASS, revisión y checks aprobados. |
| 13 | `tmc-app` + `apps/headless` | ✅ hecha | 31b439f | RED 6 FAIL → GREEN 6 app y 145 workspace PASS, `tmc-headless` + `/api/health` verificados. |
| 14 | `tmc-replay` — reproducción y anonimización | ✅ hecha | 885b0a2 | RED 3 FAIL → GREEN 3 replay, fixtures reales anonimizados, snapshot `insta` y 149 workspace PASS. |
| 15 | UI — base (WS, i18n, tema, shell) | ✅ hecha | bd78913 | RED 6 FAIL → GREEN 6 tests UI, `svelte-check` 0 errores, `vite build` OK y job `ui` en CI. |
| 16 | UI — panel general y ajustes | ✅ hecha | 8d26c0b | RED 4 FAIL → GREEN 10 tests UI, `smoke.spec.ts` Playwright PASS, `ConfigHost` y 150 tests Rust PASS. |
| 17 | `apps/desktop` — Tauri, bandeja, instalador | ✅ hecha | db4507d | `TMC.exe` y `TMC_0.1.0_x64-setup.exe` generados y verificados (bandeja, WebView2, instancia única). |
| 18 | Presupuesto de consumo y cierre de F0 | ✅ hecha | fe99cd6 | `measure-footprint.ps1` (`max_mb=144.91` < 150, `avg_cpu_idle=0.09%` < 1% PASS), `README.md` y 9/9 criterios §15 verificados. |

## Bloqueos y pendientes del propietario

- [x] **F3 · Task 16:** corregidos cierres SQL, mercancías ausentes/catálogos vacíos y compatibilidad antes del límite. Consulta dirigida acotada, marcador completo con migración aditiva y purga. APIs y Tasks 18/19/20 sincronizadas en el plan. Revisiones spec/calidad aprobadas; 924 Rust PASS, fmt/clippy OK.

- [x] **F3 · Task 12:** corregidos helper de siembra tras importación, guardia replay explícita, recuperación tras lag y bloqueo de coordenadas por importación incompleta. Dos expectativas UI heredadas alineadas con rangos ingleses aprobados. Revisiones spec/calidad aprobadas y checks completos correctos (874 Rust/144 UI).
- [x] **F2 · Criterio 9 (consumo con ventana):** ✅ medido 2026-10-19 (112,7 MB / 0 % CPU). Antes: cierra tu TMC.exe (PID 3808 ocupa 47320 y la instancia única impide otra) y ejecuta `pwsh scripts/measure-footprint.ps1 -Exe target-f2/release/TMC.exe -JournalDir .tmp/footprint-f2 -FixturePath tests/fixtures/journal/combat_session.log -ShowWindow -Seconds 120` y lo mismo con `mining_session.log`; o autoriza al agente a cerrarlo y medir. Límite < 150 MB y < 1 % CPU.
- [ ] **F2 · Eval sobre tu historial:** vales INSUFICIENTE (el único canje incluye vales ganados antes del primer diario) y minería sin datos (n=0). `tmc-headless eval` queda listo para validar con futuras sesiones de caza/minería.

- [x] **F2 · Nombres de rangos de combate:** decidido 2026-10-19 → se mantienen **en inglés** en todos los idiomas, como en el juego (`4e75386`).

- [ ] **F2 · Resultados de los spikes (para tu visto bueno; F2 sigue avanzando):**
  - **AX:** tus 7 diarios no tienen ningún dato AX. Los paneles AX se construyen con nombres de eventos y naves de fuentes secundarias, marcados como no verificados, y se prueban con datos sintéticos; se ajustarán cuando juegues una sesión AX.
  - **Asistencias:** no aparecen en tus diarios (`SharedWithOthers`), así que cada recompensa cuenta como baja completa.
  - **Vales cobrados en Interstellar Factors:** se usa el importe del diario tal cual; el % del broker se muestra como información.
  - **Ardent:** valores por defecto de «Dónde vender»: radio 100 al, volumen mínimo 100 t, datos de hasta 3 días, sin carriers; el filtro de plataforma se hace en TMC (Ardent no lo aplica en esa consulta). Se excluyen asentamientos a pie.

- [x] **F1 · Corrección de spec §5.1:** propietario aprueba explícitamente el 2026-10-18 validar MaxJumpRange con masa en vacío más combustible máximo por salto (no depósito lleno). Spec actualizada; evidencia de 15 loadouts en Task 29. Código ya alineado.
- [x] **F1 · Cierre con el historial local actual (7 diarios):** propietario ordena finalizar F1 (2026-10-18). Los criterios 3, 4 y 6 pasan en fixtures y los 15/15 loadouts locales coinciden (±0,0023 al); `tmc-headless eval` queda disponible para validar futuras sesiones con FSD de ingeniería/Guardian, ventas de exploración cubiertas y exobiología.
- [x] **Sincronización de F1:** propietario autoriza explícitamente push de `fase/f1-navegacion-exploracion` al repo privado Danikuso/TheMultipurposeCompanion (2026-10-18). Rechazo previo resuelto por esta autorización; push ejecutado correctamente hasta 3656ee3, los cierres siguientes se sincronizarán tras verificar.
- [x] **F1 · Canonn (capa 3 de exobiología):** el propietario **tiene el permiso** de Canonn (2026-10-19) → capa activada (`4f33956`, `7c35289`, `0ff96f1`): solo se envía el nombre del sistema (`getSystemPoi`) y la categoría (`codex/ref`), nunca datos del comandante; 1 petición/s con semáforo propio, timeout 15 s, caché persistente; respeta opt-out, Legacy y replay.
- [x] **F1 · Nombres de especies:** propietario aprueba aplazar tabla propia por idioma; F1 usa `_Localised` del diario y nombre latino de respaldo (2026-10-18). Spec §8 y plan actualizados.
- [x] **F1 — plan aprobado** (2026-10-17).
- [x] **F1 — dónde vive `eval`:** resuelto → `tmc-headless eval`.
- [x] **F1 — fuente de las reglas de predicción de exobiología:** el propietario eligió unificar reglas propias + historial (offline) con Canonn como refuerzo online (2026-10-17). Recogido en la spec §5.3, §6 y §11 (spike 3).
- [x] **F1 — spec aprobada** y **F0 fusionada en `main`** (2b953e6, merge --no-ff; re-verificado: fmt, clippy y 150 tests PASS). Sin push.
- [x] **Antes de Task 8 — dependencia arquitectónica aprobada:** spec §3.1 permite tmc-store→tmc-model, pero Task 8 requiere también tmc-journal para list_journals/LineTailer. Propuesta concreta en el plan: permitir tmc-store→tmc-journal (sin ciclos, sin cambiar interfaces). El propietario aprobó explícitamente añadir la dependencia; spec §3.1 sincronizada. Tasks 6–7 continúan.
- [x] **Discrepancia de seguimiento resuelta por autorización del propietario (2026-10-17):** al abrir la sesión, Task 4 figuraba pendiente y en Step 1, pero `git log` y `git show --stat` confirman `70e4999` (GameState, SourceEvent, HudTheme y reductores) y `177eac1` (correcciones de reductores). Además, la siguiente acción apuntaba a Task 2, ya marcada hecha, y F0 figura «sin empezar» pese a Tasks 0–3 hechas. La spec conserva «pendiente de revisión» mientras esta cabecera la declara aprobada. El propietario confirmó «Sí, hazlo»: aprobación de spec y plan válida; se autoriza revisar y verificar Task 4 y continuar con Task 5 si pasa.
- [ ] **(en aprobación 2026-10-19) CAPI de Frontier:** solicitud de cliente en user.frontierstore.net (scopes `auth` + `capi`). Pendiente confirmar que se envió y recibir el Client ID → guardarlo en `.env` como `TMC_CAPI_CLIENT_ID` (nunca en el repo). Necesario en F6.
- [ ] **(en aprobación 2026-10-19) Inara:** la clave personal se pegó en el chat el 2026-10-17 → regenerarla antes de usarla en TMC. Whitelisting del nombre `TheMultipurposeCompanion` cuando haya versión funcional (F6).
- [ ] **EDSM:** obtener API key y nombre exacto de comandante (F6).

## Decisiones tomadas

- 2026-10-20 00:03 · Corregidas las tablas históricas F0/F1/F2: una sustitución sin acotar a F3 había sobrescrito sus tareas 16 y 17. Restauradas sus secciones completas desde HEAD y verificadas por igualdad exacta normalizando saltos de línea. Los logs serán VERBOSE de toda la app, exclusivamente debug_assertions y en debug_logs junto al ejecutable, por instrucción del propietario.

- 2026-10-19 23:58 · Propietario prioriza logs exhaustivos de TODA la app para reportes de pruebas; exploración añade contexto específico, pero el alcance es general. F3 Task17 se pausa; se preserva TDD parcial. Se corrige tabla16 que seguía en curso pese a cierre real 3a33097 (spec/calidad y 924 Rust PASS ya verificados).

- 2026-10-18 · Propietario: spec de F2 aprobada; plan y ejecución con subagentes autorizados.

- 2026-10-18 · Propietario: F2 sin Rhino ni Law & Disorder, con AX; maquetas de combate y minería v2 aprobadas; calor como indicador de 3 estados y casco por escalones (el juego no expone calor % ni casco en vivo).

- 2026-10-18 · Task 32 (cierre de F1): se añaden `--disable-gpu --disable-gpu-compositing --num-raster-threads=1 --js-flags=--max-old-space-size=32`, `StorageServiceOutOfProcess` desactivado y `COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW` por defecto en `apps/desktop/src/window.rs` para mantener el consumo total con los 3 módulos de F1 en **108,01 MB** (< 150 MB) y **0,00 % CPU** en reposo (< 1 %). Por instrucción del propietario («Revisa el estado del proyecto y finaliza F1»), se cierra F1 dejando documentada la cobertura parcial del historial local de 7 diarios.
- 2026-10-18 · Propietario: aprueba corrección §5.1 de validación MaxJumpRange con masa en vacío y combustible máximo por salto; spec sincronizada.
- 2026-10-18 · Propietario: aprobados nombres de especies desde diario con respaldo latino y aplazamiento de tabla por idioma para después de F1; autorizado push de la rama F1 al repositorio privado.
- 2026-10-18 · Task 22: reconstrucción ordenada por timestamp UTC, diario y línea, manteniendo snapshot por ID; corrige la suposición de orden de inserción del plan. Checkpoints y reintento conservan equivalencia vivo/historial sin cambiar interfaces públicas.
- 2026-10-18 · Propietario: panel del planificador validado en demo en vivo.
- 2026-10-18 · Seguridad (Task 12): toda petición que cambia estado exige Origin coincidente (o ausencia de cabeceras de navegador) y `Content-Type: application/json` si lleva cuerpo; clientes externos (Stream Deck, VoiceAttack) deben enviar JSON.
- 2026-10-17 · Propietario: Canonn desactivado en F1; él solicitará permiso sin enviar datos de usuarios.
- 2026-10-17 · Propietario: plan de F1 aprobado; `eval` en `tmc-headless` (sin tocar arquitectura); ejecución con subagentes.
- 2026-10-17 · Propietario: spec de F1 aprobada; fusión de F0 en `main` y rama `fase/f1-navegacion-exploracion`.
- 2026-10-17 · Propietario: predicción de exobiología en F1 = reglas propias + historial del comandante, reforzadas con Canonn online (sin copiar datos al repo salvo licencia compatible).
- 2026-10-17 · Task 18: se pasan argumentos adicionales a WebView2 (`--in-process-gpu --enable-features=NetworkServiceInProcess2 --renderer-process-limit=1 --disable-gpu-shader-disk-cache --js-flags=--max-old-space-size=64`) en `WebviewWindowBuilder::additional_browser_args` para mantener los Private Bytes totales del árbol de procesos (`TMC.exe` + `msedgewebview2.exe`) en ~144,91 MB (< 150 MB) y 0,09 % de CPU en reposo (< 1 %).
- 2026-10-17 · Task 11: TTL de alertas evaluado al siguiente Journal/Status con tiempo >= since + 60 s; sin caducidad autónoma si no llega información. Desviación explícita del plan, sin cambiar spec ni API del módulo.
- 2026-10-17 · Task 8: flush_blocking devuelve Result<(), StoreError> para no confirmar persistencia fallida; desviación documentada en plan, sin cambio de spec. R1 conserva el diario como recuperación y offset exclusivo del importador.
- 2026-10-17 · Propietario: aprobada dependencia tmc-store → tmc-journal para la importación de Task 8; spec §3.1 actualizada sin cambiar interfaces.
- 2026-10-17 · Stack: Rust + Tauri 2 + Svelte 5 + SQLite + axum + Fluent (spec §2).
- 2026-10-17 · Licencia PolyForm Noncommercial 1.0.0 (público, sin uso comercial). Sin código/datos GPL.
- 2026-10-17 · Voz con KokoroTTS en proceso aparte; alemán sin voz.
- 2026-10-17 · Diseño de UI para segunda pantalla UW 2560×1080; colores del HUD del jugador.
- 2026-10-17 · Repositorio privado en GitHub; se hará público más adelante.
- 2026-10-17 · Rulings de ejecución R1–R5 (writer no toca offset; ruta de locales en UI; trait ConfigHost; Task 0 la ejecuta el controlador; job `ui` de CI en Task 15).

## Registro de sesiones

- 2026-10-19 23:58 (Europe/Madrid) · Codex · Propietario pide instrumentación diagnóstica de toda la app tras detectar cuerpos ausentes en exploración. Pausado implementadorTask17; código parcial preservado. Análisis con deep-plan/architect-opus dentro del logging aprobado F0§9, antes de implementar.

- 2026-10-19 23:52 (Europe/Madrid) · Codex · Task 16 cerrada con top5, snapshots locales, mezcla por frescura y origen; revisión detectó y corrigió resurrección de ofertas, ausencia de mercancías y límite antes de pad. Migración aditiva f3_market_full_snapshots y consultas bounded; spec/calidad aprobadas, 924 Rust PASS (2 ignorados), fmt/clippy OK.

- 2026-10-19 22:09 (Europe/Madrid) · Codex · Task 15 cerrada con 13 tests RED→GREEN: coste medio, inventario parcial, misión/robado, guardia SRV y reconstrucción por comandante/fecha. Revisiones aprobadas, fmt/clippy OK, 893 Rust PASS (2 ignorados); UI sin cambios.

- 2026-10-19 22:02 (Europe/Madrid) · Codex · Task 14 cerrada: comercio/misiones en 5 idiomas, cobertura de rangos y argumentos de formato; RED→GREEN, spec/calidad aprobadas, 880 Rust y 144 UI PASS, fmt/clippy/check/build OK.

- 2026-10-19 21:58 (Europe/Madrid) · Codex · Task 13: datos propios provisionales (1 rara/4 cabinas), carga embebida tolerante y tests RED→GREEN; spec/calidad aprobadas; fmt/clippy OK, 878 Rust PASS (2 ignorados); UI sin cambios.

- 2026-10-19 21:54 (Europe/Madrid) · Codex · Task 12 cerrada con TDD y dos revisiones: guardia replay explícita, siembra tras importación y coords sin dependencia del progreso. fmt/clippy OK, 874 Rust PASS (2 ignorados), 144 UI PASS, Svelte check/build OK; binario main de prueba intacto.

- 2026-10-19 21:47 (Europe/Madrid) · Codex · Propietario reanuda F3 mientras prueba main. Retomadas correcciones Task 12 con subagente; checks en debug y datos temporales, binario de prueba target/release preservado.

- 2026-10-19 21:46 (Europe/Madrid) · Codex · Propietario pide PAUSAR desarrollo y compilar main para juego real. Compilado checkout aislado `main@9154d41`: UI build OK; `cargo tauri build -- --offline` release e instalador NSIS OK (6,02 MiB). Copias estables en `target/prueba-main-9154d41/{TMC.exe,TMC_0.1.0_x64-setup.exe}`. F3 permanece sin commit con correcciones parciales de Task 12.

- 2026-10-19 21:43 (Europe/Madrid) · Codex · Retomada Task 12 desde c7840f5 y correcciones pendientes. RED de compilación E0425 por helper incompleto; revisión detecta guardia replay insuficiente. Correcciones con subagente y revisiones secuenciales en curso; sin push.

- 2026-10-19 19:45 (Europe/Madrid) · Claude Opus 5.5 · Spec de F3 diseñada con el propietario (propuesta, maquetas, turismo opción B), revisada por builder y aprobada; rama `fase/f3-comercio-misiones`.
- 2026-10-19 17:45 (Europe/Madrid) · Claude Opus 5.5 · Respuestas del propietario: rangos de combate en inglés (`4e75386`); permiso de Canonn obtenido → capa 3 activada y revisada (`4f33956`, `7c35289`, `0ff96f1`; 768 tests Rust PASS); cuentas CAPI/Inara/EDSM en aprobación.
- 2026-10-19 17:25 (Europe/Madrid) · Claude Opus 5.5 · Propietario autoriza la fusión: `fase/f2-combate-mineria` fusionada en `main` (`e069a14`, --no-ff). Sobre `main`: fmt/clippy limpios, 755 tests Rust PASS, svelte-check 0 errores, 144 vitest PASS, build OK. `main` subida a GitHub privado.
- 2026-10-19 14:35 (Europe/Madrid) · Claude Opus 5.5 · F2 ejecutada con subagentes (Tasks 19–28) y cerrada: 10/10 criterios PASS con evidencias en el plan; footprint con ventana 112,7 MB / 0 % CPU; revisión final de rama (Opus) con 1 crítico corregido (acciones reset de combate/minería no llegaban al núcleo, `c54a541`) y robustez (`cb484d8`); `core.clock` para la hora del diario en paneles. Rama `fase/f2-combate-mineria` subida; falta fusionar en `main` con autorización del propietario.
- 2026-10-18 20:56 (Europe/Madrid) · Antigravity · Solicitud del propietario completada: añadida visualización de todos los tipos de estrellas del sistema actual en la vista de exploración (commit `6e4276e`). Incluye `star_classes` ordenadas y desduplicadas en `SystemTopic`, insignias por tipo con marcado repostable en `SystemBodiesPanel`, reflejo en la tarjeta de `RouteStrip` y textos en los 5 idiomas. 527 tests Rust PASS, 91 tests Vitest PASS, svelte-check y build en verde.
- 2026-10-18 18:43 (Europe/Madrid) · Gemini 4 Argon (Medium) · Task 32 y cierre de F1 completados: `cargo tauri build` (`TMC_0.1.0_x64-setup.exe` 5,79 MiB), `measure-footprint.ps1 -ShowWindow -Seconds 120` (`max_mb=108.01` < 150 MB, `avg_cpu_idle=0.00%` < 1 % PASS), Criterio 7 verificado en vivo contra Spansh + portapapeles, 526 tests Rust PASS, 89 tests Vitest PASS, Playwright E2E PASS y 10/10 criterios documentados en el plan. Fase F1 cerrada.
- 2026-10-18 18:01 (Europe/Madrid) · Codex · Task31 terminada con TDD, revisión y 3 rondas de corrección de bordes: 515 Rust/81 UI, fmt/clippy/check/build y smoke real PASS (puerto47322, viewport2560/800). §5.1 aprobada por el propietario y sincronizada. Task32 inicia build/consumo; criterios3/4/6 sin datos suficientes siguen pendientes.

- 2026-10-18 17:44 (Europe/Madrid) · Codex · Reconciliado estado tras sesión posterior: Tasks 24–30 completas, HEAD66f10b8 y árbol limpio. Continúo Task31 con TDD y subagente. Nombres y push ya aprobados; §5.1 y datos suficientes para criterios3/4/6 siguen pendientes. No rehacer código de especies ni geodesia.

- 2026-10-18 13:55 (Europe/Madrid) · Claude Opus 5.5 · Retomado tras la sesión de Codex (Tasks 21–23 y geodesia de 24). Task 21 fix (3984224) revisado; Task 24 tabla de especies completada (c1857be). CI de GitHub deshabilitada por el propietario.

- 2026-10-18 04:11 (Europe/Madrid) · Codex · Geodesia de Task 24 revisada y verificada: 8/8 nuevos, 421 Rust/56 UI, fmt/clippy/check/build correctos. Task 24 incompleta: nombres/tabla esperan decisión; Task 25 no iniciada. Sin archivos de código pendientes tras commit parcial. Push retenido por rechazo de revisión automática hasta autorización explícita.

- 2026-10-18 04:08 (Europe/Madrid) · Codex · Task 23 cerrada b669a77, revisión y visual local aprobadas, 413 Rust/56 UI con fmt/clippy/check/build PASS. Task 24 avanza solo geodesia; la tabla/nombres espera aprobación de la desviación registrada.

- 2026-10-18 04:02 (Europe/Madrid) · Codex · Task 22 cerrada (ffa3433; status9bd4ba8), revisión aprobada y 413 Rust/46 UI con fmt/clippy/check/build PASS. Task 23 en curso: virtualización y panel de cuerpos, tests pendientes. Push y nombres de especies siguen pendientes de respuesta del propietario.

- 2026-10-18 03:48 (Europe/Madrid) · Codex · Task 21 revisada y cerrada (c25b238; status96dc926), con marca Terraforming y payload300cuerpos corregidos (máximo54.560bytes). Test de configuración aislado en4e01e51. 392 Rust/46 UI yfmt/clippy/check/build PASS. Task22 en curso; push retenido por revisión automática hasta autorización explícita.

- 2026-10-18 01:00 (Europe/Madrid) · Claude Opus 5.5 · F1 Bloques 0 y 1 completados (Tasks 0–12) con subagentes, revisión por tarea y rondas de corrección; workspace en verde. Decisión del propietario: Canonn desactivado hasta su permiso. Siguiente: Task 13. Ledger local (no versionado): `.superpowers/sdd/2026-10-17-tmc-f1-navegacion-exploracion/progress.md`.

- 2026-10-17 22:40 (Europe/Madrid) · Claude Opus 5.5 · Spec de F1 aprobada y commiteada (797f65e); F0 fusionada en `main` (2b953e6, sin push); rama `fase/f1-navegacion-exploracion`. Plan de F1 escrito (33 tareas) por subagente architect y revisado contra la spec; pendiente de aprobación.

- 2026-10-17 22:20 (Europe/Madrid) · Claude Opus 5.5 · F0 re-verificada (fmt/clippy/150 tests PASS, árbol limpio). Diseño de F1 con el propietario mediante maquetas (tira de ruta, cuerpos del sistema, exobiología; aprobadas). Borrador de spec de F1 escrito y revisado por builder. Archivos sin commit: spec de F1, `status.md`, `.gitignore` (`.superpowers/`).

- 2026-10-17 20:59 (Europe/Madrid) · Gemini 4 Argon (Medium) · Tasks 10–18 completadas y verificadas (`f56862d`, `0ee2585`, `d1e12be`, `31b439f`, `885b0a2`, `bd78913`, `8d26c0b`, `db4507d` + cierre de Task 18): 150 tests Rust PASS, 10 tests Vitest UI PASS, smoke Playwright PASS, instalador NSIS `TMC_0.1.0_x64-setup.exe` (3,83 MiB) verificado y presupuesto de consumo (`max_mb=144.91` < 150 MB, `avg_cpu_idle=0.09%` < 1 %) cumplido. Fase F0 cerrada.
- 2026-10-17 18:44 (Europe/Madrid) · Codex · Continuación sin interrupciones: Tasks 6–9 cerradas (7634de8, 2cc55b7, 9fb90b7, 5565fbb) con TDD y revisión; 107 tests, fmt/clippy PASS. Task 10 en curso: RED 4 FAIL, cinco FTL con 67 claves listos. Pendiente validar bundles, revisión y cierre; archivos actuales sin commit: tmc-i18n, locales, Cargo.lock, avisos y seguimiento.
- 2026-10-17 18:12 (Europe/Madrid) · Codex · Task 4 cerrada en 2ec0abe tras revisión y correcciones TDD (31 PASS/4 FAIL → 35 PASS); fmt y clippy del workspace PASS. Task 5 completada con TDD y revisión independiente: 9 tests nuevos, workspace 44 PASS, fmt/clippy PASS. Pendiente ejecución nativa Linux; UI y medición de consumo corresponden a Tasks 15/18. Commits: 2ec0abe (Task 4), e8e6cc8 (Task 5); seguimiento final en commit documental inmediato. Sin código sin commit. Cargo disponible añadiendo C:\Users\danik\.cargo\bin al PATH del proceso. UI aún no creada (Task 15).
- 2026-10-17 17:59 (Europe/Madrid) · Codex · Discrepancia de Task 4 documentada; ejecución detenida según AGENTS.md §1, pendiente autorización del propietario (resuelta en la sesión siguiente).
- 2026-10-17 · Claude Opus 5.5 · Inicio de ejecución de F0 por subagentes: Tasks 0–1 hechas.
- 2026-10-17 · Claude Opus 5.5 · Investigación (`docs/research/`), spec de arquitectura y F0 aprobada, plan de F0 escrito, `AGENTS.md` y `status.md` creados.
