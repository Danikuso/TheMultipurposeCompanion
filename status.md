# Estado del desarrollo de TMC

> Lee primero [`AGENTS.md`](AGENTS.md). Este archivo indica el punto **exacto** del desarrollo. Actualízalo según las reglas de `AGENTS.md` §4.

- **Última actualización:** 2026-10-17 22:20 (Europe/Madrid) · Claude Opus 5.5
- **Fase actual:** F1 — Navegación, exploración y exobiología (spec aprobada; plan en redacción). F0 ✅ fusionada en `main` (2b953e6).
- **Spec activa:** [`docs/specs/2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md`](docs/specs/2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md) — ✅ aprobada (2026-10-17); base: [`2026-10-17-tmc-arquitectura-y-f0-design.md`](docs/specs/2026-10-17-tmc-arquitectura-y-f0-design.md)
- **Plan activo:** [`docs/plans/2026-10-17-tmc-f1-navegacion-exploracion.md`](docs/plans/2026-10-17-tmc-f1-navegacion-exploracion.md) — ✅ aprobado (2026-10-17), 33 tareas, bloques 0–5. Ejecución con subagentes. Anterior: [`2026-10-17-tmc-f0-cimientos.md`](docs/plans/2026-10-17-tmc-f0-cimientos.md) ✅ 19/19
- **Rama de trabajo:** `fase/f1-navegacion-exploracion` (desde `main` tras fusionar F0)

## Puntero actual

- **Tarea:** F1 · Bloque 0 (spikes) completado; Task 7 (`tmc-core` — peticiones de red, tareas de SQLite y migraciones) 🔄 en curso.
- **Siguiente acción:** completar la Task 7 del plan de F1 (ejecución con subagentes; ledger en `.superpowers/sdd/2026-10-17-tmc-f1-navegacion-exploracion/progress.md`, no versionado).

## Fases

| Fase | Contenido | Estado |
|---|---|---|
| F0 | Cimientos (lectura, estado, SQLite, servidor, UI base, panel general, instalador) | ✅ completada y fusionada (2b953e6) |
| F1 | Navegación, exploración, exobiología | 🔄 en ejecución |
| F2 | Cazarrecompensas/combate, minería | ⬜ |
| F3 | Comercio, misiones y pasajeros, recepción EDDN | ⬜ |
| F4 | Ingenieros y materiales | ⬜ |
| F5 | Fleet carrier, colonización, Powerplay 2.0, BGS, a pie | ⬜ |
| F6 | Cuentas: envío EDDN, EDSM, Inara, CAPI | ⬜ |
| F7 | Voz (KokoroTTS; alemán sin voz) | ⬜ |
| F8 | Overlay ligero e integrado | ⬜ |
| F9 | Crimen y piratería (Law & Disorder, dic. 2026) | ⬜ |
| F10 | Atajos, Stream Deck, VoiceAttack, build Linux | ⬜ |

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
| 7–32 | Ver plan | ⬜ | — | |

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

- [ ] **F1 · Canonn (capa 3 de exobiología)** — decidido 2026-10-17: queda **desactivada**; el propietario solicitará permiso a Canonn **sin enviar datos de usuarios**. Pendiente: respuesta de Canonn. Contexto: su README exige una **solicitud formal** para usar la API en software propio y pide **capturar/enviar datos de los usuarios**. Decisión del propietario: (a) solicitar permiso sin enviar datos de usuario, (b) aceptar enviar datos (requiere cambio de spec y opt-in), o (c) renunciar a Canonn. Mientras tanto, la capa 3 se implementa desactivada y F1 avanza con reglas propias + historial.
- [ ] **F1 · Nombres de especies:** aprobar aplazar la tabla propia por idioma (se usa `_Localised` del diario y nombre latino de respaldo).

- [x] **F1 — plan aprobado** (2026-10-17).
- [x] **F1 — dónde vive `eval`:** resuelto → `tmc-headless eval`. Antes: (spec §10.1 dice `tmc-replay eval`): requiere que `tmc-replay` dependa de `tmc-journal` y `tmc-modules` (cambio de la tabla §3.1 de la spec de arquitectura) o, alternativa sin tocar la arquitectura, `tmc-headless eval`. Task 3 del plan.

- [x] **F1 — fuente de las reglas de predicción de exobiología:** el propietario eligió unificar reglas propias + historial (offline) con Canonn como refuerzo online (2026-10-17). Recogido en la spec §5.3, §6 y §11 (spike 3).
- [x] **F1 — spec aprobada** y **F0 fusionada en `main`** (2b953e6, merge --no-ff; re-verificado: fmt, clippy y 150 tests PASS). Sin push.

- [x] **Antes de Task 8 — dependencia arquitectónica aprobada:** spec §3.1 permite tmc-store→tmc-model, pero Task 8 requiere también tmc-journal para list_journals/LineTailer. Propuesta concreta en el plan: permitir tmc-store→tmc-journal (sin ciclos, sin cambiar interfaces). El propietario aprobó explícitamente añadir la dependencia; spec §3.1 sincronizada. Tasks 6–7 continúan.

- [x] **Discrepancia de seguimiento resuelta por autorización del propietario (2026-10-17):** al abrir la sesión, Task 4 figuraba pendiente y en Step 1, pero `git log` y `git show --stat` confirman `70e4999` (GameState, SourceEvent, HudTheme y reductores) y `177eac1` (correcciones de reductores). Además, la siguiente acción apuntaba a Task 2, ya marcada hecha, y F0 figura «sin empezar» pese a Tasks 0–3 hechas. La spec conserva «pendiente de revisión» mientras esta cabecera la declara aprobada. El propietario confirmó «Sí, hazlo»: aprobación de spec y plan válida; se autoriza revisar y verificar Task 4 y continuar con Task 5 si pasa.
- [ ] **CAPI de Frontier:** solicitud de cliente en user.frontierstore.net (scopes `auth` + `capi`). Pendiente confirmar que se envió y recibir el Client ID → guardarlo en `.env` como `TMC_CAPI_CLIENT_ID` (nunca en el repo). Necesario en F6.
- [ ] **Inara:** la clave personal se pegó en el chat el 2026-10-17 → regenerarla antes de usarla en TMC. Whitelisting del nombre `TheMultipurposeCompanion` cuando haya versión funcional (F6).
- [ ] **EDSM:** obtener API key y nombre exacto de comandante (F6).

## Decisiones tomadas

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

- 2026-10-17 22:40 (Europe/Madrid) · Claude Opus 5.5 · Spec de F1 aprobada y commiteada (797f65e); F0 fusionada en `main` (2b953e6, sin push); rama `fase/f1-navegacion-exploracion`. Plan de F1 escrito (33 tareas) por subagente architect y revisado contra la spec; pendiente de aprobación.

- 2026-10-17 22:20 (Europe/Madrid) · Claude Opus 5.5 · F0 re-verificada (fmt/clippy/150 tests PASS, árbol limpio). Diseño de F1 con el propietario mediante maquetas (tira de ruta, cuerpos del sistema, exobiología; aprobadas). Borrador de spec de F1 escrito y revisado por builder. Archivos sin commit: spec de F1, `status.md`, `.gitignore` (`.superpowers/`).

- 2026-10-17 20:59 (Europe/Madrid) · Gemini 4 Argon (Medium) · Tasks 10–18 completadas y verificadas (`f56862d`, `0ee2585`, `d1e12be`, `31b439f`, `885b0a2`, `bd78913`, `8d26c0b`, `db4507d` + cierre de Task 18): 150 tests Rust PASS, 10 tests Vitest UI PASS, smoke Playwright PASS, instalador NSIS `TMC_0.1.0_x64-setup.exe` (3,83 MiB) verificado y presupuesto de consumo (`max_mb=144.91` < 150 MB, `avg_cpu_idle=0.09%` < 1 %) cumplido. Fase F0 cerrada.
- 2026-10-17 18:44 (Europe/Madrid) · Codex · Continuación sin interrupciones: Tasks 6–9 cerradas (7634de8, 2cc55b7, 9fb90b7, 5565fbb) con TDD y revisión; 107 tests, fmt/clippy PASS. Task 10 en curso: RED 4 FAIL, cinco FTL con 67 claves listos. Pendiente validar bundles, revisión y cierre; archivos actuales sin commit: tmc-i18n, locales, Cargo.lock, avisos y seguimiento.
- 2026-10-17 18:12 (Europe/Madrid) · Codex · Task 4 cerrada en 2ec0abe tras revisión y correcciones TDD (31 PASS/4 FAIL → 35 PASS); fmt y clippy del workspace PASS. Task 5 completada con TDD y revisión independiente: 9 tests nuevos, workspace 44 PASS, fmt/clippy PASS. Pendiente ejecución nativa Linux; UI y medición de consumo corresponden a Tasks 15/18. Commits: 2ec0abe (Task 4), e8e6cc8 (Task 5); seguimiento final en commit documental inmediato. Sin código sin commit. Cargo disponible añadiendo C:\Users\danik\.cargo\bin al PATH del proceso. UI aún no creada (Task 15).
- 2026-10-17 17:59 (Europe/Madrid) · Codex · Discrepancia de Task 4 documentada; ejecución detenida según AGENTS.md §1, pendiente autorización del propietario (resuelta en la sesión siguiente).
- 2026-10-17 · Claude Opus 5.5 · Inicio de ejecución de F0 por subagentes: Tasks 0–1 hechas.
- 2026-10-17 · Claude Opus 5.5 · Investigación (`docs/research/`), spec de arquitectura y F0 aprobada, plan de F0 escrito, `AGENTS.md` y `status.md` creados.
