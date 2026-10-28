# TMC — The Multipurpose Companion

[![Versión](https://img.shields.io/badge/versi%C3%B3n-0.1.0-orange.svg)](https://github.com/Danikuso/TheMultipurposeCompanion/releases)
[![Licencia: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/licencia-PolyForm%20Noncommercial%201.0.0-blue.svg)](LICENSE)
[![Plataforma](https://img.shields.io/badge/plataforma-Windows%2010%2F11%20%7C%20Linux-green.svg)]()
[![Inara Whitelist](https://img.shields.io/badge/Inara%20InAPI%20v1-Whitelist%20Oficial-success.svg)]()

Aplicación auxiliar de código abierto en tiempo real para ***Elite Dangerous*** (Live 4.0 / Odyssey y Legacy 3.8), desarrollada en **Rust**, **Tauri 2** y **Svelte 5**.

TMC lee de manera pasiva e incremental los archivos de diario (`Journal.*.log`) y de estado (`Status.json`, `NavRoute.json`, `Market.json`, `Cargo.json`, `ModulesInfo.json`, etc.) que el juego escribe en disco, organizando la telemetría y las herramientas de asistencia en 16 módulos especializados y 11 disposiciones de cabina.

---

## Principios de Diseño y Arquitectura

- **Asistencia secuencial sin bloqueos:** Cuando el usuario consulta una tarea cuyos requisitos previos aún no cumple (por ejemplo, modificar un módulo con un ingeniero que todavía no conoce o no ha desbloqueado), la interfaz expone primero de forma ordenada los pasos necesarios para desbloquearlo (descubrimiento, invitación y donación) leídos del diario, y a continuación el desglose de materiales e intercambios.
- **Eficiencia de recursos:** Núcleo escrito en Rust (edición 2024) con lectura de diarios por desplazamiento de bytes (*byte offset*), almacenamiento persistente en SQLite local (modo WAL) y límite de flujo de hasta 10 actualizaciones/s hacia la interfaz. Consumo habitual inferior a 60 MB de RAM (< 1 % CPU en reposo) y liberación del proceso WebView2 al minimizar a la bandeja del sistema.
- **Ergonomía «TMC Glass HUD»:** Cada panel dedica su tercio superior a las métricas prioritarias y controles directos, agrupando listados extensos en secciones desplegables con altura dinámica adaptativa y barras de desplazamiento horizontal fijas.
- **Datos locales y aislamiento Legacy:** Funciona de forma 100 % local si se desactivan las consultas en línea. Detecta automáticamente sesiones en la galaxia Legacy 3.8 y bloquea cualquier transmisión hacia servicios comunitarios externos (EDDN, EDSM, Inara).
- **Multilingüe:** Interfaz y locuciones traducidas íntegramente a 5 idiomas (`es`, `en`, `fr`, `de`, `pt`) mediante catálogos Fluent verificados por tests automatizados.

---

## Módulos Incluidos

| Módulo | Funciones principales |
|---|---|
| **Panel general (`Dashboard`)** | Estado del comandante, créditos, rangos, banderas en vivo (`Status.json`), resumen de sesión y alertas críticas de cabina. |
| **Nave (`Ship`)** | Auditoría rápida de equipamiento por rol (Exploración, Combate, Carga/Minería) y desglose de ranuras con integridad, prioridades energéticas e ingeniería aplicada. |
| **Navegación (`Navigation`)** | Integración con **Spansh Exact Plotter** (extrayendo masa vacía, depósitos, perfil FSD ingenierizado y Guardian Booster directamente del evento `Loadout`, sin exportar a Coriolis/EDSY) y **Autopista de Neutrones**, resolución por coordenadas 3D (`StarPos`) en sistemas no indexados, detección de desvío y copia automática del siguiente salto al portapapeles. |
| **Exploración (`Exploration`)** | Progreso de escaneo FSS, clasificación de estrellas repostables, tabla de cuerpos celestes y marcado de planetas recomendados para sondeo DSS según umbral configurable. |
| **Exobiología (`Exobiology`)** | Predicción de especies candidatas en Odyssey, cálculo GPS en tiempo real (latitud, longitud y radio planetario) de la distancia mínima clonal entre las 3 muestras, rumbo en superficie y registro de datos pendientes para Vista Genomics. |
| **Ingeniería (`Engineering`)** | Inventario de materiales de nave y taquilla de Odyssey, asistente `EngineerFlow` con guía de desbloqueo en 3 etapas, lista de la compra con reserva de materiales y propuestas de trueque con excedentes en el *Material Trader* más cercano. |
| **Combate (`Combat`)** | Datos del objetivo fijado (`ShipTargeted`), salud de subsistemas, seguimiento de Zonas de Conflicto, contabilidad de bonos/recompensas y telemetría Anti-Xeno (Thargoides). |
| **Misiones (`Misiones`)** | Registro de contratos activos con control de expiración, evaluación de rutas de pasajeros y matriz de agrupación de misiones de masacre por facción emisora. |
| **Crimen (`Crime`)** | Seguimiento de multas, recompensas, carga ilegal en bodega, nivel de notoriedad (decaimiento de 2 h en sesión activa por punto) y búsqueda de *Interstellar Factors* o mercados negros cercanos. |
| **Minería (`Mining`)** | Lectura en vivo de drones prospectores (`ProspectedAsteroid`), tasa de refinado en toneladas/hora, filtro de minerales de interés y búsqueda de estaciones de venta en radio configurable. |
| **Comercio (`Trade`)** | Inventario de bodega, comparativa de precios del mercado local (`Market.json`) y consulta de rutas comerciales filtradas por capacidad, plataforma y antigüedad de precios EDDN. |
| **Fleet Carrier** | Ubicación del portaaviones, cuenta atrás de saltos programados, nivel de Tritio en depósito y bodega, distribución de capacidad y presupuesto de mantenimiento semanal. |
| **Colonización** | Control de entregas de materias primas requeridas en proyectos de colonización de sistemas y consulta de estaciones cercanas con oferta disponible. |
| **Powerplay 2.0 y BGS** | Seguimiento de potencia vinculada, rango y méritos del ciclo semanal; tabla de influencia y estados activos/pendientes de facciones menores del sistema. |
| **Cuentas (`Accounts`)** | Conexión opcional con **Frontier CAPI** (OAuth2 PKCE para consultar flota remota en astilleros), **Inara InAPI v1** (whitelist oficial bajo el nombre `The Multipurpose Companion (TMC)` con envío por lotes máx. 2 req/min), **EDSM** y **EDDN**. |
| **Voz y Pantalla LAN** | Salida de voz TTS (*Windows SAPI5/OneCore* nativo o *KokoroTTS* neuronal local), consultas habladas STT (*Whisper AI Local* offline o motor del sistema), exportación de perfil `TMC-Copiloto.vap` para *VoiceAttack* y acceso desde tablets/móviles en la misma red local mediante **código QR**. |

---

## Instalación y Puesta en Marcha

### Windows 10 / 11 (64 bits)

1. Descarga el instalador **`TMC_0.1.0_x64-setup.exe`** desde la sección [Releases](https://github.com/Danikuso/TheMultipurposeCompanion/releases).
2. Ejecuta el instalador y sigue los pasos del asistente.

> [!NOTE]
> **Nota sobre Windows Defender SmartScreen:**  
> Al tratarse de un proyecto independiente de código abierto cuyos binarios no cuentan con un certificado comercial EV de pago, Windows puede mostrar la ventana preventiva *«Windows protegió su PC»* en la primera ejecución.  
> Para continuar, pulsa en **«Más información»** y después en **«Ejecutar de todas formas»** (o compila el binario directamente desde el código fuente siguiendo las instrucciones inferiores).

3. Inicia **TMC**. El programa detecta automáticamente la carpeta estándar de diarios en `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous\`.

### Archivos locales y parámetros de línea de comandos

- **Configuración:** `%APPDATA%\TMC\config.toml` (Windows) / `~/.config/tmc/config.toml` (Linux).
- **Base de datos SQLite:** `%APPDATA%\TMC\tmc.db` (Windows) / `~/.local/share/tmc/tmc.db` (Linux).
- **Argumentos CLI opcionales:**
  - `--journal-dir <carpeta>`: especifica una ruta personalizada para los archivos `Journal.*.log` y `.json`.
  - `--config <archivo>`: utiliza un archivo `config.toml` alternativo.
  - `--ui-dir <carpeta>`: sirve los recursos estáticos de la interfaz desde un directorio concreto.

---

## Manual de Instrucciones y Documentación

- **Manual de Instrucciones en PDF (imprimible / lectura offline):** [`website/Manual_TMC_Comandante.pdf`](website/Manual_TMC_Comandante.pdf) (código fuente HTML en [`docs/manual/manual_print.html`](docs/manual/manual_print.html)).
- **Portal Web de Documentación:** [`website/index.html`](website/index.html).
- **Especificaciones y Arquitectura Interna:** carpeta [`docs/specs/`](docs/specs/).

---

## Compilación desde el Código Fuente

### Requisitos previos
- **Rust** estable (`1.90+`, edición 2024) con herramientas de compilación nativas (MSVC Build Tools en Windows; `libwebkit2gtk-4.1-dev` en Linux para escritorio).
- **Node.js** `22+` y `npm`.
- **Tauri CLI v2** (`cargo install tauri-cli@^2`) si se desea empaquetar el instalador.

### Comandos

```powershell
# 1. Instalar dependencias del frontend y compilar la interfaz (Svelte 5 + Vite)
npm.cmd --prefix ui ci
npm.cmd --prefix ui run build

# 2. Verificar tipos y ejecutar pruebas automatizadas (Rust + Vitest)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm.cmd --prefix ui run check
npm.cmd --prefix ui run test

# 3. Compilar el ejecutable de escritorio en modo Release (o el servidor sin ventana)
cargo build -p tmc-desktop --release
cargo build -p tmc-headless --release

# 4. Generar el instalador NSIS (Windows)
Push-Location apps/desktop
cargo tauri build
Pop-Location
# Salida: target/release/bundle/nsis/TMC_0.1.0_x64-setup.exe
```

---

## Soporte, Reporte de Incidencias y Contribuciones

- **Desde la propia aplicación:** abre `Ajustes` → tarjeta **Soporte, Feedback y Comunidad** → pulsa **«Reportar problema o sugerencia en GitHub»** para abrir una incidencia con los metadatos básicos de versión y entorno prellenados (sin incluir datos personales).
- **En GitHub:** abre un ticket directamente en [GitHub Issues](https://github.com/Danikuso/TheMultipurposeCompanion/issues).

---

## Privacidad y Licencia

- **Licencia:** Este proyecto es de código abierto bajo la licencia **[PolyForm Noncommercial 1.0.0](LICENSE)** (permite consultar, compilar, modificar y redistribuir el código de forma gratuita con fines personales, educativos o comunitarios sin ánimo de lucro). Consulta también [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) para las licencias de dependencias de terceros.
- **Privacidad:** TMC no recopila telemetría de uso propia. Las credenciales de EDSM e Inara se almacenan localmente en SQLite, Frontier CAPI emplea OAuth2 PKCE directo contra Frontier Store y el identificador de envío a EDDN se seudonimiza con una sal criptográfica local.
- **Aviso legal:** *Elite Dangerous*, *Horizons* y *Odyssey* son marcas registradas de Frontier Developments plc. TMC es una herramienta comunitaria independiente no afiliada ni respaldada oficialmente por Frontier Developments.
