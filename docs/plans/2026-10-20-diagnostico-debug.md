# TMC — Plan de diagnóstico debug

Spec: [diseño autorizado](../specs/2026-10-20-diagnostico-debug-design.md). Ejecución secuencial con subagent-driven-development, un implementador y dos revisiones por tarea (spec y calidad). TDD: RED observado → mínimo GREEN → revisión → fmt/clippy/tests workspace y checks/UI antes de commit. Un commit por tarea, status actualizado. Sin dependencias nuevas.

## D1 — Contrato y redacción (verificada)
Archivos: tmc-model/src/{diagnostics.rs,lib.rs}, tests de contrato.
Interfaces: macro diagnostic!(context, "constant.code", payload); DiagnosticContext<'a> y redact_diagnostic según spec; emit<T: Serialize>(context, &'static str, &T).
Tests: canarios anidados/Unicode, Name de ScanBody público, fallback seguro, fallo de serialización sin texto del error, release no evalúa argumentos (target alterno). Casos reales GameState y NetQuery/NetOutcome; cmdr conserva credits/ranks/progress, location.body público. Rutas exactas navigation.route y listas finitas schema/topic/module/action/event; OTP/pin/authcode y ship_id/ship.ident numéricos privados.
Pasos: [x] RED observado (5 iniciales, 3 primera revisión, 3 segunda revisión y 2 de calidad); [x] GREEN (13 diagnósticos, 96 tmc-model); [x] revisión spec; [x] revisión calidad; [x] checks. Commit registrado inmediatamente en status.md. Evidencia final: cargo fmt --all --check y cargo clippy --workspace --all-targets --offline -- -D warnings PASS; cargo test --workspace --offline --quiet: 781 PASS, 1 ignorado; release_diagnostic_does_not_evaluate_or_typecheck_arguments PASS con target alterno; UI check 0 errores/0 avisos, 144 tests PASS y build PASS. La captura efectiva de emit se verifica en D2 junto al writer.

## D2 — Writer circular y ciclo de vida
Archivos: logging tmc-app y arranque desktop/headless. LoggingGuards/init_logging conserva INFO y se exporta desde lib; añadir patrón sin anclar debug_logs/ en .gitignore. Writer independiente: cola2048×2KiB, chunk máximo1MiB/evento, 32×8MiB, rotación diaria/7d, captura reciente continua, marcas drop/incomplete/disk/exit, sesión/secuencia/monotónico. Solo executable/debug_logs; limpieza propia. Tests límites, rotación, disco fallido, cierre/flush y release sin writer.

## D3 — Fuentes y persistencia
Instrumentar diario/Status/config/importador/checkpoints/SQLite. Offsets reales bytes, SQL plantilla+tipos/resultados, ningún parámetro privado. Filas SQL sin nombres: tipos/count; con nombres: mapear y redactar por contrato. Jamás Display/Debug de Unsupported con valores. Tests lectura fragmentada/corrupta y fallos clasificados.

## D4 — Actor y módulos
Evento, estado previo/posterior, duración, relevancia, efectos, topics completos; códigos de decisiones internas en todos los módulos. Tests replay sin E/S y decisiones/topics reproducibles.

## D5 — Red, HTTP, WS, autenticación y acciones
Caché/reintentos/IDs y consultas/respuestas redactadas; plantillas de ruta sin URL/query; seguridad/emparejamiento sin credenciales; portapapeles. Tests canarios y fallos sin raw Debug.

## D6 — UI
Hello publica capacidad solo debug; backend app.diagnostics solo cfg debug con validación y tasa estrictas. Sin helper/listeners/peticiones sin capacidad; cola acotada, temporizador solo pendiente ≤1 lote/s, sin recursión. Panel/VirtualList/shell cubren todas las vistas, aplicación de topic y errores. Snapshots sin seq no producen huecos. App.svelte registra activeTab/visibleSlots/relevance/contextPanel/AX/CZ. Hooks específicos SystemCard/RouteStrip y cobertura TopBar/Toasts/ImportProgress/Pairing. Fetch directo Settings/Pairing instrumentado sin pairingData.url/qr_svg/fullConfig.server.lan_token. Tests debug-capability on/off, backlog y errores.

## D7 — Integración y reanudación de F3
fmt --all --check, clippy workspace all-targets -D warnings, tests workspace, UI check/test/build. Replay sintético aislando puertos/BD y footprint. Build debug con CARGO_TARGET_DIR=D:/Proyectos/EDPyCompanion/target, preservar hash target/release/TMC.exe. Documentar evidencias, revisar rama, integrar autorización vigente y retomar exactamente F3 en checkout principal sin mezclar cambios parciales. Añadir diagnóstico propio a EddnService al integrar; hooks genéricos cubrirán trade/missions.

## Desviaciones
- D1: baseline UI tenía dos expectativas obsoletas de rango Maestro; el propietario ya aprobó rangos en inglés el 2026-10-19. Se sincronizan solo esas dos expectativas con Master, como en F3, sin importar el resto de F3. RED observado por el controlador (142 PASS/2 FAIL).
- Validación completa y commits se coordinan con el controlador; ninguna tarea se marca hecha antes de revisión y checks.
