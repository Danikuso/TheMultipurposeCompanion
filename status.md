# Estado del desarrollo de TMC

> Lee primero [`AGENTS.md`](AGENTS.md). Este archivo indica el punto **exacto** del desarrollo. Actualízalo según las reglas de `AGENTS.md` §4.

- **Última actualización:** 2026-10-18 19:20 (Europe/Madrid) · Claude Opus 5.5
- **Fase actual:** F1 — Navegación, exploración y exobiología (✅ completada y fusionada en `main` en `b96fecf`). F0 ✅ fusionada en `main` (2b953e6).
- **Spec activa:** [`docs/specs/2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md`](docs/specs/2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md) — ✅ aprobada (2026-10-17); base: [`2026-10-17-tmc-arquitectura-y-f0-design.md`](docs/specs/2026-10-17-tmc-arquitectura-y-f0-design.md)
- **Plan activo:** [`docs/plans/2026-10-17-tmc-f1-navegacion-exploracion.md`](docs/plans/2026-10-17-tmc-f1-navegacion-exploracion.md) — ✅ completado (33/33 tareas, bloques 0–5). Anterior: [`2026-10-17-tmc-f0-cimientos.md`](docs/plans/2026-10-17-tmc-f0-cimientos.md) ✅ 19/19
- **Rama de trabajo:** `main`

## Puntero actual

- **Tarea:** F2 — spec ✅ aprobada (2026-10-18): [`docs/specs/2026-10-18-tmc-f2-combate-mineria-design.md`](docs/specs/2026-10-18-tmc-f2-combate-mineria-design.md) (diseñada con el propietario con maquetas; revisada por builder). Rama `fase/f2-combate-mineria`.
- **Siguiente acción:** ejecutar el plan de F2 [`docs/plans/2026-10-18-tmc-f2-combate-mineria.md`](docs/plans/2026-10-18-tmc-f2-combate-mineria.md) (29 tareas, aprobado al ordenar «escribe el plan y ejecútalo») (en curso: Task 4), con subagentes; ledger en `.superpowers/sdd/2026-10-18-tmc-f2-combate-mineria/progress.md`.

## Fases

| Fase | Contenido | Estado |
|---|---|---|
| F0 | Cimientos (lectura, estado, SQLite, servidor, UI base, panel general, instalador) | ✅ completada y fusionada (2b953e6) |
| F1 | Navegación, exploración, exobiología | ✅ completada y fusionada (b96fecf) |
| F2 | Cazarrecompensas/combate (incl. AX), minería | 🔄 en ejecución |
| F3 | Comercio, misiones y pasajeros, recepción EDDN | ⬜ |
| F4 | Ingenieros y materiales | ⬜ |
| F5 | Fleet carrier, colonización, Powerplay 2.0, BGS, a pie | ⬜ |
| F6 | Cuentas: envío EDDN, EDSM, Inara, CAPI | ⬜ |
| F7 | Voz (KokoroTTS; alemán sin voz) | ⬜ |
| F8 | Overlay ligero e integrado | ⬜ |
| F9 | Crimen y piratería (Law & Disorder, dic. 2026) | ⬜ |
| F10 | Atajos, Stream Deck, VoiceAttack, build Linux | ⬜ |

## Tareas de F2

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 0 | Spike AX | ✅ hecha | a2276dd | Sin datos AX en los 7 diarios; AX desde fuentes secundarias (sin verificar); BrokerPercentage: Amount tal cual; sin asistencias. Revisión aprobada. |
| 1 | Spike Ardent | ✅ hecha | e9937cd | Rutas por address OK; filtro de plataforma local; nombres = Type del diario; fixtures sintéticos. |
| 2 | Aprobaciones del propietario tras los spikes | ✅ hecha | (este commit) | Registradas en Bloqueos; no bloquean. |
| 3 | `tmc-model` — eventos de combate | ✅ hecha | ecce185 | ShipTargeted (alias ShipTargetted), Bounty, bonos, RedeemVoucher, calor, escudos… Revisión aprobada. |
| 4 | `tmc-model` — eventos de minería y mercado, `Cargo` con inventario, reparaciones | 🔄 en curso | — | |
| 5 | `tmc-model` — bloque `ThargoidWar` y facciones en `LocationInfo` | ⬜ | — | |
| 6 | `tmc-model` — consultas de Ardent, `PadSize` y ajustes de minería | ⬜ | — | |
| 7 | `tmc-net` — adaptador Ardent | ⬜ | — | |
| 8 | configuración nueva en `config.toml` y `PUT /api/config` | ⬜ | — | |
| 9 | datos estáticos `ship_pads.json` y `mining_yield.json` | ⬜ | — | |
| 10 | i18n — `combat.ftl` y `mining.ftl` | ⬜ | — | |
| 11 | `combat` — objetivo actual | ⬜ | — | |
| 12 | `combat` — mi nave | ⬜ | — | |
| 13 | `combat` — libro de vales | ⬜ | — | |
| 14 | `combat` — sesión de caza y reconstrucción | ⬜ | — | |
| 15 | `combat` — zona de conflicto y AX | ⬜ | — | |
| 16 | `combat` — módulo, topics, relevancia, red y registro | ⬜ | — | |
| 17 | UI — paneles «Objetivo» y «Mi nave» | ⬜ | — | |
| 18 | UI — «Sesión de caza», «Zona de conflicto» y «AX» | ⬜ | — | |
| 19 | `mining` — prospector | ⬜ | — | |
| 20 | `mining` — sesión de minería | ⬜ | — | |
| 21 | `mining` — `PriceBook` y «Dónde vender» | ⬜ | — | |
| 22 | `mining` — módulo, topics, relevancia, red y registro | ⬜ | — | |
| 23 | UI — panel «Prospector» | ⬜ | — | |
| 24 | UI — «Sesión de minería» y «Dónde vender» | ⬜ | — | |
| 25 | UI — disposiciones «Combate» y «Minería», relevancia y ajustes | ⬜ | — | |
| 26 | fixtures sintéticos y pruebas de extremo a extremo | ⬜ | — | |
| 27 | `tmc-headless eval` — vales y rendimiento de asteroides | ⬜ | — | |
| 28 | avisos, consumo, verificación de los 10 criterios y cierre | ⬜ | — | |

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

- [ ] **F2 · Resultados de los spikes (para tu visto bueno; F2 sigue avanzando):**
  - **AX:** tus 7 diarios no tienen ningún dato AX. Los paneles AX se construyen con nombres de eventos y naves de fuentes secundarias, marcados como no verificados, y se prueban con datos sintéticos; se ajustarán cuando juegues una sesión AX.
  - **Asistencias:** no aparecen en tus diarios (`SharedWithOthers`), así que cada recompensa cuenta como baja completa.
  - **Vales cobrados en Interstellar Factors:** se usa el importe del diario tal cual; el % del broker se muestra como información.
  - **Ardent:** valores por defecto de «Dónde vender»: radio 100 al, volumen mínimo 100 t, datos de hasta 3 días, sin carriers; el filtro de plataforma se hace en TMC (Ardent no lo aplica en esa consulta). Se excluyen asentamientos a pie.

- [x] **F1 · Corrección de spec §5.1:** propietario aprueba explícitamente el 2026-10-18 validar MaxJumpRange con masa en vacío más combustible máximo por salto (no depósito lleno). Spec actualizada; evidencia de 15 loadouts en Task 29. Código ya alineado.
- [x] **F1 · Cierre con el historial local actual (7 diarios):** propietario ordena finalizar F1 (2026-10-18). Los criterios 3, 4 y 6 pasan en fixtures y los 15/15 loadouts locales coinciden (±0,0023 al); `tmc-headless eval` queda disponible para validar futuras sesiones con FSD de ingeniería/Guardian, ventas de exploración cubiertas y exobiología.
- [x] **Sincronización de F1:** propietario autoriza explícitamente push de `fase/f1-navegacion-exploracion` al repo privado Danikuso/TheMultipurposeCompanion (2026-10-18). Rechazo previo resuelto por esta autorización; push ejecutado correctamente hasta 3656ee3, los cierres siguientes se sincronizarán tras verificar.
- [ ] **F1 · Canonn (capa 3 de exobiología)** — decidido 2026-10-17: queda **desactivada**; el propietario solicitará permiso a Canonn **sin enviar datos de usuarios**. Pendiente: respuesta de Canonn. Mientras tanto, F1 funciona con reglas propias + historial.
- [x] **F1 · Nombres de especies:** propietario aprueba aplazar tabla propia por idioma; F1 usa `_Localised` del diario y nombre latino de respaldo (2026-10-18). Spec §8 y plan actualizados.
- [x] **F1 — plan aprobado** (2026-10-17).
- [x] **F1 — dónde vive `eval`:** resuelto → `tmc-headless eval`.
- [x] **F1 — fuente de las reglas de predicción de exobiología:** el propietario eligió unificar reglas propias + historial (offline) con Canonn como refuerzo online (2026-10-17). Recogido en la spec §5.3, §6 y §11 (spike 3).
- [x] **F1 — spec aprobada** y **F0 fusionada en `main`** (2b953e6, merge --no-ff; re-verificado: fmt, clippy y 150 tests PASS). Sin push.
- [x] **Antes de Task 8 — dependencia arquitectónica aprobada:** spec §3.1 permite tmc-store→tmc-model, pero Task 8 requiere también tmc-journal para list_journals/LineTailer. Propuesta concreta en el plan: permitir tmc-store→tmc-journal (sin ciclos, sin cambiar interfaces). El propietario aprobó explícitamente añadir la dependencia; spec §3.1 sincronizada. Tasks 6–7 continúan.
- [x] **Discrepancia de seguimiento resuelta por autorización del propietario (2026-10-17):** al abrir la sesión, Task 4 figuraba pendiente y en Step 1, pero `git log` y `git show --stat` confirman `70e4999` (GameState, SourceEvent, HudTheme y reductores) y `177eac1` (correcciones de reductores). Además, la siguiente acción apuntaba a Task 2, ya marcada hecha, y F0 figura «sin empezar» pese a Tasks 0–3 hechas. La spec conserva «pendiente de revisión» mientras esta cabecera la declara aprobada. El propietario confirmó «Sí, hazlo»: aprobación de spec y plan válida; se autoriza revisar y verificar Task 4 y continuar con Task 5 si pasa.
- [ ] **CAPI de Frontier:** solicitud de cliente en user.frontierstore.net (scopes `auth` + `capi`). Pendiente confirmar que se envió y recibir el Client ID → guardarlo en `.env` como `TMC_CAPI_CLIENT_ID` (nunca en el repo). Necesario en F6.
- [ ] **Inara:** la clave personal se pegó en el chat el 2026-10-17 → regenerarla antes de usarla en TMC. Whitelisting del nombre `TheMultipurposeCompanion` cuando haya versión funcional (F6).
- [ ] **EDSM:** obtener API key y nombre exacto de comandante (F6).

## Decisiones tomadas

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
