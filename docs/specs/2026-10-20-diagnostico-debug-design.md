# TMC — Diagnóstico VERBOSE en compilaciones de depuración

- Estado: extensión autorizada expresamente por el propietario el 2026-10-20.
- Base: arquitectura F0 §9; conserva los logs INFO existentes.
- Alcance: toda la aplicación existente. Tras integrar y verificar, retomar F3.

## Contrato y privacidad

Solo `debug_assertions` activa diagnósticos. `tmc_model::diagnostic!(context, "constant.code", payload)` existe en ambos perfiles; en release su expansión es vacía y no evalúa argumentos. El módulo `diagnostics` solo existe en debug. `DiagnosticContext<'a>`: Journal { event: &'a str }, State, Topic { name: &'a str }, NetQuery, NetOutcome, Action { name: &'a str }, Http, Auth, Ui, Sql. `redact_diagnostic(&serde_json::Value, DiagnosticContext<'_>) -> Value` es la frontera central.

Antes de emitir TRACE a `tmc::diagnostics`, serializar sin pánico y redactar por contexto y ruta. Nunca registrar texto de errores sin clasificar. Preservar números, booleanos, estructura y longitud de arrays; conservar textos de campos públicos reconocidos (sistemas, cuerpos, estaciones y datos del juego). `Name` de Scan/ScanBody es un cuerpo público; no tratar cualquier `name` como público. Comandante/FID, nombre personalizado e identificador de nave, chats, tokens, claves, contraseñas, cookies, autorización, rutas personales y URLs completas se redactan incluso anidados. Un contenedor privado (chat, auth, credenciales, cabeceras) domina cualquier campo público anidado; cmdr/commander son objetos de estado: conservar créditos, rangos y progreso, redactando nombre/FID/id. Texto desconocido se sustituye por marcador; contexto desconocido no habilita texto libre. Los valores numéricos privados se sustituyen por cero conservando tipo. Las claves son nombres del esquema, nunca datos personales: los mapas indexados por usuario deben convertirse en arrays antes de emitir.

TRACE incluye context fijo y schema: Journal.event/Topic.name/Action.name solo si pertenecen a las listas explícitas del parser y registros actuales; desconocidos se redactan. Los campos payload event/topic/module/action usan las mismas listas finitas; reason/code libres no se consideran públicos. Los códigos de decisiones son el literal de la macro. En navigation.route se conservan Name de current/next/following y remote.data por ruta exacta, nunca Name global. core.location.body y State.location.body son públicos. ship_id y ship.ident se redactan; también OTP/pin/authcode numéricos, sin ocultar attempts o contadores.

El writer añade sesión, secuencia y tiempo monotónico; los emisores no los duplican. Un fallo de serialización produce únicamente un marcador fijo.

## Captura y límites

En debug el TRACE es incondicional e independiente de RUST_LOG. `init_logging` devuelve `LoggingGuards`; el INFO actual conserva destino, filtro y retención. El writer propio usa cola acotada de 2048 registros de 2 KiB, fragmentos con máximo de 1 MiB por evento, circular de 32 archivos × 8 MiB, rotación diaria y retención de 7 días. Se conserva captura reciente continuamente, sin corte por cuota diaria. Marcas de pérdida, fragmentos incompletos, fallo de disco y salida. Solo `current_exe()?.parent()/debug_logs`; si falla no hay destino alternativo. Limpieza exclusivamente de archivos propios; .gitignore incluye debug_logs/ sin anclar.

## Cobertura

- Diario: offsets reales en bytes, lecturas y parseo; configuración sin valores privados; persistencia: importación, checkpoints, SQL como plantilla y tipos/resultados, sin parámetros privados. Las filas SQL sin nombres de columna solo se describen por tipos/count; con nombres, mapear al contrato antes de redactar. StoreError::Unsupported nunca se formatea con el valor original.
- Actor: evento, estado previo/posterior, relevancia, duración, efectos y topics completos redactados. Todos los módulos incorporan decisiones internas identificadas por códigos.
- Red: consultas/respuestas, caché, reintentos y correlación; HTTP/WS con plantillas de ruta sin URL/query; autenticación/emparejamiento solo decisiones, nunca credenciales. Incluir portapapeles.
- UI: capacidad en hello solo backend debug (el bundle Vite de producción también puede servirse en debug). Acción app.diagnostics solo debug, validación estricta, cola acotada y límite de tasa. Sin helper, listeners ni peticiones sin capacidad. Temporizador solo con pendientes, máximo un lote/segundo y sin recursión. Panel y VirtualList compartidos cubren tarjetas, shell y todas las vistas; topics, aplicación y errores. Snapshots sin secuencia no generan falsos huecos. App.svelte registra activeTab/visibleSlots/relevance/contextPanel/AX/CZ para explicar tarjetas ocultas; SystemCard/RouteStrip requieren hooks propios. TopBar, Toasts, ImportProgress y Pairing también se cubren. Settings y Pairing tienen fetch directo y requieren instrumentación; nunca pairingData.url, qr_svg ni fullConfig.server.lan_token.

## Aceptación

Canarios sensibles anidados y Unicode no aparecen en JSON emitido; datos públicos de Scan sí. Release no evalúa argumentos. Writer acotado y fallos recuperables. Verificación Rust/UI completa y replay sintético con puertos/BD aislados; presupuesto de consumo existente. Compilar TMC.exe debug en target original sin modificar ni sobrescribir release; comparar hash release antes/después. Al retomar F3, los hooks genéricos cubren trade/missions; instrumentar también la frontera propia de EddnService. Sin telemetría, servicios ni dependencias nuevas.
