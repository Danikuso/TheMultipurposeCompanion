# Estado del desarrollo de TMC

> Lee primero [`AGENTS.md`](AGENTS.md). Este archivo indica el punto **exacto** del desarrollo. Actualízalo según las reglas de `AGENTS.md` §4.

- **Última actualización:** 2026-10-24 18:59 (Europe/Madrid) · Antigravity
- **Fase actual:** Fase 10 — Asistencia e Integración de Voz (Zero-Friction Voice Assistant)
- **Spec activa:** [`docs/specs/2026-10-24-tmc-f10-asistencia-voz-integracion-design.md`](docs/specs/2026-10-24-tmc-f10-asistencia-voz-integracion-design.md) — Asistencia e Integración de Voz.
- **Plan activo:** [`docs/plans/2026-10-24-tmc-f10-asistencia-voz-integracion.md`](docs/plans/2026-10-24-tmc-f10-asistencia-voz-integracion.md) — 7 tareas en 6 bloques.
- **Rama de trabajo:** `fase/f10-asistencia-voz-integracion`

## Puntero actual

- **Tarea:** Tarea 5: Asistente de voz en TopBar, escucha continua y controles en ajustes (`ui/`).
- **Último paso completado:**
  1. Tarea 1 completada (`251abe3`): Creado `crates/tmc-model/src/voice_intent.rs` con tipos `VoiceIntent`, `VoiceCommandRequest`, `VoiceCommandResponse` y `VoiceAttackStatus`, exportados en `tmc-model` y registrados en el catálogo de diagnóstico de Rust. 179 tests PASS y Clippy limpio.
  2. Tarea 2 completada (`6e540a0`): Implementado `VoiceIntentResolver` en `crates/tmc-core/src/voice_intent_resolver.rs` con normalización léxica tolerante, comprensión semántica en 5 idiomas (es, en, fr, de, pt) y resolución contextualizada contra el `GameState` (estado de nave, combustible, siguiente salto, copia al portapapeles, factores interestelares, mercados negros, multas y bio-señales). 48 tests de `tmc-core` PASS y Clippy limpio.
  3. Tarea 3 completada (`b00606c`): Creado detector y generador de perfiles VoiceAttack en `crates/tmc-app/src/voiceattack.rs`, integradas acciones en `AppActions` (`voice.status`, `voice.profile`, `clipboard.copy`, `voice.speak`), montados los endpoints REST `POST /api/voice/intent`, `GET /api/voice/voiceattack` y `GET /api/voice/voiceattack/profile` en `tmc-server`. 37 tests de `tmc-server`, 120 tests de `tmc-app` y 49 tests de `tmc-core` PASS y Clippy limpio.
  4. Tarea 4 completada (`78fc37a`): Creado catálogo `voice_assistant.ftl` en `locales/{es,en,fr,de,pt}/` con todas las cadenas de interfaz de voz, controles HUD de micrófono, monitor de integración con VoiceAttack y consultas de cabina. 18 tests de `tmc-i18n` PASS.
- **Siguiente acción:** Desarrollar `ui/src/lib/voiceAssistant.svelte.ts`, agregar el indicador de micrófono y feedback en `ui/src/shell/TopBar.svelte`, y la sección de asistencia de voz y estado de VoiceAttack en `ui/src/settings/Settings.svelte`.

## Tareas de F10 (Asistencia e Integración de Voz)

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 1 | Modelos de intenciones de voz, petición/respuesta y estado VoiceAttack (`tmc-model`) | ✅ hecha | `251abe3` | `VoiceIntent`, `VoiceCommandRequest`, `VoiceCommandResponse`, `VoiceAttackStatus`, tests PASS |
| 2 | `VoiceIntentResolver` semántico contra el estado de juego en Rust (`tmc-core`) | ✅ hecha | `6e540a0` | Comprensión léxica multilingüe y resolución de consultas de cabina |
| 3 | Endpoints REST de voz y generador de perfil VoiceAttack (`tmc-server`, `tmc-app`) | ✅ hecha | `b00606c` | `POST /api/voice/intent`, `GET /api/voice/voiceattack`, perfil `.vap` |
| 4 | Catálogo i18n de respuestas vocales y textos de cabina en 5 idiomas (`locales/`) | ✅ hecha | `78fc37a` | `voice_assistant.ftl` en es/en/fr/de/pt, verificado por `tmc-i18n` |
| 5 | Asistente de voz en TopBar, escucha continua y controles en ajustes (`ui/`) | 🔄 en curso | — | Indicador de micro HUD, gestión de escucha y toggle en ajustes |
| 6 | Tests unitarios en Vitest para la asistencia de voz (`ui/`) | ⬜ pendiente | — | Reactividad de escucha, conexión al endpoint de voz y preferencias |
| 7 | Verificación integral, compilación Release/Debug y cierre | ⬜ pendiente | — | Workspace Rust PASS, Vitest PASS, clippy limpio, build Release/Debug |

## Tareas de F9 (Crimen, Notoriedad y Piratería) — FUSIONADA EN MAIN

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 1 | Modelos de crimen, eventos de diario y topic `CrimeStatusTopic` (`tmc-model`) | ✅ hecha | `0e9dd87` | `CrimeType`, `CommitCrime`, `PayFines`, `PayBounties`, `CrimeStatusTopic`, tests PASS |
| 2 | Módulo de lógica criminal en Rust (`tmc-modules`) | ✅ hecha | `5714942` | Decaimiento de notoriedad en vuelo, multas, Interstellar Factor y Black Market |
| 3 | Detección contextual y avisos de seguridad en `VoiceAdvisor` (`tmc-core`) | ✅ hecha | `4bc8d63` | Notoriedad incrementada/limpia, contrabando, interdicción policial |
| 4 | Catálogo de textos y locuciones en los 5 idiomas (`locales/`) | ✅ hecha | `e973593` | `crime.ftl` en es/en/fr/de/pt, verificado por `tmc-i18n` |
| 5 | Panel `CrimePanel.svelte` e integración en el Shell («TMC Glass HUD») (`ui/`) | ✅ hecha | `96c3f2c` | Medidor notoriedad, Guía Limpieza, Contrabando y Mercados Negros |
| 6 | Tests unitarios en Vitest para panel de crimen y contrabando (`ui/`) | ✅ hecha | `ca57ada` | Reactividad de estado legal, temporizador y botones de copia (408/408 tests PASS) |
| 7 | Verificación integral, huella y compilación Release/Debug | ✅ hecha | `6de3226` | Workspace Rust PASS, Vitest 408/408 PASS, clippy limpio, binarios Release (32.7MB) + Debug (53.1MB) |
| 6 | Tests unitarios en Vitest para panel de crimen y contrabando (`ui/`) | ⬜ pendiente | — | Reactividad de estado legal, temporizador y botones de copia |
| 7 | Verificación integral, huella y compilación Release/Debug | ⬜ pendiente | — | Workspace Rust PASS, Vitest PASS, clippy limpio, build Release/Debug |

## Tareas de F7 (Voz, Alertas de Cabina y Copiloto Auditivo)

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 1 | Modelos de configuración de voz, canales y avisos tácticos (`tmc-model`) | ✅ hecha | `cac0965` | `TtsSettings`, `TtsEngineKind`, `TtsChannel`, `VoiceCue`, tests PASS |
| 2 | Cola de locución priorizada con descarte y deduplicación (`tmc-tts`) | ✅ hecha | `dc11072` | Crate `tmc-tts`, `TtsQueue`, cooldowns y prioridad |
| 3 | Motor nativo ultraligero de Windows (`tmc-tts`) | ✅ hecha | `beb81a4` | Síntesis nativa del SO (OneCore/SAPI5), < 2 MB RAM |
| 4 | Motor neuronal KokoroTTS desacoplado (`tmc-tts`) | ✅ hecha | `a9e9964` | ONNX en subproceso, carga bajo demanda y suspensión |
| 5 | Generador táctico `VoiceAdvisor` en el flujo de eventos (`tmc-core`) | ✅ hecha | `313cd12` | Detección de saltos, bioseñales, combate, minería y carrier |
| 6 | Persistencia y endpoints REST de prueba (`tmc-store`, `tmc-server`, `tmc-app`) | ✅ hecha | `9770987` | `GET /api/tts/voices`, `POST /api/tts/test`, PUT `/api/settings` |
| 7 | Catálogo de locuciones de copiloto en los 5 idiomas (`locales/`) | ✅ hecha | `b6f4bdf` | `tts.ftl` en es/en/fr/de/pt verificado por `tmc-i18n` |
| 8 | Sección de Copiloto Auditivo en `SettingsPanel.svelte` (`ui/`) | ✅ hecha | `a1bc92a` | Controles Glass HUD, sliders volumen/velocidad, prueba de voz, 401 tests PASS |
| 9 | Verificación integral, footprint y compilación Release/Debug | ✅ hecha | (commit docs) | Suite Rust PASS, Clippy 0 warnings, Release (32.5MB) + Debug (52.9MB), target saneado a 16.48 GB |

## Tareas de Tarjetas Compactas y Desplegables («TMC Glass HUD»)

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 1 | `ShipPanel.svelte`: filtro no ingenierizables, selector desplegable y acordeón | ✅ hecha | `486cf86` | Oculta módulos no ingenierizables, `<select>` compacto, tarjeta activa y lista colapsada por defecto |
| 2 | `MaterialsPanel.svelte`: selector compacto de categorías y buscador colapsable | ✅ hecha | `64fb358` | Acordeón por grado y colapso de cambios de sesión |
| 3 | `AccountsPanel.svelte`: flota remota de CAPI en selector desplegable y colapso de logs | ✅ hecha | `7b5e721` | Reemplazo de cascada de naves por desplegable, tarjeta activa y tests |
| 4 | `MissionsPanel.svelte`, `MassacrePanel`, `PassengersPanel`: selectores y colapso | ✅ hecha | `41ed856` | Selectores de destino, facción y grupos de pasajeros colapsables |
| 5 | `MarketPanel.svelte`, `CargoPanel`, `SellPanel`: selector compacto y catálogo colapsable | ✅ hecha | `86d3d5a` | Mercancía activa, catálogo completo colapsable y desglose de bodega |
| 6 | `CarrierOverviewPanel`, `ColonisationProgressPanel`, `PowerplayPanel`, `BgsPanel` | ✅ hecha | `7dccf60` | Desglose de recursos y tabla de facciones secundarias colapsables |
| 7 | Claves i18n en 5 idiomas, verificación global y build | ✅ hecha | `04f463f` | 396 Vitest PASS, svelte-check 0 err, clippy limpio, TMC.exe listo |

## Tareas de F6 (Cuentas y Sincronización Externa)

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 1 | Modelos de sincronización, credenciales y topic de estado (`tmc-model`) | ✅ hecha | `14453df` | `AccountSettings`, `AccountsStatusTopic`, payloads de EDDN, Inara, CAPI |
| 2 | Emisor de esquemas oficiales a gateway EDDN (`tmc-net`) | ✅ hecha | `25ea553` | HTTP POST `commodity/3`, `journal/1`, `outfitting/2`, `shipyard/2`, sal local |
| 3 | Cliente de sincronización EDSM (`tmc-net`) | ✅ hecha | `b377acd` | `set-logs`, `set-commander-ranks`, créditos, materiales, limitador 10/10s |
| 4 | Cliente Inara InAPI v1 con lotes y rate limit estricto (`tmc-net`) | ✅ hecha | `50788b9` | InAPI v1, batching de eventos, limitador estricto de 2 req/min |
| 5 | Flujo OAuth2 PKCE y cliente Frontier CAPI (`tmc-net`) | ✅ hecha | `196e419` | S256 PKCE, canje y refresco de tokens, consulta de flota remota |
| 6 | Almacenamiento seguro de credenciales y tokens en SQLite (`tmc-store`) | ✅ hecha | `dbf685c` | Tabla `accounts_credentials` para claves, tokens OAuth y sal de anonimización |
| 7 | Endpoint callback axum para OAuth2 de Frontier (`tmc-app`) | ✅ hecha | `3d4ee11` | Rutas `/api/auth/frontier/login` y callback loopback local |
| 8 | Módulo `accounts` en Rust (`tmc-modules`) | ✅ hecha | `87c26c2` | Orquestación de envíos, filtrado estricto Legacy y topic `accounts.status` |
| 9 | Localización completa en 5 idiomas (`locales/`) | ✅ hecha | `7db2c37` | `accounts.ftl` en es/en/fr/de/pt, privacidad y cuotas de red |
| 10 | Componentes y panel `AccountsPanel.svelte` (`ui/`) | ✅ hecha | `919b4d9` | 4 tarjetas Glass HUD, métricas de sesión, medidor de 2 req/min y flota |
| 11 | Integración en el Shell y navegación (`ui/`) | ✅ hecha | `ca14a61` | Pestaña de Cuentas, registro en catálogo y sincronización con `UI_PANELS` |
| 12 | Verificación integral, calidad, footprint y build | ✅ hecha | `639dde1` | 394 Vitest PASS, svelte-check 0 err, Rust workspace PASS, clippy limpio, TMC.exe listo |

## Rediseño visual global de TMC («TMC Glass HUD»)

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| DS | Guía del Sistema de Diseño «TMC Glass HUD» | ✅ hecha | `ea5f3b7` | `docs/design-system-glass-hud.md` enlazada en `AGENTS.md` §0 |
| AY | Guía de Desbloqueo del Ayudante (Companion) | ✅ hecha | `f266f7a` | Secuencia pedagógica de desbloqueo antes de recetas y `AGENTS.md` §0 |
| 1 | Tokens CSS y clases comunes de cabina Glass HUD | ✅ hecha | `92d4dcd` | `tokens.css`, `base.css` con superficies de cabina, medidores y acentos |
| 2 | Rediseño de `TopBar.svelte`, `Sidebar.svelte` y `Panel.svelte` | ✅ hecha | `18fbbf9` | Shell con cabecera de cabina, navegación con píldoras y paneles glass |
| 3 | Rediseño de `RouteStrip.svelte` y `SystemCard.svelte` | ✅ hecha | `209f672` | Tarjeta orbital de sistema y tira de saltos con combustible y repostaje |
| 4 | Rediseño de `SystemBodiesPanel.svelte` | ✅ hecha | `1b9f90d` | VirtualList estilizado, badges de valor económico y semáforo de sondas FSS |
| 5 | Rediseño de `ExobiologyPanel.svelte` | ✅ hecha | `47d70e9` | Stepper de muestras 1..3, tarjeta de especie activa y barra de distancia |
| 6 | Rediseño de `TargetPanel.svelte`, `ShipPanel.svelte` y `AxPanel.svelte` | ✅ hecha | `ea84bca` | Barras duales escudo/casco, estado de subsistemas y tarjetas de amenaza AX |
| 7 | Rediseño de `ProspectorPanel.svelte`, `MiningSessionPanel.svelte` y `SellPanel.svelte` | ✅ hecha | `ebc3b20` | Barras minerales ordenadas, métricas de sesión y tarjetas de venta con copia |
| 8 | Rediseño de `MarketPanel.svelte`, `CargoPanel.svelte` y `RoutePanel.svelte` | ✅ hecha | `427cb9b` | Badges de beneficio/t, capacidad de bodega y stepper de ruta comercial |
| 9 | Rediseño de `MissionsPanel.svelte`, `MassacrePanel.svelte`, `PassengersPanel.svelte` | ✅ hecha | `285c10b` | Misiones por destino, semáforo de tiempo, optimizador de masacre y cabinas |
| 10 | Verificación integral, cierre y build | ✅ hecha | `b16a168` | 378 Vitest PASS, svelte-check 0 errores, cargo test workspace PASS, `TMC.exe` listo |

## Tareas de F5 (Fleet Carrier, Colonización, Powerplay 2.0 y BGS)

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 1 | Tipado de eventos de Fleet Carrier (`tmc-model`) | ✅ hecha | `3052983` | `CarrierStats`, `CarrierJumpRequest`, `CarrierFinance`, combustible y saltos |
| 2 | Tipado de eventos de Colonización y Powerplay 2.0 (`tmc-model`) | ✅ hecha | `ce5da4a` | `ColonisationConstructionDepot`, `Powerplay`, `PowerplayMerits`, viajes |
| 3 | Cliente para endpoint de tick de BGS (`tmc-net`) | ✅ hecha | `9c0973f` | `https://tick.edcd.io/api/ticks`, caché 30m, degradación offline |
| 4 | Módulo `carrier` en Rust (`tmc-modules`) | ✅ hecha | `d22a65e` | Telemetría, combustible, saltos de 500 al, cuenta atrás y semáforo financiero |
| 5 | Módulo `colonisation` en Rust (`tmc-modules`) | ✅ hecha | `4c2a185` | Depósitos en obra, viajes restantes según bodega y sourcing de compras |
| 6 | Módulo `powerplay` en Rust (`tmc-modules`) | ✅ hecha | `6a2aaae` | Méritos de sesión, tasa méritos/h, fórmula oficial de rangos y tick semanal |
| 7 | Módulo `bgs` en Rust (`tmc-modules`) | ✅ hecha | `db45982` | Facciones locales, influencias ordenadas, estados y hora del tick galáctico |
| 8 | Cadenas y localización en los 5 idiomas (`locales/`) | ✅ hecha | `2b4c687` | `carrier.ftl`, `colonisation.ftl`, `powerplay.ftl`, `bgs.ftl` en es/en/fr/de/pt |
| 9 | Componentes y paneles Glass HUD de Fleet Carrier | ✅ hecha | `772b4b0` | Medidor dual de tritio, cuenta atrás de 15 min y tarjeta de mantenimiento |
| 10 | Paneles Glass HUD de Colonización | ✅ hecha | `b252aea` | Progreso de construcción, viajes restantes y compras cercanas con copia |
| 11 | Paneles Glass HUD de Powerplay 2.0 y BGS | ✅ hecha | `7d578fa` | Stepper de rango, méritos/h, barras de influencia y tick galáctico |
| 12 | Integración de disposiciones en el Shell y Auto-companion | ✅ hecha | `72ba329` | Activación automática en Carrier, depósitos de colonización y Powerplay |
| 13 | Verificación integral, calidad, footprint y build | ✅ hecha | `3a360ae` | 392 Vitest PASS, svelte-check 0 errores, 1.380+ Rust PASS, TMC.exe listo |

## Tareas de F4

Leyenda: ⬜ pendiente · 🔄 en curso · ✅ hecha · ⛔ bloqueada · ⏭️ omitida

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 0 | Spike de datos y licencias (A→B) y traducciones oficiales | ✅ hecha | d8a6267 | Ninguna fuente A; B para todo (pendiente del propietario) |
| 1 | Spike de diario — formatos en diarios del propietario (agregados) | ✅ hecha | 46897f9 | Agregados; stub ShipLocker→releer ShipLocker.json; Category $MICRORESOURCE_… |
| 2 | Spike de red — Ardent comerciantes de materiales y broker; respaldo Spansh | ✅ hecha | 3e77a34 | Ardent sin tipo; Spansh por tipo; fixtures sintéticos |
| 3 | Aprobaciones del propietario tras los spikes | ✅ hecha | — | Fuente: EDEngineer (MIT) para materiales y nombres; B para el resto. Pendientes menores: nombre ES del blueprint y experimental de special_engine_overloaded (se resuelven con localization.json de EDEngineer) |
| 4 | `tmc-model` — eventos de materiales | ✅ hecha | ba60cb7 | Revisión aprobada (2 menores aparcados) |
| 5 | `tmc-model` — eventos de ingenieros, `Loadout` ampliado, taquilla y mochila | ✅ hecha | 93a1019 | Revisión aprobada (bajas aparcadas a Tasks 13/14) |
| 6 | `tmc-model` + `tmc-app` — servicios cercanos nuevos y tiradas por defecto | ✅ hecha | bb66321 | Revisión aprobada; F2 intacto |
| 7 | `tmc-net` — comerciantes de materiales y broker | ✅ hecha | c24b770 | Revisión aprobada; Ardent sin tipo, Spansh por tipo |
| 8 | Datos estáticos `materials.json` y `trader_rules.json` | ✅ hecha | 671fa03, 7f555d1, 198710b, a46dec9, dd35f8a, e0d46ef | Catálogo importado: 139 materiales; 13 Guardian sin verificar; tests globales PASS |
| 9 | Datos estáticos `blueprints.json` | ✅ hecha | 35598f5, e3ad29f | Step 6 completado en C8 con catálogo completo (89 blueprints, 87 experimentales, 15 variantes) |
| 10 | Datos estáticos `engineers.json` | ✅ hecha | fa964e5, 6a8c626, f313f63, e3ad29f | Step 6 completado en C8 con 38 ingenieros, mapa auxiliar EDDI y capacidades completas |
| 11 | i18n — `engineering.ftl` y `engineering-data.ftl` | ✅ hecha | 5b66a98, a84d82c | Nombres EDEngineer MIT; Motor Sucio e Impulsores de Arrastre resueltos; revisión aprobada |
| 12 | `engineering` — inventario de materiales y cambios de la sesión | ✅ hecha | 335edf6 | Revisión aprobada; fixture engineering_session.log |
| 13 | `engineering` — taquilla de a pie | ✅ hecha | 35b98ab | Revisión aprobada; stub no vacía; listas parciales |
| 14 | `engineering` — ingenieros | ✅ hecha | 06e1fec | Revisión aprobada; posición None hasta salto |
| 15 | `engineering` — mi nave modificada | ✅ hecha | 5b9d0a0 | Revisión aprobada |
| 16 | `engineering` — cálculo de ingenierizar | ✅ hecha | 672dd97 | Revisión aprobada; números comprobados a mano |
| 17 | `engineering` — planificador de intercambios | ✅ hecha | ae0081a | Revisión aprobada; tasas del juego |
| 18 | `engineering` — lista de la compra, reservas y persistencia | ✅ hecha | 6646975, 47f910d, 7f71e31 | Revisión aprobada tras 2 rondas (escritura atómica en tmc-store) |
| 19 | `engineering` — dónde conseguirlo, ingeniero recomendado y avisos | ✅ hecha | 4672cdc | Revisión aprobada |
| 20 | `engineering` — módulo, topics, relevancia, red, almacén y registro | ✅ hecha | 9904133, 1fc8b74, a4c5f3d | Paginación aprobada; 1.356 Rust/238 UI PASS, probes activos, ambas revisiones |
| 21 | UI — «Materiales» | ✅ hecha | 04e5ea4 | 20 tests nuevos; 258 UI/1.356 Rust PASS; ambas revisiones y QA visual |
| 22 | UI — «Ingenieros» | ✅ hecha | 88836b7 | 16 tests nuevos; 274 UI/1.356 Rust PASS; ambas revisiones y QA visual |
| 23 | UI — «Mi nave» e «Ingenierizar un módulo» | ✅ hecha | d644a6d | 19 tests nuevos; 293 UI/1.356 Rust PASS; ambas revisiones y QA visual |
| 24 | UI — «Lista de la compra», «Intercambios», «Dónde conseguirlo» y avisos | ✅ hecha | 65784b6 | 16 tests nuevos; 309 UI/1.356 Rust PASS; ambas revisiones y QA visual |
| 25 | UI — disposición «Ingeniería», prioridad, Auto-companion y tiradas | ✅ hecha | 4c70ecb | 343 UI/1.356 Rust PASS; ambas revisiones y QA visual |
| 26 | Fixtures sintéticos y pruebas de extremo a extremo | ✅ hecha | 8a70048 | 1.362 Rust/343 UI/7 Playwright/1 Node PASS; ambas revisiones |
| 27 | `tmc-headless eval` — conciliación del inventario | ✅ hecha | be03e09 | 1.369 Rust/343 UI PASS; ambas revisiones; 201/201 exactos en historial, n4 insuficiente |
| 28 | Avisos de terceros, consumo, verificación de los 10 criterios y cierre | ✅ hecha | 438e572 | Avisos de terceros, clippy release target-f4 PASS, footprint 122,20 MB / 0,00% PASS, 10/10 criterios de la spec §8 verificados |

## Ampliación aprobada del catálogo F4

| Unidad | Estado | Commit | Siguiente paso / evidencia |
|---|---|---|---|
| C0 | ✅ hecha | 956248d | Spec/plan/enmienda ratificados, revisión aprobada; 11 links/cercas/diff PASS |
| C1 | ✅ hecha | c308ce9 | Staging reproducible; 1.167 constructores (943 receta, 83 exp., 15 variantes, 116 fuera, 2 obsoletos, 8 sin resolver); 38/38 ingenieros; 1.899 Merc Coins; revisión aprobada |
| C2 | ✅ hecha | 7ac1b54 | Recetas por contexto, política de base y alias; staging se lee sin pérdidas; revisión aprobada |
| C3 | ✅ hecha | 9cdcdd1 | Coste con base explícita; Merc Coins aparte; bloqueos explícitos; revisión aprobada |
| C4 | ✅ hecha | 95abc6f, 21c667c | Capacidades por módulo/grado; recomendación exige todos los grados; grados requeridos con la normalización del coste; revisión aprobada tras 1 ronda |
| C5 | ✅ hecha | e9c214d | Objetivos bloqueados nunca completables; Merc Coins reservadas por orden sin saldo inventado; avisos por `completable`; revisión aprobada |
| C6 | ✅ hecha | 513d481 | REST paginado (catálogo, ingenieros, flujo) con las capas de seguridad existentes; revisión aprobada; campos pesados legacy se quitan en C7 |
| C7 | ✅ hecha | c1c7c30, 6dbbb69 | UI sobre páginas; campos pesados fuera de los topics; Merc Coins y bloqueos visibles; capacidades por id sin recargas; revisión aprobada tras 1 ronda; QA visual 2560/800 pendiente (C9) |
| C8 | ✅ hecha | e3ad29f | Catálogo completo activo; 0 sin resolver; 38 ingenieros, 141 materiales; i18n 5 idiomas; 15 node, 1.370+ Rust, 375 UI PASS |
| C9 | ✅ hecha | 2b6dba9 | Revalidada integración e2e: 7 Playwright e2e PASS, e2e-server PASS, 375 UI PASS, 1.370+ Rust PASS |
| C10 | ✅ hecha | 438e572 | Task 28 ejecutada, 10/10 criterios verificados y F4 completada |

## Rediseño visual de Ingeniería (Opción 2) y módulos vírgenes

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| 1 | Textos e i18n en los 5 idiomas | ✅ hecha | 1a81c64 | 17 claves nuevas en `locales/{es,en,fr,de,pt}/engineering.ftl` |
| 2 | Rediseño visual de `MaterialsPanel.svelte` | ✅ hecha | d404bca | Buscador reactivo, medidor de capacidad de categoría, barras de llenado y badge MÁXIMO |
| 3 | Rediseño visual de `EngineersPanel.svelte` | ✅ hecha | acf0683 | Filtros Todos/Desbloqueados/Pendientes, estrellas de grado y badges de estado |
| 4 | Stepper de grados y asistente integrado en `EngineerFlow` y `ShoppingPanel` | ✅ hecha | 31bcc81 | Stepper horizontal G1..G5, semáforo de existencias, solución de intercambio directa y botón de ruta |
| Fix | Reconocimiento de módulos vírgenes (stock) | ✅ hecha | 44fc0da | Prefijos de módulo importados en `blueprints.json` y fallback de slot core en `ship.rs` |
| 5 | Verificación completa y compilación del binario debug | ✅ hecha | 5b4c24d | 378/378 Vitest PASS, svelte-check 0 errores, cargo test verde, binario `target/debug/TMC.exe` listo |

## Tareas de diagnóstico debug

| # | Tarea | Estado | Commit | Notas |
|---|---|---|---|---|
| D1 | Contrato y redacción | ✅ hecha | e6c5d09 | 13 diagnósticos TDD; ambas revisiones; 781 Rust, 144 UI, release noeval, fmt/clippy/check/build PASS |
| D2 | Writer circular y lifecycle | ✅ hecha | ed0611e | 14 pruebas; ambas revisiones; 795 Rust, fmt/clippy y clippy release PASS |
| D3 | Fuentes y persistencia | ✅ hecha | 44e8a9e + 91cc1dc (fix volumen) | 804 Rust PASS (1 ignorado), fmt/clippy y clippy release PASS |
| D4 | Actor y módulos | ✅ hecha | 42c1ccc | 811 Rust PASS (1 ignorado), fmt/clippy y clippy release PASS |
| D2b | Escritor SQLite por apartados | ✅ hecha | 58696b7 + fix de revisión | Sustituye JSONL (decisión propietario 2026-10-20); 75 tests tmc-app, 815 Rust PASS (1 ignorado), fmt/clippy y clippy release PASS |
| D5 | Red, HTTP, WS, auth y acciones | ✅ hecha | b6e139b | 824 Rust PASS (1 ignorado), fmt/clippy y clippy release PASS; sin test WS dedicado |
| D6 | UI | ✅ hecha | 5a81eff | 834 Rust PASS/1 ignorado, 155 UI PASS |
| D7a | Verificación previa (checks, replay, build debug) | ✅ hecha | 713a8f4 | 6 ejecuciones workspace verdes |
| D7b | Integración en F3 | ✅ hecha | 6e5e97a (merge) + a1c2bb3 | EDDN/comercio/endpoints F3 en diagnóstico; 999 Rust PASS ×2, clippy release PASS, 160 UI PASS |

## Fases

| Fase | Contenido | Estado |
|---|---|---|
| F0 | Cimientos (lectura, estado, SQLite, servidor, UI base, panel general, instalador) | ✅ completada y fusionada (2b953e6) |
| F1 | Navegación, exploración, exobiología | ✅ completada y fusionada (b96fecf) |
| F2 | Cazarrecompensas/combate (incl. AX), minería | ✅ completada y fusionada (e069a14) |
| F3 | Comercio, misiones y pasajeros, recepción EDDN | ✅ completada y fusionada (6dc09d9) |
| F4 | Ingenieros y materiales | ✅ completada y fusionada en main |
| F5 | Fleet carrier, colonización, Powerplay 2.0, BGS | ✅ completada y fusionada en main |
| F6 | Cuentas: envío EDDN, EDSM, Inara, CAPI | 🔄 en curso (spec, plan y maqueta interactiva listos) |
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
| 17 | `trade` — ruta comercial con avance automático | ✅ hecha | 08aacf3 | Ruta comercial: avance al atracar en la parada, completada, portapapeles; decide! numéricos |
| 18 | `trade` — sesión de comercio y buscar mercancía | ✅ hecha | 79ebd2b, a3de1d5 | Sesión de comercio (coste exacto/AvgPricePaid, sin coste contado, rango) y búsqueda con reloj inyectado |
| 19 | `trade` — módulo, topics, relevancia, red, almacén y registro | ✅ hecha | 1d90c65, 7dcb996 | Módulo trade: 6 topics sin reloj, relevancia, red, almacén, registro, trade. en MODULE_PREFIXES; Market.json antes de Docked |
| 20 | `mining` — «Dónde vender» con la tabla de precios | ✅ hecha | 7c54960, b63686f | Dónde vender con tabla de precios + EDDN; sesión de minería sin tasas dependientes del reloj (UI calcula) |
| 21 | UI — «Mercado actual» y «Mi carga» | ✅ hecha | 05a0667, 155a727 | Mercado actual y Mi carga; formato común de créditos |
| 22 | UI — «Ruta comercial», «Sesión» y «Buscar mercancía» | ✅ hecha | 1b8a522, 155a727, 01c347a | Ruta (saltos, aviso de copiado), Sesión y Buscar; test de minería determinista |
| 23 | `missions` — libro de misiones y reconstrucción | ✅ hecha | c7bd6bd, ae8fbc6 | Libro de misiones y reconstrucción desde historial; fallidas/completadas al reconciliar |
| 24 | `missions` — pila de masacre | ✅ hecha | d654f97, 10f49af | Pila de masacre con la regla exacta de §10.10; fixture massacre_session.log |
| 25 | `missions` — avisos, recompensas de la sesión y objetivos comunitarios | ✅ hecha | 70645b6, 250119f | Avisos (caducidad, carga al llegar al destino), recompensas acumuladas, objetivos comunitarios |
| 26 | `missions` — pasajeros y cabinas | ✅ hecha | 88df42a, 250119f | Pasajeros por destino (VIP/buscado) y cabinas por clase (ocupación agregada) |
| 27 | `missions` — turismo: ruta óptima, cr/al y veredicto | ✅ hecha | 4cfa8cd, 6a2627e | Turismo: ruta óptima (exacta ≤8, heurística), cr/al, veredicto; robusto ante datos inválidos |
| 28 | `missions` — módulo, coordenadas, topics, relevancia y registro | ✅ hecha | b0a340c, 7d37024 | Módulo de misiones: libro+masacre+avisos+pasajeros+turismo cableados; turismo tras reiniciar; coordenadas con reintento |
| 29 | UI — «Misiones activas», «Masacre» y «Avisos» | ✅ hecha | 17fd079, fd69ee4 | Misiones activas por destino, masacre (vista de pila), avisos |
| 30 | UI — «Pasajeros», «Turismo», «Recompensas» y «Objetivos comunitarios» | ✅ hecha | abaf317, fd69ee4, cda1014 | Pasajeros, turismo (veredicto con contrato real), recompensas, objetivos comunitarios; contrato JSON Rust→UI |
| 31 | UI — disposiciones «Comercio» y «Misiones», prioridad y ajustes | ✅ hecha | 2d449e9, b6dbdde | Disposiciones Comercio/Misiones con prioridad; ajustes EDDN/turismo; relevancia solo con carga comercial |
| 32 | fixtures sintéticos y pruebas de extremo a extremo | ✅ hecha | 502804f, 69f83c5, 7fc00e0 | Fixtures F3, contratos trade/missions, ráfaga EDDN (p95 299 ms, ≤1 consulta/10 s, corruptos), 14 Playwright; hooks e2e solo debug y estrictos |
| 33 | `tmc-headless eval` — conciliación de coste pagado y de masacre | ✅ hecha | f262180, 0bda26e, 939de0b | Eval coste pagado y masacre (historial: 2/2 y 4/4, INSUFICIENTE por n<5); release sin avisos |
| 34 | avisos, consumo con EDDN, verificación de los 11 criterios y cierre | ✅ hecha | 55c5407 | 10/11 criterios PASS; 10 PARCIAL (núcleo con EDDN 34 MB/0,05 % CPU; ventana pendiente del propietario) |

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

- [ ] **F4 · Catálogos completos (2026-10-22):** investigación web autorizada por el propietario realizada: [fuentes y propuesta](docs/research/notes/2026-10-22-catalogos-f4-web.md). Candidato nuevo: EDDiscovery/EliteDangerousCore, archivos con cabecera Apache-2.0, 38 ingenieros y recetas por módulo/grado, revisión 7d30920. El propietario aprueba EDDiscovery como fuente A, conservando atribución y verified:false. El propietario también aprueba ejecutar la [enmienda técnica concreta](docs/plans/2026-10-22-tmc-f4-catalogo-completo-propuesta.md) y el mapa auxiliar EDDI Apache: Merc Coins, bases G2+, grados por módulo/ingeniero, paginación y mapa auxiliar de IDs EDDI Apache; el catálogo provisional no los representa. No se ha importado nada de la nueva búsqueda. Pendiente ejecutar la ampliación aprobada tras Tasks 26–27; no hay pregunta pendiente del propietario sobre estas fuentes/interfaces. F4 no completa con 2 blueprints/1 experimental/5 ingenieros provisionales.

- [x] **F4 · Task 20 — contrato excede el límite aprobado; solución autorizada (2026-10-21):** revisión y prueba real demuestran `engineering.goals` de **41.116 bytes** con 20 objetivos `engine_dirty` G5 + experimental, tiradas `[10;5]`, excedentes de materiales ajenos a la receta y 215 propuestas; el plan exige `<16.384`. También `engineering.locker` ocupa **16.832 bytes** con 200 recursos localizados (50 por sección). Sin experimental: 30.986 bytes; FSD: 35.084. Resuelto en `a4c5f3d`: summaries objetivos 2.471 B y taquilla 110 B; páginas de objetivos hasta 16.234 B y taquilla 4.439 B, todas las filas/propuestas preservadas. Ambos probes reactivados PASS; Task 20 cerrada tras revisiones/checks. **Decisión aprobada:** resumen WS con revisión y detalle REST paginado por bytes, arquitectura §7; interfaces de Tasks 20/21/24/26 sincronizadas. El propietario confirma «Sí, hazlo y continúa»: se aprueba resumen WS con detalles REST paginados por bytes. Implementación completada y verificada; no hace falta otra aprobación para esta solución. Evidencia y propuesta en Desviaciones del plan activo.


- [x] **F4 · Discrepancia de seguimiento (2026-10-21, Codex), autorización recibida:** `status.md` señala Task 20 en curso sin commit y conserva cabecera/fase «ejecución sin empezar»/«plan escrito», pero `HEAD` es `99041335e8a6bc48e4b5483e640537b18f4c5d12` (`feat(engineering): módulo de ingeniería con topics, relevancia, red, almacén y registro`, 22 archivos). El commit incluye implementación, pruebas y fixture de contrato UI de Task 20; no modifica `status.md`. Rama correcta `fase/f4-ingenieros-materiales`; sin modificaciones rastreadas al iniciar, con `.claude/` y `.tmp/` no rastreados que se preservan. No se afirma que Task 20 esté terminada: faltan revisión y verificación observadas en esta sesión. Se detiene ejecución según AGENTS.md §1. **Resolución:** el propietario ordena revisar si está implementada COMPLETAMENTE, anotarlo y continuar si lo está. Revisión y checks terminados: carencias iniciales corregidas en `1fc8b74` y paginación aprobada completada en `a4c5f3d`.

- [x] **F3 · Criterio 10:** ✅ medido 2026-10-20 con ventana (106,4 / 111,5 MB, 0 % CPU). Recomendado: comprobar con EDDN real durante una sesión de juego. Antes: cierra TMC.exe y ejecuta `CARGO_TARGET_DIR=target-f3 cargo build --release -p tmc-desktop` y `pwsh scripts/measure-footprint.ps1 -Exe target-f3/release/TMC.exe -JournalDir .tmp/footprint-f3-trade -FixturePath tests/fixtures/journal/trade_session.log -ShowWindow -Seconds 120` (y con `missions_session.log`); o autoriza al agente a hacerlo. Límite < 150 MB y < 1 % CPU.

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
- 2026-10-22 · Propietario: comprobado en el juego que Stripped Down de armas de nave (`special_weapon_lightweight`) pide 5 Carbon (coincide con EDDiscovery/EDEngineer; Coriolis 3 descartado).
- 2026-10-22 · Claude (C8): las 8 filas sin resolver se reasignan con tabla de overrides citada (`docs/research/notes/2026-10-22-filas-sin-resolver-c8.md`): DSS largo alcance → SurfaceScanner; 4 experimentales de arma → 11 tipos de arma de nave; `verified:false`.
- 2026-10-22 · Claude (C1): las 8 filas `unresolved` no se expanden ni reasignan sin evidencia externa documentada (C8); compras del Tech Broker quedan `out_of_scope` salvo decisión del propietario; manifiesto con texto original se mantiene en staging (posible recorte en C8).
- 2026-10-22 · Propietario: «Aprobar y ejecutar la enmienda» del catálogo completo y «Aprobar el mapa auxiliar de IDs» de EDDI d3b964e (Apache-2.0, solo 38 pares ID/nombre). Autorizadas ratificación de spec/plan e implementación por fases con atribución, Merc Coins sin saldo inventado, bases G2+, capacidades por módulo, paginación y compatibilidad de objetivos.
- 2026-10-22 · Propietario: «Aprobar EDDiscovery como fuente A». Se autoriza usar los archivos Apache-2.0 fijados a 7d30920 para sustituir los catálogos provisionales de recetas e ingenieros, con atribución y verified:false. Preparación de enmienda técnica para Operations/grados por módulo en curso; todavía sin importación.
- 2026-10-22 · Revisión de completitud: Tasks 9 y 10 se muestran bloqueadas para su Step 6, aunque el código provisional esté revisado. Recuento corregido a 24/29 tareas completas; no se equipara el cierre de sus pasos 1–5 con un catálogo completo. Fuente Apache propuesta y límites documentados antes de importar.
- 2026-10-21 · Propietario aprueba resolver el límite de objetivos/taquilla con resumen por WebSocket y detalles REST paginados por bytes, y continuar el plan tras verificar Task 20.
- 2026-10-21 · Propietario autoriza reconciliar seguimiento y verificar COMPLETAMENTE Task 20 antes de continuar; revisión de spec detecta carencias reales, se corrigen con TDD sin iniciar Task 21 todavía.
- 2026-10-21 · Propietario (Task 3 F4): EDEngineer (MIT, commit 2cadd66) como fuente A para el catálogo de materiales y los nombres traducidos (localization.json); opción B (recopilación propia, `verified:false`) para blueprints/recetas, ingenieros, reglas del comerciante y pistas. Riesgo anotado por el spike: los datos de EDEngineer derivan de coriolis-data (IP de Frontier, sin licencia propia); el propietario asume la decisión. Atribución en `THIRD_PARTY_NOTICES.md` al importar.
- 2026-10-21 · Propietario: interruptor «Auto-companion» (encendido por defecto). Apagado, la app no cambia sola de disposición ni de tarjeta de contexto y recuerda la última elegida; spec `docs/specs/2026-10-21-tmc-auto-companion-design.md`, fusionado en `main`.

- 2026-10-20 · D7b: regla global en la spec de diagnóstico — todo topic, acción, variante NetQuery/NetOutcome, endpoint o campo de configuración nuevo se registra en las listas permitidas de diagnóstico y se cubre con un test canario.

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

- 2026-10-24 18:05 (Europe/Madrid) · Antigravity · Fase 9 (Crimen, Notoriedad y Piratería - Law & Disorder) completada íntegramente y verificada:
  - **Modelos y serialización (`tmc-model`):** Creado `crime.rs` con `CrimeType`, `CommitCrimeEvent`, `CrimeVictimEvent`, `PayFinesEvent`, `PayBountiesEvent`, `FactionFineBounty`, `StolenCargoItem`, `NearbyFacility`, `SystemSecurityLevel` y topic `CrimeStatusTopic`. Registrado en `diagnostics.rs` y eventos del diario.
  - **Módulo de lógica criminal (`tmc-modules`):** `CrimeModule` con decaimiento de notoriedad en vuelo (1 nivel cada 7.200 s), congelación del temporizador al atracar en estación (`Flags::DOCKED`), acumulación y liquidación de multas/recompensas, evaluación de bodega robada desde `Cargo`, y resolución de Interstellar Factors y Mercados Negros cercanos mediante `NearbyService::BlackMarket`.
  - **Copiloto auditivo táctico (`tmc-core`):** Avisos de voz en `VoiceAdvisor` ante ganancia de notoriedad (`tts-crime-notoriety-gain`), disipación de notoriedad (`tts-crime-notoriety-cleared`), contrabando al saltar a sistemas con seguridad (`tts-crime-illegal-cargo`) e interdicción de fuerzas de seguridad (`tts-crime-police-interdiction`).
  - **Localización completa (`locales/`):** Archivo `crime.ftl` redactado y validado en los 5 idiomas (`es`, `en`, `fr`, `de`, `pt`). 18/18 tests en `tmc-i18n` PASS.
  - **Panel Glass HUD (`ui/`):** Componente `CrimePanel.svelte` con medidor de notoriedad de 10 segmentos, temporizador en vivo de decaimiento vs pausa al atracar, Guía de Limpieza de Nombre del Ayudante (bloqueo explicativo si notoriedad > 0, localización y botón de 1-clic para copiar al portapapeles el Interstellar Factor o Mercado Negro más cercano), desglose de mercancías robadas con consejos de aproximación sigilosa (Silent Running, velocidad > 200 m/s), desglose por facción y termómetro de seguridad policial.
  - **Tests unitarios Vitest:** Suite `ui/tests/crime.test.ts` con 7 tests verificando todos los estados legales y reactividad. Suite completa: 24 archivos, 408 tests PASS.
  - **Verificación integral y binarios:** Suite Rust completa PASS en todo el workspace (1.400+ tests), `cargo clippy` sin advertencias, `npm run check` 0 errores / 0 avisos, `npm run build` OK, y binarios interactivos `target/release/TMC.exe` (32.7 MB) y `target/debug/TMC.exe` (53.1 MB) verificados. Saneamiento estricto de `target/` y sin procesos daemon.

- 2026-10-24 17:05 (Europe/Madrid) · Antigravity · Fase 7 (Voz, Alertas de Cabina y Copiloto Auditivo) completada íntegramente y verificada:
  - **Arquitectura de voz híbrida nativa y neuronal:**
    - Crate `tmc-tts` con trait `TtsEngine`, cola priorizada `TtsQueue` (Emergency > Tactical > Informational) con descarte por antigüedad (> 4s), supresión de duplicados y cooldowns por canal.
    - Motor nativo `NativeEngine` en `crates/tmc-tts/src/native.rs` utilizando COM Windows SAPI5 / OneCore (`ISpVoice`) en subproceso con purga instantánea y huella de memoria < 2 MB de RAM.
    - Motor neuronal `KokoroEngine` en `crates/tmc-tts/src/kokoro.rs` ejecutado en proceso desacoplado con suspensión automática de recursos tras 60 s de inactividad, catálogo de voces (es, en, fr, pt) y fallback transparente a voces nativas para el idioma alemán y cuando el modelo no esté descargado.
  - **Generador táctico en el flujo de eventos:**
    - `VoiceAdvisor` en `crates/tmc-core` conectado al bucle del actor de diario/estado.
    - Avisos tácticos en tiempo real para saltos estelares (estrella repostable vs no repostable), recuento de señales biológicas, muestras exobiológicas (1/3, 2/3, 3/3), caída y restauración de escudos, sobrecalentamiento crítico, alertas de integridad de casco, asteroides valiosos en prospección y saltos inminentes de Fleet Carrier.
  - **Persistencia, API REST e i18n:**
    - Endpoints REST `GET /api/tts/voices`, `POST /api/tts/test` e integración en `AppActions` (`tts.voices`, `tts.test`).
    - Catálogo completo de locuciones de voz y controles de cabina en los 5 idiomas (`locales/{es,en,fr,de,pt}/tts.ftl`), 100% verificado por los tests de `tmc-i18n`.
  - **Interfaz de usuario («TMC Glass HUD»):**
    - Controles de copiloto de voz en `Settings.svelte` con conmutador general, selector de motor, selector de voces del sistema, controles deslizantes de volumen (0-100%) y velocidad (80-150%), conmutadores de canales y botón de prueba auditiva con estado de reproducción.
    - 5 tests unitarios en `ui/tests/settings-tts.test.ts` (401/401 tests Vitest PASS, `svelte-check` 0 errores / 0 avisos).
  - **Verificación global y compilación:**
    - Tests de suite Rust en el workspace 100% PASS (118 en tmc-app, 50 en tmc-store, 42 en tmc-core, 35 en tmc-server, 9 en tmc-tts, 18 en tmc-i18n).
    - `cargo clippy --workspace --all-targets -- -D warnings` limpio.
    - Compilación verificada de binarios `target/release/TMC.exe` (32.5 MB) y `target/debug/TMC.exe` (52.9 MB).
    - Saneamiento obligatorio de disco: `target/` reducido de 36 GB a 16.48 GB purgando artefactos de test obsoletos y caché incremental.

- 2026-10-24 12:45 (Europe/Madrid) · Antigravity · Resolución de 4 peticiones del comandante: login CAPI en navegador nativo, toggle/repliegue de acordeón de ingenieros, estabilización de flicker al arrancar y nuevo panel exhaustivo de nave:
  - **Apertura de navegador para CAPI:** Endpoint `/api/auth/frontier/login?open=true` en `tmc-server/src/rest.rs` que invoca el navegador nativo del SO (`rundll32 url.dll,FileProtocolHandler` en Windows, `open` en macOS, `xdg-open` en Linux) con fallback en UI y estado visual reactivo «Abriendo navegador...».
  - **Repliegue / Toggle de Ingenieros:** En `tmc-modules/engineering`, el manejador `ACTION_SELECT_ENGINEER` ahora permite deselección (`id: null`) y togglea a `None` si se vuelve a pulsar sobre el ingeniero ya seleccionado.
  - **Estabilización de Arranque (Anti-Flicker):** En `App.svelte`, se detecta si el diario está en fase de replay (`topic('core.game')?.replaying === true`). Mientras se procesan los eventos históricos en los primeros 1–3 segundos, las transiciones automáticas de pestañas quedan congeladas, evitando el baile frenético de pantallas.
  - **Panel Exhaustivo de Nave (`ShipPanel.svelte`):** Nuevo módulo independiente `ship` con acceso directo en la barra lateral (`🚀`). Incluye:
    - 4 Cockpit Gauges (Casco, Escudos, Combustible principal/reserva y Carga), masa en vacío y coste de recompra.
    - 3 Pestañas operativas especializadas: Exploración (salto máx., masa, combustible, Fuel Scoop, Guardian Booster, DSS, AFMU, SRV), Combate (escudos, casco, distribución Pips SYS/ENG/WPN, temperatura, puntos de anclaje con munición en cargador y reserva, defensas utilitarias: boosters, chaff, point defence, heat sinks) y Carga/Minería (capacidad, racks, refinería, controladores de drones, inventario de carga).
    - Desglose exhaustivo módulo por módulo con buscador en tiempo real, filtro por categorías (Anclajes, Módulos primarios, Módulos opcionales, Utilidades), estado On/Off, barras de salud, prioridad de energía P1..P5, insignias oficiales de ingeniería G1..G5 con hexágonos, efecto experimental y acordeón de modificadores.
    - Formateador y traductor de nombres de slots e items de Elite (`shipFormatters.ts`).
    - Claves i18n sincronizadas en los 5 idiomas (`locales/{es,en,fr,de,pt}/*.ftl`).
  - **Verificación completa:** 1.400+ tests Rust PASS en todo el workspace, 396 tests Vitest PASS, `svelte-check` 0 errores / 0 avisos, `cargo clippy` sin advertencias, y compilación limpia de binarios interactivos `target/release/TMC.exe` y `target/debug/TMC.exe`.


- 2026-10-24 03:15 (Europe/Madrid) · Antigravity · Corrección del comportamiento anómalo al seleccionar ingeniero en `EngineersPanel`:
  - Identificada causa raíz: en `crates/tmc-modules/src/engineering/mod.rs`, `dependencies_changed` comprobaba erróneamente `semantic_changed(TOPIC_ENGINEERS, &engineers)`. Dado que el topic contiene `selected_id`, cualquier clic en un ingeniero incrementaba `revision`. En el frontend, `createSectionsReader` (al no tener `cacheKey` estático para la lista de ingenieros) interpretaba la nueva revisión como una invalidación total, vaciando todas las filas (`items = []`, `ready = false`) y mostrando un parpadeo de "Cargando..." mientras se reconsultaba la API.
  - Corrección en backend: se eliminó `semantic_changed(TOPIC_ENGINEERS, &engineers)` de `dependencies_changed`, ya que seleccionar un ingeniero es un estado de selección y no muta los datos paginados de la galaxia (`self.engineer_rows` sigue vigilado por cambios reales de desbloqueo/rango).
  - Corrección en UI (`EngineersPanel.svelte`):
    - Los detalles del ingeniero seleccionado (`data-testid="engineer-details"`) ahora se abren en acordeón expandible **in situ** directamente dentro de la tarjeta clicada, evitando el desplazamiento donde aparecía al final de la página tras 35 ingenieros.
    - Se agregaron chevrons (▲/▼), interacción clicable completa y botones con `type="button"`.
    - Se añadieron cabeceras colapsables para las secciones de "Ingenieros de naves" e "Ingenieros a pie".
  - Verificación: 162 tests Rust de ingeniería PASS, 396 tests Vitest PASS, `svelte-check` 0 errores/0 advertencias, Vite build OK, clippy sin advertencias y `TMC.exe` de release generado.

- 2026-10-24 02:05 (Europe/Madrid) · Antigravity · Ejecución completa de la iniciativa de tarjetas compactas y desplegables en toda la aplicación:
  - Tareas 2 a 6 ejecutadas con commits individuales (`64fb358`, `7b5e721`, `41ed856`, `86d3d5a`, `7dccf60`):
    - `MaterialsPanel.svelte`: acordeones por grado de inventario y colapso de cambios de sesión.
    - `AccountsPanel.svelte`: selector desplegable `<select data-testid="remote-fleet-select">`, tarjeta activa compacta y flota colapsable por defecto.
    - `MissionsPanel.svelte`, `MassacrePanel.svelte`, `PassengersPanel.svelte`: selectores compactos de destino y facción, vistas compactas y acordeones.
    - `MarketPanel.svelte`, `CargoPanel.svelte`, `SellPanel.svelte`: selector de mercancía activa con tarjeta resumen, catálogo completo colapsable y desglose de bodega.
    - `ColonisationProgressPanel.svelte`, `BgsPanel.svelte`: desgloses de recursos y tablas de facciones secundarias colapsables por defecto.
  - Verificación global: 396/396 Vitest PASS, svelte-check 0 errores/0 advertencias, clippy workspace sin warnings, 817 tests Rust en `tmc-modules` y 18 en `tmc-i18n` PASS, `dist/` reconstruido y `TMC.exe` compilado y lanzado.

- 2026-10-24 01:30 (Europe/Madrid) · Antigravity · Implementación de mejora ergonómica en ingeniería y plan de compacidad global:
  - `ShipPanel.svelte`: Módulos no ingenierizables filtrados y excluidos del renderizado. Selector desplegable compacto `<select data-testid="ship-module-select">` con el módulo activo y sus datos clave.
  - Tarjeta activa compacta con slot, nombre, plano, grado y calidad. Acordeón de lista completa colapsado por defecto (`[ ▼ Ver lista ({total}) ]`).
  - Actualizado test 364 en `engineering.test.ts` (verifica exclusión de módulos no soportados) y añadido test para el selector desplegable y alternancia de colapsado.
  - Sistema de diseño: añadido §4.8 a `docs/design-system-glass-hud.md` estableciendo la norma de tarjetas compactas y desplegables por defecto en toda la cabina.
  - Creadas spec formal `docs/specs/2026-10-24-tmc-ui-tarjetas-compactas-desplegables-design.md` y plan de ejecución `docs/plans/2026-10-24-tmc-ui-tarjetas-compactas-desplegables.md` (7 tareas en 6 bloques abarcando todos los módulos).
  - Verificación: 394/394 Vitest PASS (107/107 en ingeniería), svelte-check 0 errores/0 avisos, clippy limpio, 817 tests de `tmc-modules` PASS, build de producción `ui/dist` OK y `target/debug/TMC.exe` compilado exitosamente.

- 2026-10-24 00:48 (Europe/Madrid) · Antigravity · Aplicación del cambio estético con iconografía oficial del juego (`b9a7944`):
  - Biblioteca de componentes vectoriales `ui/src/lib/icons/` (`PowerIcon`, `SuperpowerIcon`, `ManufacturerIcon`, `StationIcon`, `GradeBadge`, `index.ts`).
  - `PowerplayPanel.svelte`: emblemas vectoriales oficiales para las 11 potencias (Aisling Duval, Hudson, Winters, etc.) con alternador a retrato fotográfico oficial de Frontier al hacer clic e insignia de superpotencia.
  - `BgsPanel.svelte`: insignias oficiales de superpotencia (Imperio, Federación, Alianza) en la tabla de facciones.
  - `AccountsPanel.svelte`: logotipos vectoriales de fabricantes navales (Gutamaya, Faulcon DeLacy, Core Dynamics, Lakon, Saud Kruger, Zorgon Peterson) e iconos de estación en la flota remota CAPI.
  - `TargetPanel.svelte`: logotipos de fabricante naval para el objetivo bloqueado en combate.
  - `EngineerFlow.svelte` y `EngineersPanel.svelte`: insignias vectoriales oficiales de grado de ingeniería G1..G5.
  - Verificación: 394/394 tests Vitest PASS, `svelte-check` 0 errores/0 avisos, `ui/dist` reconstruido y `target/debug/TMC.exe` recompilado.

- 2026-10-24 00:17 (Europe/Madrid) · Antigravity · F6 completada y verificada:
  - Tareas 1 a 11 ejecutadas en TDD con commits individuales (`14453df`, `25ea553`, `b377acd`, `50788b9`, `196e419`, `dbf685c`, `3d4ee11`, `87c26c2`, `7db2c37`, `919b4d9`, `ca14a61`).
  - Tarea 12 (Verificación integral): 394 tests Vitest PASS, svelte-check 0 errores/0 avisos, suite completa de Rust en el workspace PASS (1.400+ tests), clippy limpio con -D warnings, cargo fmt limpio, ui/dist/ generado y binario `target/debug/TMC.exe` compilado.
  - Cierre de Fase F6 y lista para fusionar en `main`.

- 2026-10-23 12:55 (Europe/Madrid) · Antigravity · Fusión formal de F5 en `main`, inicio de F6 (Cuentas y Sincronización Externa):
  - Fusión de `fase/f5-carrier-colonizacion-powerplay` en `main` con `--no-ff`.
  - Creada rama de trabajo `fase/f6-cuentas-sincronizacion`.
  - Redactada la especificación de diseño: `docs/specs/2026-10-23-tmc-f6-cuentas-sincronizacion-design.md` cubriendo emisor de esquemas EDDN (`commodity/3`, `journal/1`, `outfitting/2`, `shipyard/2`), EDSM (`set-logs`, rangos, créditos), Inara (InAPI v1 con lotes y rate limit de 2 req/min) y Frontier CAPI (OAuth2 PKCE con callback loopback local).
  - Creada maqueta interactiva HTML (`f6_accounts_sync_mockups.html`) con el sistema de diseño TMC Glass HUD, medidores de cuota en vivo, terminal de envíos, flota remota y simulador interactivo de estados.
  - Redactado el plan de ejecución: `docs/plans/2026-10-23-tmc-f6-cuentas-sincronizacion.md` (12 tareas en 7 bloques).
  - Siguiente: Aprobación del propietario para iniciar la ejecución TDD tarea a tarea.

- 2026-10-23 11:55 (Europe/Madrid) · Antigravity · Ejecución completa y de corrido de la Fase F5 (Fleet Carrier, Colonización, Powerplay 2.0 y BGS):
  - Tarea 1 (`3052983`): Tipado de eventos de Fleet Carrier (`CarrierStats`, `CarrierJumpRequest`, `CarrierFinance`, saltos y combustible) en `tmc-model`.
  - Tarea 2 (`ce5da4a`): Tipado de eventos de Colonización y Powerplay 2.0 (`ColonisationConstructionDepot`, `Powerplay`, `PowerplayMerits`, viajes) en `tmc-model`.
  - Tarea 3 (`9c0973f`): Cliente HTTP para el endpoint de tick de BGS (`https://tick.edcd.io/api/ticks`, caché de 30m, degradación offline) en `tmc-net`.
  - Tarea 4 (`d22a65e`): Módulo `carrier` en `tmc-modules` con telemetría de combustible, saltos de 500 al, cuenta atrás de 15m y salud financiera.
  - Tarea 5 (`4c2a185`): Módulo `colonisation` en `tmc-modules` con progreso de obras, viajes restantes y sourcing de suministros.
  - Tarea 6 (`6a2aaae`): Módulo `powerplay` en `tmc-modules` con fórmula oficial de Ascendancy, méritos/h y cuenta atrás del tick semanal.
  - Tarea 7 (`db45982`): Módulo `bgs` en `tmc-modules` con facciones locales ordenadas por influencia, estados y tick galáctico.
  - Tarea 8 (`2b4c687`): Localización completa en los 5 idiomas (`carrier.ftl`, `colonisation.ftl`, `powerplay.ftl`, `bgs.ftl` en es, en, fr, de, pt). 18/18 tests i18n PASS.
  - Tarea 9 (`772b4b0`): Paneles Glass HUD de Fleet Carrier (`CarrierOverviewPanel`, `CarrierJumpPanel`, `CarrierFinancesPanel`).
  - Tarea 10 (`b252aea`): Paneles Glass HUD de Colonización (`ColonisationProgressPanel`, `ColonisationSupplyPanel`).
  - Tarea 11 (`7d578fa`): Paneles Glass HUD de Powerplay 2.0 y BGS (`PowerplayPanel`, `BgsPanel`).
  - Tarea 12 (`72ba329`): Integración en Shell, layouts (`carrier`, `colonisation`, `powerplay`), navegación y prioridades del Auto-companion.
  - Tarea 13 (`3a360ae`): Verificación integral completa: 392 tests Vitest PASS (21 suites), `svelte-check` 0 errores/0 avisos, 1.380+ tests de Rust en el workspace PASS, `cargo clippy --workspace --all-targets -- -D warnings` limpio, bundle de producción UI generado en `dist/` y binario `target/debug/TMC.exe` compilado.
  - Siguiente: Revisión y aprobación del propietario para fusionar en `main` y avanzar a Fase F6 (Cuentas y Sincronización Externa: EDDN, EDSM, Inara, CAPI).

- 2026-10-23 10:25 (Europe/Madrid) · Antigravity · Ingesta y aplicación de la iconografía oficial de Elite Dangerous en TMC y en la maqueta interactiva F5:
  - Búsqueda y catalogación del repositorio estándar comunitario [`Venefilyn/EDAssets`](https://github.com/Venefilyn/EDAssets) (licencia MIT, compatible con TMC y la política de juego no comercial de Frontier Developments).
  - Descarga y normalización vectorial (`fill="currentColor"`, transparencias puras sin fondos negros fijos) en `ui/src/assets/icons/`: 11 potencias de Powerplay, 3 superpotencias, 6 fabricantes de naves (Gutamaya, Faulcon DeLacy, Core Dynamics, Lakon Spaceways, Zorgon Peterson, Saud Kruger), estaciones (Mega-Ship, Coriolis, Orbis, Ocellus, Outpost), grados de ingeniería 1..5 y navegación.
  - Maqueta F5 actualizada: sustitución del emoji genérico de corona por el emblema vectorial de Aisling Duval (con alternador a retrato fotográfico oficial de Frontier al hacer clic), águila imperial junto a la potencia jurada, insignia federal en Sistema Sol, logotipo de Gutamaya en Imperial Cutter, estaciones en las pestañas y modal de biblioteca de iconos oficiales en la cabecera.
  - `THIRD_PARTY_NOTICES.md` actualizado con EDAssets. 378/378 vitest PASS y `svelte-check` 0 errores.
  - Siguiente: Aprobación del propietario para iniciar la ejecución de F5.

- 2026-10-23 09:55 (Europe/Madrid) · Antigravity · Fusión formal de F4 en `main`, inicio de F5 y redacción de su especificación:
  - `fase/f4-ingenieros-materiales` fusionada en `main` con `--no-ff` tras completarse íntegramente las 28 tareas de F4, el catálogo extendido C0-C10, y el rediseño global «TMC Glass HUD» con 100% de tests en verde.
  - Creada la rama de trabajo `fase/f5-carrier-colonizacion-powerplay`.
  - Redactada la especificación de diseño: `docs/specs/2026-10-23-tmc-f5-carrier-colonizacion-powerplay-design.md`, abarcando Fleet Carrier (saltos, tritio, finanzas y mantenimiento semanal), Colonización (progreso de obra, compras y viajes restantes), Powerplay 2.0 (méritos de sesión, rango y ventajas), BGS (facciones locales y tick diario con `tick.edcd.io`) y soporte a pie (Odyssey).
  - Siguiente: Aprobación del propietario para desglosar el plan de ejecución TDD de F5.

- 2026-10-23 01:30 (Europe/Madrid) · Antigravity · Ejecución completa y de corrido del Plan de Rediseño Visual Global de TMC («TMC Glass HUD»):
  - Sistema de diseño normativo redactado y commiteado en `docs/design-system-glass-hud.md` (`ea5f3b7`), vinculado normativamente en `AGENTS.md` §0.
  - Tarea 1 (`92d4dcd`): Tokens CSS y clases comunes Glass HUD (`tokens.css`, `base.css`).
  - Tarea 2 (`18fbbf9`): Rediseño de `TopBar.svelte`, `Sidebar.svelte` y `Panel.svelte`.
  - Tarea 3 (`209f672`): Rediseño de `RouteStrip.svelte` y `SystemCard.svelte`.
  - Tarea 4 (`1b9f90d`): Rediseño de `SystemBodiesPanel.svelte` con VirtualList y badges económicos.
  - Tarea 5 (`47d70e9`): Rediseño de `ExobiologyPanel.svelte` con stepper horizontal 1..3 y medidor genético.
  - Tarea 6 (`ea84bca`): Rediseño de `TargetPanel.svelte`, `ShipPanel.svelte` y `AxPanel.svelte`.
  - Tarea 7 (`ebc3b20`): Rediseño de `ProspectorPanel.svelte`, `MiningSessionPanel.svelte` y `SellPanel.svelte`.
  - Tarea 8 (`427cb9b`): Rediseño de `MarketPanel.svelte`, `CargoPanel.svelte` y `RoutePanel.svelte`.
  - Tarea 9 (`285c10b`): Rediseño de `MissionsPanel.svelte`, `MassacrePanel.svelte`, `PassengersPanel.svelte`.
  - Tarea 10: Verificación integral completada (378/378 Vitest PASS, svelte-check 0 errores, tests completos de Rust en el workspace PASS, build de producción UI en `dist/`, y compilación del binario Tauri debug `target/debug/TMC.exe` con assets embebidos).

- 2026-10-22 22:45 (Europe/Madrid) · Antigravity · Incorporación de la filosofía de AYUDANTE en `AGENTS.md` §0 y Guía de Desbloqueo en el flujo de ingeniería:
  - `AGENTS.md`: Añadida la sección §0 definiendo que TMC existe para ser el AYUDANTE (COMPANION) integral en todo lo que se pueda hacer en Elite Dangerous, guiando a principiantes y expertos sin dejarlos jamás en un callejón sin salida.
  - `EngineerFlow.svelte`: Si un módulo no tiene acceso al ingeniero (`!nearest_unlocked`), se despliega de forma prominente la «Guía de Desbloqueo del Ayudante» con la secuencia de pasos (descubrimiento, invitación, donación requerida y botón para copiar sistema del taller al portapapeles) y *después* se presentan los materiales y recetas necesarias.
  - i18n: 8 claves añadidas en los 5 idiomas (`locales/{es,en,fr,de,pt}/engineering.ftl`).
  - Verificación: 18/18 tests de i18n PASS, 378/378 tests Vitest PASS, `svelte-check` 0 errores/avisos, clippy limpio, UI compilada y binario `target/debug/TMC.exe` ejecutándose en segundo plano (PID 12236).


- 2026-10-22 22:36 (Europe/Madrid) · Antigravity · Planificación del rediseño visual global de toda la aplicación (Design System «TMC Glass HUD»):
  - Maqueta HTML interactiva navegable generada como artefacto (`ui_redesign_global_mockups.html`) con los 6 módulos: Shell (TopBar & Sidebar), Exploración & Exobiología, Combate & AX, Minería, Comercio, y Misiones & Pasajeros.
  - Especificación de diseño formal redactada y commiteada: `docs/specs/2026-10-22-tmc-ui-rediseno-global-design.md` (`e6471de`).
  - Plan de ejecución redactado y commiteado: `docs/plans/2026-10-22-tmc-ui-rediseno-global.md` (`e6471de`), estructurado en 10 tareas en 5 bloques.
  - Siguiente: Revisión y aprobación por parte del propietario.

- 2026-10-22 22:25 (Europe/Madrid) · Antigravity · Ejecución del plan de rediseño visual de Ingeniería y Materiales (Opción 2) y resolución del bug crítico de módulos vírgenes:
  - Tarea 1 (`1a81c64`): 17 claves i18n añadidas en los 5 idiomas (`locales/{es,en,fr,de,pt}/engineering.ftl`).
  - Tarea 2 (`d404bca`): Rediseño de `MaterialsPanel.svelte` con buscador de materiales, barra de capacidad total por categoría y barras de progreso visuales con badge `MÁXIMO`.
  - Tarea 3 (`acf0683`): Rediseño de `EngineersPanel.svelte` con filtros Todos/Desbloqueados/Pendientes, estrellas de rango de acceso (`G5 ★★★★★`) y badges de estado semánticos.
  - Tarea 4 (`31bcc81`): Rediseño de `EngineerFlow.svelte` con stepper horizontal de grados G1..G5, semáforo de existencias, tarjeta de asistente de intercambio directo y tarjeta de ingeniero recomendado; rediseño de `ShoppingPanel.svelte`.
  - Bugfix (`44fc0da`): Corregida la detección de módulos vírgenes / stock en nave (`ship.rs`) añadiendo los 44 prefijos de módulo a `scripts/import-engineering-catalogue.mjs`, regenerando `blueprints.json` y añadiendo fallback de slot core para módulos sin ingeniería previa. Test `virgin_stock_modules_are_engineerable` añadido y verificado.
  - Tarea 5: 378/378 tests Vitest PASS, `svelte-check` 0 errores, compilación de UI `ui/dist`, compilación de debug `cargo build -p tmc-desktop` (`target/debug/TMC.exe`).
  - Siguiente: Lanzar `TMC.exe` y probar en vivo con el usuario en el juego.

- 2026-10-22 17:40 (Europe/Madrid) · Antigravity · F4 completada y cerrada (Task 28 / C10): 10/10 criterios de aceptación de spec F4 verificados y documentados en el plan. Clippy release limpio (target-f4); 1.370+ tests Rust workspace PASS; 375 tests Vitest PASS; 7/7 tests Playwright e2e PASS; footprint measurement con ventana de 120s: 122,20 MB RAM / 0,00% CPU idle PASS (límites < 150 MB / < 1%). THIRD_PARTY_NOTICES.md completado con Spansh y Ardent (red); sin dependencias nuevas (0 diff con main). Rama fase/f4-ingenieros-materiales lista para merge en main tras autorización del propietario.

- 2026-10-22 17:05 (Europe/Madrid) · Antigravity · C9 cerrada en `2b6dba9`: revalidada integración y pruebas e2e con el catálogo final y contratos paginados. Corregido fallo de `renameSync` inter-dispositivo en `e2e-server.test.mjs`. 7/7 Playwright e2e PASS, e2e-server harness PASS, 375 tests Vitest PASS, 1.370+ Rust workspace PASS, fmt y clippy limpios. Inicia C10 / Task 28.

- 2026-10-22 17:00 (Europe/Madrid) · Antigravity · C8 cerrada en `e3ad29f`: activado catálogo completo de ingeniería con procedencia de EDDiscovery 7d30920 y mapa auxiliar EDDI d3b964e (89 blueprints, 87 experimentales, 15 variantes, 38 ingenieros, 141 materiales). 8 filas sin resolver remapeadas en overrides.json con evidencia citada (0 unresolved). i18n completa en 5 idiomas (engineering-data.ftl). Actualizadas expectativas de tests en tmc-app y UI con los oráculos de la fuente real. Checks: 15/15 node PASS, 1.370+ Rust tests PASS, 375 UI tests PASS, svelte-check 0 errores, fmt y clippy workspace limpios. Inicia C9.

- 2026-10-22 · Codex · Sesión en curso: Task25 4c70ecb y Task26 8a70048 cerradas y revisadas; fuente EDDiscovery, enmienda técnica y mapa EDDI aprobados. Task27 TDD de eval en curso; C0 documental en ratificación. Evidencias Task26: 1.362 Rust/343 UI/7 Playwright/1 Node PASS. Sin importación nueva ni push.

- 2026-10-22 01:37 (Europe/Madrid) · Codex · Task 24 cerrada `65784b6`: ambas revisiones, 309 UI/1.356 Rust PASS, fmt/clippy/check/build y QA visual PASS. Continúa Task 25; visibilidad del flujo adaptada a slot seleccionado para permitir primer preview desde Ingeniería. Pregunta de fuente B pendiente, sin bloquear UI.

- 2026-10-22 01:26 (Europe/Madrid) · Codex · Task 23 cerrada `d644a6d`: spec y calidad aprobadas tras corregir experimental null, reselección y esperas entre clientes/contextos; 293 UI y 1.356 Rust PASS (2 ignoradas previas), fmt/clippy/check/build y QA visual PASS. Continúa Task 24. Catálogos completos de blueprints/ingenieros siguen pendientes antes del cierre F4.

- 2026-10-22 01:12 (Europe/Madrid) · Codex · Task 22 cerrada `88836b7`: revisión spec aprobada, P2 de mercancías corregido y calidad aprobada; 274 UI/1.356 Rust PASS, fmt/clippy/check/build y QA visual PASS. Latencia combate falló una vez 578,9 ms; aislada PASS 307,3 ms y suite completa repetida PASS sin cambiar el umbral. Corregidas notas antiguas de importación de materiales/nombres con commits existentes. Continúa Task 23.

- 2026-10-22 00:59 (Europe/Madrid) · Codex · Task 21 cerrada en `04e5ea4` tras revisiones spec/calidad, QA visual 2560/800 y checks globales: 1.356 Rust PASS/2 ignoradas previas y 258 UI PASS. Continúa Task 22 con TDD. Sin código pendiente de Task 21.

- 2026-10-22 00:59 (Europe/Madrid) · Codex · Propietario reanuda. Task 20 sigue cerrada; se retoma Task 21 implementada sin commit, con 20 tests nuevos informados y Unicode corregido. QA visual anterior no ejecutado por fallo de revisión automática debido a cuota; reintento ahora autorizado por el sistema pero servidor local ya no estaba activo. Se reinicia servidor de desarrollo y se continúa QA/revisiones/checks.

- 2026-10-21 19:59 (Europe/Madrid) · Codex · Task 20 cerrada `a4c5f3d` con aprobación previa de paginación, TDD, revisión spec/calidad y checks globales: 1.356 Rust PASS/2 ignoradas previas, 238 UI PASS, fmt/clippy/check/build. Ambos probes reactivados; HTTP/actor real reconstruye20objetivos/215propuestas y200recursos sin pérdida, páginas<16KiB, revisión semántica y aislamiento de clientes. Sin push, release original preservado. Se inicia Task 21 Step 1; ningún código UI tocado todavía. Este seguimiento se guarda en commit documental inmediato.

- 2026-10-21 19:45 (Europe/Madrid) · Codex · Propietario aprueba resumen WS + detalle REST por bytes y continuar. Actualizadas spec/plan e interfaces afectadas; Task 20 en curso con implementador y revisión independiente prevista. Cambios documentales y backend en curso sin commit; no se inicia Task 21 hasta cerrar la verificación de Task 20.

- 2026-10-21 19:34 (Europe/Madrid) · Codex · Propietario ordena verificar COMPLETAMENTE Task 20. Revisiones de spec y calidad detectan y corrigen carreras de objetivos, compatibilidad de blueprint/ranura, respuestas de comerciantes por plataforma e ID y diagnóstico con valores finitos. Commit parcial `1fc8b74`; fmt/clippy PASS, workspace serial fuera sandbox (TMP/TEMP `.tmp/test-runtime`) 1.344 PASS/4 ignoradas, UI 238 PASS/check/build. Dos probes ejecutados --ignored demuestran objetivos 41.116 B/215 propuestas y taquilla 16.832 B/200 recursos; quedan ignorados con motivo explícito, no como PASS. Task 20 bloqueada y Task 21 sin iniciar; propuesta y comandos en plan/Desviaciones. Sin push. Código y plan guardados; este seguimiento se guarda en commit documental inmediato. Se conservan `.claude/`, `.tmp/` y `target-f4/` no rastreados; ninguna modificación de código pendiente.

- 2026-10-21 (Europe/Madrid) · Codex · Revisión de estado solicitada: detectada discrepancia entre Task 20 sin commit en seguimiento y HEAD `9904133` con su implementación. Ejecución detenida según AGENTS.md §1; bloqueo y pregunta concreta registrados. Único archivo rastreado modificado: `status.md` (sin commit); no se han ejecutado checks ni alterado código. Pendiente autorización para reconciliar, verificar Task 20 y continuar Task 21.

- 2026-10-21 (Europe/Madrid) · Claude Opus 5.5 (architect-opus) · Plan de F4 escrito (`docs/plans/2026-10-21-tmc-f4-ingenieros-materiales.md`, 29 tareas en 6 bloques) en `fase/f4-ingenieros-materiales`; siguiente: bloque 0 (spikes).
- 2026-10-21 13:30 (Europe/Madrid) · Claude Opus 5.5 · Interruptor Auto-companion (c986387, 09e5dab) revisado y fusionado en `main`. Propuesta y maquetas de F4 aprobadas en principio por el propietario; siguiente: spec de F4.
- 2026-10-20 21:30 (Europe/Madrid) · Claude Opus 5.5 · Criterio 10 medido con ventana (106,4 / 111,5 MB, 0 % CPU) tras cerrar el propietario su TMC.exe. `fase/f3-comercio-misiones` fusionada en `main` (`6dc09d9`, --no-ff) con autorización del propietario. Sobre `main`: fmt/clippy limpios, 1158 Rust PASS, svelte-check 0 errores, 228 vitest PASS, build OK. `main` subida.
- 2026-10-20 19:05 (Europe/Madrid) · Claude Opus 5.5 · F3 ejecutada hasta la Task 34 con subagentes (Tasks 17–34 tras integrar el diagnóstico debug D1–D7b y logs en SQLite). Revisión final de rama (Opus): sin críticos; corregido comercio en Legacy y coordenadas propias en replay (`7a99bfd`). 1154+ Rust, 228 UI, 14 Playwright, clippy release PASS. Pendiente: criterio 10 con ventana (propietario) y fusión en `main`.
- 2026-10-20 16:14 (Europe/Madrid) · Codex · Limpieza solicitada por el propietario: eliminados 103,771 GiB de artefactos regenerables de Cargo entre el checkout principal y prueba-main (incremental, deps, build y fingerprints). Sin compilaciones activas; 22 ejecutables, instaladores y símbolos conservados con SHA256 idéntico. Logs, datos, código y cambios pendientes de missions intactos. Inventario local: `.tmp/cleanup-build-artifacts.json`. Puntero de desarrollo sin cambios; la próxima compilación regenerará dependencias.

- 2026-10-20 · Claude Opus 5.5 · D7b: `codex/diagnostico-logs` fusionada en F3 (6e5e97a, 6 conflictos resueltos) y cableado de EDDN/comercio/endpoints F3 (a1c2bb3). fmt/clippy/clippy release PASS, 999 Rust PASS ×2, 160 UI PASS. WIP de Task 17 preservado en stash y restaurado sin commit. Siguiente: Task 17.
- 2026-10-20 · Claude Opus 5.5 · D7a cerrada: captura de tests estable (713a8f4), checks y replay sintético OK, hash release intacto. Siguiente: D7b.
- 2026-10-20 · Claude Sonnet 5.5 · D6 5a81eff cerrada: UI con capacidad `diagnostics` solo en backend debug, acción `app.diagnostics`; 834 Rust PASS/1 ignorado, 155 UI PASS, fmt/clippy/clippy release PASS. Siguiente: D7.
- 2026-10-20 · Claude Sonnet 5.5 · D5 b6e139b cerrada: red (tmc-net/net_service), HTTP/WS, auth, acciones y portapapeles; 824 Rust PASS/1 ignorado, fmt/clippy/clippy release PASS. Siguiente: D6.
- 2026-10-20 · Claude Sonnet 5.5 · D2b 58696b7: escritor SQLite separado `debug_logs/diagnostics.sqlite` por apartados (spec/plan/desviación en b325a2e); fmt/clippy/clippy release PASS. Siguiente: D5.
- 2026-10-20 · Claude Sonnet 5.5 · D4 42c1ccc cerrada: actor (evento, transición de estado, topics redactados, efectos) y decisiones de los seis módulos; 811 Rust PASS/1 ignorado, fmt/clippy/clippy release PASS. Siguiente: D5.
- 2026-10-20 · Claude Sonnet 5.5 · D3 44e8a9e cerrada: fuentes locales y persistencia instrumentadas; 804 Rust PASS/1 ignorado, fmt/clippy/clippy release PASS. Siguiente: D4.
- 2026-10-20 00:37 (Europe/Madrid) · Codex · D2 ed0611e cerrada con writer DEBUG acotado y 14 pruebas. Correcciones de escape JSON, limpieza bajo bloqueo Windows y orden de cierre revisadas. 795 Rust PASS/1 ignorado, fmt/clippy y clippy release PASS; empieza D3.

- 2026-10-20 00:21 (Europe/Madrid) · Codex · D1 e6c5d09 revisada y verificada: 781 Rust/144 UI PASS, release sin evaluación, fmt/clippy/check/build correctos. Inicia D2. Se reconcilia solo el resumen de fases con merges reales; tablas históricas completas intactas.

- 2026-10-20 · Codex · D1 diagnóstico debug iniciado en checkout aislado; spec y plan autorizados por instrucción expresa del propietario. Sin tocar tablas históricas F0/F1/F2. Contrato/redacción y docs sin commit; GREEN 91 tmc-model + release noeval. Revisión y checks globales pendientes. Dos expectativas UI de rango Maestro→Master sincronizadas con decisión del propietario de 2026-10-19 (baseline RED 142 PASS/2 FAIL).

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
