# TMC — The Multipurpose Companion

Companion en tiempo real de bajo consumo para **Elite Dangerous** (Windows 10/11, preparado para Linux).

> Estado: **F0 y F1 completadas**. Consulta [`status.md`](status.md), la [spec de F1](docs/specs/2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md) y el [plan y sus evidencias](docs/plans/2026-10-17-tmc-f1-navegacion-exploracion.md).

## Características de F0

- **Lectura en tiempo real:** seguimiento incremental por offset de bytes de los diarios `Journal.*.log`, `Status.json` (100–250 ms), archivos compañeros (`Cargo.json`, `NavRoute.json`, `Market.json`, `Outfitting.json`, `Shipyard.json`, `ModulesInfo.json`, `Backpack.json`, `ShipLocker.json`), detección del proceso del juego y matriz de color HUD (`GraphicsConfigurationOverride.xml`).
- **Almacenamiento e historial:** SQLite en modo WAL (`tmc.db`) con importación en segundo plano reanudable por archivo y offset, y reconstrucción automática si la base de datos se corrompe.
- **Servidor local y protocolo WebSocket:** actor central que publica topics con *coalescing* a máx. 10 Hz (`hello`, `snapshot`, `update`, `alert`, `import-progress`) y endpoints REST (`/api/health`, `/api/history/events`, `/api/config`, `/api/actions/{name}`, `/api/pairing`).
- **Interfaz adaptable (2560×1080 UW / 1080p / móvil):** panel general (`dashboard`) con Comandante, Nave, Ubicación, Status en vivo, Ruta (con copia al portapapeles del siguiente sistema), Sesión y Alertas; 5 idiomas (`es`, `en`, `fr`, `de`, `pt`) con Fluent y tema dinámico del HUD del jugador.
- **Escritorio e instalador:** aplicación Tauri 2 (`TMC.exe`) con icono en bandeja del sistema, cierre a bandeja destruyendo los procesos `msedgewebview2.exe`, ahorro de memoria al minimizar, instancia única e instalador NSIS (`TMC_0.1.0_x64-setup.exe`).

## Navegación, exploración y exobiología (F1)

- **Disposición Exploración por defecto:** tira de ruta a todo el ancho, contexto de cuerpos o exobiología, planificador y alcance/combustible. El contexto cambia con la relevancia del módulo; los botones manuales lo fijan hasta cambiar de cuerpo. Los cuatro módulos siguen accesibles en la barra lateral.
- **Navegación:** siguiente salto y tres posteriores, fichas locales/remotas, visitas del historial, alcance y combustible estimados, planificación de neutrones o galaxia, aceptación y seguimiento de la ruta y copia automática opcional. Los FSD sin datos suficientes muestran alcance desconocido o desactivan los modos que necesitan esos datos.
- **Exploración:** tabla virtualizada de cuerpos, progreso FSS, valores estimados de escaneo/mapeo, consejo de mapeo, sondas aproximadas y saldo de datos sin vender reconstruido desde el historial.
- **Exobiología:** candidatos por reglas propias e historial, origen de la predicción, muestreo, distancia y rumbo respecto a las muestras y valor sin vender. Nombres desde los campos localizados del diario, con el nombre latino como respaldo. Las especies sin reglas se muestran sin predicción.

Las estimaciones y tablas tienen cobertura y verificación parciales sobre el historial local actual; véase [Evaluación local](#evaluación-local-del-historial).

## Servicios online y privacidad

En **Ajustes**, «Consultar servicios online (Spansh, EDSM, Canonn)» controla `online_services` (activado por defecto). El cambio se guarda y se propaga en caliente. Al desactivarlo no se envían nuevas consultas; siguen disponibles los datos locales y el historial, y el planificador explica que necesita conexión. Un fallo al guardar muestra un aviso y restaura la última configuración confirmada.

El opt-out se comprueba de nuevo antes de cada envío, incluso tras esperar un cupo o durante el sondeo de una ruta. Una petición que ya se había enviado puede terminar; no se puede retirar del servicio remoto.

- **Spansh:** fichas de sistema y planificador, con un límite propio de una petición por segundo y un trabajo de planificación a la vez.
- **EDSM:** alternativa para información de sistemas; límite de diez peticiones cada diez segundos.
- **Canonn permanece desactivado en F1**, aunque aparezca en la etiqueta del ajuste: su adaptador devuelve `Disabled` y no consulta el servicio hasta que el propietario confirme el permiso formal requerido. La predicción sigue usando reglas propias e historial. Decisión y evidencia en el [spike de Canonn](docs/research/notes/spike_canonn_f1.md).

Las consultas usan nombres o identificadores de sistemas y los parámetros necesarios para la ruta o el cuerpo consultado; no se envían nombres de comandante ni diarios completos. Los servicios no reciben datos de la galaxia Legacy. Las respuestas permitidas se guardan en `api_cache`; no se incorporan al repositorio. Las fichas tienen una frescura de 24 h y los datos políticos de 1 h, con uso limitado de caché antigua y recuperación ante caídas.

Los ajustes incluyen el umbral de «Mapear» (500 000 cr por defecto) y la eficiencia predeterminada del planificador (60 %, rango 0–100). La copia automática del siguiente sistema se controla por separado.

## Compilación

### Requisitos

- **Rust** estable (edición 2024, `1.90+`) con MSVC Build Tools en Windows.
- **Node.js** 22+ y `npm`.
- **Tauri CLI** 2.x (`cargo install tauri-cli@^2`).

### Pasos

```powershell
# 1. Instalar dependencias y compilar la interfaz web (ui/dist)
npm.cmd --prefix ui ci
npm.cmd --prefix ui run build

# 2. Compilar todo el workspace Rust
cargo build --workspace --release

# 3. Generar el instalador NSIS (desde apps/desktop)
Push-Location apps/desktop
cargo tauri build
Pop-Location
# Salida: target/release/bundle/nsis/TMC_0.1.0_x64-setup.exe
```

## Ejecución

### Aplicación de escritorio (`TMC.exe`)

```powershell
.\target\release\TMC.exe
# Opcional: indicar carpeta de diarios o archivo config.toml explícitos
.\target\release\TMC.exe --journal-dir "D:\Ruta\Saved Games\Frontier Developments\Elite Dangerous"
```

- Al cerrar la ventana principal, TMC permanece activo en la bandeja del sistema y destruye el WebView para liberar memoria.
- Clic izquierdo o «Abrir TMC» en el icono de bandeja vuelve a crear la ventana; «Salir» detiene el servidor y cierra la aplicación.

### Modo sin ventana (`tmc-headless`) y desarrollo de UI

```powershell
# Terminal 1: núcleo + servidor HTTP/WS en 127.0.0.1:47320
cargo run -p tmc-headless -- --journal-dir tests/fixtures/journal

# Terminal 2: servidor de desarrollo Vite con proxy a 127.0.0.1:47320
npm.cmd --prefix ui run dev
```

## Reproducción y anonimización de diarios (`tmc-replay`)

`tmc-replay` permite probar TMC en vivo sin abrir el juego y anonimizar diarios reales antes de usarlos como fixtures:

```powershell
# Reproducir un diario de prueba a 10x sobre una carpeta temporal
cargo run -p tmc-replay -- play `
  --from tests/fixtures/journal/basic_session.log `
  --to "$env:TEMP\tmc-replay" `
  --speed 10

# Anonimizar diarios reales (sustituye comandante, FID, naves/flota y elimina chats/amigos)
cargo run -p tmc-replay -- anonymize `
  --from "C:\Users\<usuario>\Saved Games\Frontier Developments\Elite Dangerous" `
  --to "$env:TEMP\tmc-anon"
```

## Modo LAN (tablet / móvil en red local)

Por defecto, TMC escucha exclusivamente en `127.0.0.1:47320`. Para consultar el panel desde otro dispositivo de tu red local:

1. Abre **Ajustes** en la barra lateral y activa **Acceso por red local**; el guardado es automático.
2. Reinicia TMC para que el servidor escuche en la interfaz de red local (`0.0.0.0`).
3. Desde el propio PC en `127.0.0.1`, abre **Ajustes** y escanea el código **QR de emparejamiento** con tu tablet o móvil.
4. La URL del QR incluye el token aleatorio de 128 bits (`?token=...`), que establece una cookie `HttpOnly; SameSite=Strict` para autenticar las conexiones HTTP y WebSocket. Sin token válido, cualquier petición desde fuera de loopback recibe `401 Unauthorized`.

> Nota: en F0 la conexión LAN utiliza HTTP sin TLS (no es instalable como PWA independiente).

## Evaluación local del historial

`tmc-headless eval` lee una carpeta de diarios `Journal.*.log` y genera métricas agregadas de alcance, valores de exploración y predicción de especies. No inicia consultas online ni imprime nombres de comandante o de sistemas; el informe incluye números y símbolos del juego. `--timings` mide además las consultas de visitas, saldo sin vender y estadísticas biológicas sobre SQLite temporal en memoria.

```powershell
cargo run -p tmc-headless -- eval `
  --journal-dir "D:\Ruta\Diarios" `
  --timings
```

La salida distingue `PASS`, `FAIL` e `INSUFICIENTE`; un código de proceso 0 confirma la ejecución del informe, no la aceptación de los criterios.

Última evidencia registrada (2026-10-18; 7 diarios, 3641 eventos):

| Criterio F1 | Evidencia agregada | Estado |
|---|---|---|
| 3: alcance, tolerancia ±0,05 al | 15/15 casos coinciden; error máximo 0,0023 al; faltan ingeniería y Guardian. Los cuatro FSD observados se ajustaron con ese mismo historial: demuestra consistencia, no validación independiente. | INSUFICIENTE |
| 4: exploración, tolerancia ±5 % | Ninguna venta con cobertura completa de escaneos; cuatro ventas sin cobertura. Un error mostrado como 0 con cero casos no valida la fórmula. | INSUFICIENTE |
| 6: predicción, acierto Alta/Media ≥70 % | Ningún caso cuerpo/especie evaluable en el historial disponible. El muestreo se ha probado con un fixture sintético; falta una sesión real. | INSUFICIENTE |

Consultas registradas sobre 3630 eventos importados: visitas 1,3 ms, sin vender 11,4 ms y estadísticas biológicas 1,2 ms. Los criterios completos, sus desviaciones y las verificaciones figuran en el [plan de F1](docs/plans/2026-10-17-tmc-f1-navegacion-exploracion.md#verificación-f1).

## Pruebas y presupuesto de consumo

```powershell
# Formato, linter y tests unitarios/integración en Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Verificación de tipos, tests unitarios (Vitest) y test e2e (Playwright) en UI
npm.cmd --prefix ui run check
npm.cmd --prefix ui test
Push-Location ui
$env:TMC_E2E_PORT = '47322' # Puerto libre para no interferir con TMC abierto
node node_modules/@playwright/test/cli.js test
Pop-Location

# Medición del presupuesto de consumo (< 150 MB Private Bytes, < 1 % CPU en reposo)
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-footprint.ps1 `
  -Exe target/release/TMC.exe `
  -JournalDir .tmp\footprint-f1 `
  -ShowWindow `
  -Seconds 120
```

## Licencia y marcas

Licenciado bajo **PolyForm Noncommercial 1.0.0** (ver [`LICENSE`](LICENSE)). Avisos de dependencias de terceros en [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

*Elite Dangerous* es una marca registrada de Frontier Developments plc. TMC no está afiliado ni respaldado por Frontier Developments.
