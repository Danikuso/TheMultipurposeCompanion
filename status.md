# Estado del desarrollo de TMC

> Lee primero [`AGENTS.md`](AGENTS.md). Este archivo indica el punto **exacto** del desarrollo. Actualízalo según las reglas de `AGENTS.md` §4.

- **Última actualización:** 2026-10-17 · Claude Opus 5.5 (controlador SDD)
- **Fase actual:** F0 — Cimientos
- **Spec activa:** [`docs/specs/2026-10-17-tmc-arquitectura-y-f0-design.md`](docs/specs/2026-10-17-tmc-arquitectura-y-f0-design.md) — ✅ aprobada (2026-10-17)
- **Plan activo:** [`docs/plans/2026-10-17-tmc-f0-cimientos.md`](docs/plans/2026-10-17-tmc-f0-cimientos.md) — ✅ aprobado; ejecución por subagentes
- **Rama de trabajo:** `fase/f0-cimientos`

## Puntero actual

- **Tarea:** Task 2 (`tmc-model` — Status.json), Step 1.
- **Siguiente acción:** implementar Task 2 según el plan. Ejecución por subagentes (implementador + revisor por tarea); registro detallado en `.superpowers/sdd/2026-10-17-tmc-f0-cimientos/progress.md` (local, no versionado).

## Fases

| Fase | Contenido | Estado |
|---|---|---|
| F0 | Cimientos (lectura, estado, SQLite, servidor, UI base, panel general, instalador) | 🔄 plan escrito, sin empezar |
| F1 | Navegación, exploración, exobiología | ⬜ |
| F2 | Cazarrecompensas/combate, minería | ⬜ |
| F3 | Comercio, misiones y pasajeros, recepción EDDN | ⬜ |
| F4 | Ingenieros y materiales | ⬜ |
| F5 | Fleet carrier, colonización, Powerplay 2.0, BGS, a pie | ⬜ |
| F6 | Cuentas: envío EDDN, EDSM, Inara, CAPI | ⬜ |
| F7 | Voz (KokoroTTS; alemán sin voz) | ⬜ |
| F8 | Overlay ligero e integrado | ⬜ |
| F9 | Crimen y piratería (Law & Disorder, dic. 2026) | ⬜ |
| F10 | Atajos, Stream Deck, VoiceAttack, build Linux | ⬜ |

## Tareas de F0

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 0 | Entorno de desarrollo (Build Tools, rustup, Tauri CLI) | ✅ | — | VS BuildTools 17.14, rustc 1.99.1 MSVC, tauri-cli 2.12.3 |
| 1 | Workspace, licencia y CI | ✅ | 69b4be8, 83201c9 | job `ui` de CI se añadirá en Task 15 |
| 2 | `tmc-model` — Status.json | ⬜ | — | |
| 3 | `tmc-model` — eventos del diario y archivos compañeros | ⬜ | — | |
| 4 | `tmc-model` — GameState y SourceEvent | ⬜ | — | |
| 5 | `tmc-journal` — localizar carpeta y listar diarios | ⬜ | — | |
| 6 | `tmc-journal` — lector incremental de líneas | ⬜ | — | |
| 7 | `tmc-journal` — Status, compañeros, proceso, HUD y watcher | ⬜ | — | |
| 8 | `tmc-store` — SQLite, escritor e importación | ⬜ | — | |
| 9 | `tmc-core` — actor, módulos, topics y alertas | ⬜ | — | |
| 10 | `tmc-i18n` y traducciones | ⬜ | — | |
| 11 | `tmc-modules` — registro y `dashboard` | ⬜ | — | |
| 12 | `tmc-server` — HTTP, WS, seguridad, emparejamiento | ⬜ | — | |
| 13 | `tmc-app` + `apps/headless` | ⬜ | — | |
| 14 | `tmc-replay` — reproducción y anonimización | ⬜ | — | |
| 15 | UI — base (WS, i18n, tema, shell) | ⬜ | — | |
| 16 | UI — panel general y ajustes | ⬜ | — | |
| 17 | `apps/desktop` — Tauri, bandeja, instalador | ⬜ | — | |
| 18 | Presupuesto de consumo y cierre de F0 | ⬜ | — | |

## Bloqueos y pendientes del propietario

- [ ] **CAPI de Frontier:** solicitud de cliente en user.frontierstore.net (scopes `auth` + `capi`). Pendiente confirmar que se envió y recibir el Client ID → guardarlo en `.env` como `TMC_CAPI_CLIENT_ID` (nunca en el repo). Necesario en F6.
- [ ] **Inara:** la clave personal se pegó en el chat el 2026-10-17 → regenerarla antes de usarla en TMC. Whitelisting del nombre `TheMultipurposeCompanion` cuando haya versión funcional (F6).
- [ ] **EDSM:** obtener API key y nombre exacto de comandante (F6).

## Decisiones tomadas

- 2026-10-17 · Stack: Rust + Tauri 2 + Svelte 5 + SQLite + axum + Fluent (spec §2).
- 2026-10-17 · Licencia PolyForm Noncommercial 1.0.0 (público, sin uso comercial). Sin código/datos GPL.
- 2026-10-17 · Voz con KokoroTTS en proceso aparte; alemán sin voz.
- 2026-10-17 · Diseño de UI para segunda pantalla UW 2560×1080; colores del HUD del jugador.
- 2026-10-17 · Repositorio privado en GitHub; se hará público más adelante.
- 2026-10-17 · Rulings de ejecución R1–R5 (writer no toca offset; ruta de locales en UI; trait ConfigHost; Task 0 la ejecuta el controlador; job `ui` de CI en Task 15).

## Registro de sesiones

- 2026-10-17 · Claude Opus 5.5 · Inicio de ejecución de F0 por subagentes: Tasks 0–1 hechas.
- 2026-10-17 · Claude Opus 5.5 · Investigación (`docs/research/`), spec de arquitectura y F0 aprobada, plan de F0 escrito, `AGENTS.md` y `status.md` creados.
