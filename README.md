# TMC — The Multipurpose Companion

Companion en tiempo real de bajo consumo para **Elite Dangerous** (Windows 10/11, preparado para Linux).

> Estado: **Fase F0 («Cimientos») completada**. Consulta [`status.md`](status.md), la especificación [`docs/specs/2026-10-17-tmc-arquitectura-y-f0-design.md`](docs/specs/2026-10-17-tmc-arquitectura-y-f0-design.md) y el plan [`docs/plans/2026-10-17-tmc-f0-cimientos.md`](docs/plans/2026-10-17-tmc-f0-cimientos.md).

## Características de F0

- **Lectura en tiempo real:** seguimiento incremental por offset de bytes de los diarios `Journal.*.log`, `Status.json` (100–250 ms), archivos compañeros (`Cargo.json`, `NavRoute.json`, `Market.json`, `Outfitting.json`, `Shipyard.json`, `ModulesInfo.json`, `Backpack.json`, `ShipLocker.json`), detección del proceso del juego y matriz de color HUD (`GraphicsConfigurationOverride.xml`).
- **Almacenamiento e historial:** SQLite en modo WAL (`tmc.db`) con importación en segundo plano reanudable por archivo y offset, y reconstrucción automática si la base de datos se corrompe.
- **Servidor local y protocolo WebSocket:** actor central que publica topics con *coalescing* a máx. 10 Hz (`hello`, `snapshot`, `update`, `alert`, `import-progress`) y endpoints REST (`/api/health`, `/api/history/events`, `/api/config`, `/api/actions/{name}`, `/api/pairing`).
- **Interfaz adaptable (2560×1080 UW / 1080p / móvil):** panel general (`dashboard`) con Comandante, Nave, Ubicación, Status en vivo, Ruta (con copia al portapapeles del siguiente sistema), Sesión y Alertas; 5 idiomas (`es`, `en`, `fr`, `de`, `pt`) con Fluent y tema dinámico del HUD del jugador.
- **Escritorio e instalador:** aplicación Tauri 2 (`TMC.exe`) con icono en bandeja del sistema, cierre a bandeja destruyendo los procesos `msedgewebview2.exe`, ahorro de memoria al minimizar, instancia única e instalador NSIS (`TMC_0.1.0_x64-setup.exe`).

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

1. Abre **Ajustes** (icono `⚙` en la barra lateral), activa **Modo LAN (requiere reiniciar)** y pulsa **Guardar**.
2. Reinicia TMC para que el servidor escuche en la interfaz de red local (`0.0.0.0`).
3. Abre **Emparejar** (icono `⊞` en la barra lateral desde el propio PC en `127.0.0.1`) y escanea el código **QR** con tu tablet o móvil.
4. La URL del QR incluye el token aleatorio de 128 bits (`?token=...`), que establece una cookie `HttpOnly; SameSite=Strict` para autenticar las conexiones HTTP y WebSocket. Sin token válido, cualquier petición desde fuera de loopback recibe `401 Unauthorized`.

> Nota: en F0 la conexión LAN utiliza HTTP sin TLS (no es instalable como PWA independiente).

## Pruebas y presupuesto de consumo

```powershell
# Formato, linter y tests unitarios/integración en Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Verificación de tipos, tests unitarios (Vitest) y test e2e (Playwright) en UI
npm.cmd --prefix ui run check
npm.cmd --prefix ui test
npx.cmd --prefix ui playwright test

# Medición del presupuesto de consumo (< 150 MB Private Bytes, < 1 % CPU en reposo)
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-footprint.ps1 `
  -Exe target/release/TMC.exe `
  -JournalDir "$env:TEMP\tmc-replay" `
  -Seconds 120
```

## Licencia y marcas

Licenciado bajo **PolyForm Noncommercial 1.0.0** (ver [`LICENSE`](LICENSE)). Avisos de dependencias de terceros en [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

*Elite Dangerous* es una marca registrada de Frontier Developments plc. TMC no está afiliado ni respaldado por Frontier Developments.
