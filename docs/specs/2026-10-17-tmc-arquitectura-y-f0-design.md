# TMC — The Multipurpose Companion · Especificación de arquitectura y Fase 0

- **Fecha:** 2026-10-17
- **Estado:** pendiente de revisión por el propietario (Danikuso)
- **Investigación base:** `docs/research/informe-investigacion.md` y `docs/research/notes/`

---

## 1. Contexto y objetivo

TMC es un companion de escritorio para Elite Dangerous (PC) que muestra **en tiempo real toda la información útil para cualquier rol** (explorador, exobiólogo, cazarrecompensas, minero, pasajeros/misiones, comerciante, ingenieros, fleet carrier, colonización, Powerplay, BGS, a pie, crimen/piratería) y aspira a **sustituir al conjunto de herramientas actuales** (EDMC, EDDiscovery, SrvSurvey, Observatory, EDOMH, pestañas de Inara/Spansh…).

Se usa **mientras el juego está abierto**, principalmente en una **segunda pantalla ultrapanorámica de 29" (2560×1080)**, por lo que el consumo de recursos y el respeto a la inmersión son requisitos de primer nivel.

### 1.1 Lo que dijo el propietario (requisitos explícitos)

| # | Requisito |
|---|---|
| R1 | Primero uso personal; después repositorio público en GitHub. Repo privado desde ya. |
| R2 | Juega todos los roles; prioridad: exploración, cazarrecompensas, minería, pasajeros, comercio. |
| R3 | Segunda pantalla UW 29". |
| R4 | Ventana sin bordes; overlay solo si es ligero e integrado — la inmersión es lo primero. |
| R5 | Núcleo en **Rust** (también como aprendizaje). |
| R6 | Windows ahora, diseño preparado para Linux. |
| R7 | Distribución por instalador. |
| R8 | CAPI de Frontier: sí. EDDN: sí (contribuir). Inara y EDSM: sí. |
| R9 | Nombre: **TMC — The Multipurpose Companion**. |
| R10 | Importar todo el historial de diarios. |
| R11 | i18n: es, en, fr, de, pt (+ ampliable). |
| R12 | Voz local con **KokoroTTS**; alemán sin voz. |
| R13 | Overlay planificado (fase posterior). |
| R14 | Copiar siguiente sistema de ruta al portapapeles. |
| R15 | Atajos / Stream Deck / VoiceAttack: planificar, no implementar aún. |
| R16 | Estética: estilo HUD del juego con los colores del HUD del jugador. |
| R17 | Presupuesto: < 150 MB RAM y < 1 % CPU en reposo (sin voz), sin animaciones continuas. |
| R18 | Modular: poder añadir módulos en el futuro. |
| R19 | Licencia: código público pero **nadie puede venderlo**. |

### 1.2 Criterios de éxito (globales)

- **Latencia** evento del juego → interfaz: p95 < 500 ms.
- **Consumo** (app de escritorio, ventana visible, sin voz, juego en marcha): RAM total del árbol de procesos < 150 MB; CPU en reposo < 1 % de un núcleo; GPU ≈ 0 % en reposo.
- **Robustez:** ninguna línea corrupta, evento desconocido, archivo bloqueado o servicio caído provoca un cierre de la app.
- **Fuera de línea:** todo lo que procede de archivos locales funciona sin red.

---

## 2. Decisiones de base

| Decisión | Elección | Motivo |
|---|---|---|
| Lenguaje del núcleo | Rust (edición 2024, toolchain estable) | R5; rendimiento de importación (~15× vs CPython en inserciones SQLite); bajo consumo. |
| Envoltorio de escritorio | **Tauri 2** | Instalador ~3 MB, WebView2 compartido, ventanas transparentes para overlay, soporte Linux (WebKitGTK). |
| Interfaz | **Svelte 5 + TypeScript + Vite** | Sin DOM virtual, runtime mínimo, buena ergonomía; la misma UI se sirve a navegadores en LAN. |
| Gráficas | uPlot | ~50 KB, canvas, muy eficiente. |
| Base de datos | SQLite vía `rusqlite` (feature `bundled`), modo WAL | Embebida, rápida, multiplataforma. |
| Servidor local | `axum` + `tokio` (HTTP + WebSocket) | Una única fuente para ventana Tauri, navegador en LAN, overlay y futuras integraciones. |
| i18n | **Fluent** (`.ftl`), compartido entre Rust (`fluent-bundle`) y UI (`@fluent/bundle`) | Un solo formato y una sola carpeta de traducciones para UI, avisos y voz. |
| Voz | KokoroTTS (ONNX) en **proceso aparte** | Aísla ~200–300 MB de RAM y la licencia GPL de `espeak-ng`; carga bajo demanda. |
| Secretos | Almacén de credenciales del SO (`keyring`) | Claves de Inara/EDSM y tokens CAPI nunca en archivos de texto ni en el repo. |
| Licencia | **PolyForm Noncommercial 1.0.0** | Código público, uso y modificación permitidos, **uso comercial prohibido** (R19). |

### 2.1 Consecuencias de la licencia

- No se incorpora código ni datos **GPL/AGPL** al repositorio (p. ej. los criterios de exobiología de SrvSurvey). Esos conjuntos se construyen de forma propia a partir de fuentes con licencia compatible o con permiso explícito del autor.
- Consumir APIs AGPL (Ardent) por red es correcto: no se incorpora su código.
- `espeak-ng` (GPL) solo se usa como ejecutable independiente invocado por el proceso de voz; no se enlaza.
- Contribuciones externas (cuando el repo sea público): se exigirá un CLA sencillo para que el propietario conserve la titularidad.
- Aviso de marca: «Elite Dangerous» es marca de Frontier Developments; TMC no está afiliado ni usa logotipos ni recursos gráficos del juego.

---

## 3. Arquitectura

### 3.1 Workspace de Cargo

```
TheMultipurposeCompanion/
├─ crates/
│  ├─ tmc-model/      Tipos: eventos del diario, Status, archivos compañeros, GameState, protocolo UI
│  ├─ tmc-journal/    Localización de carpeta, lectura incremental del diario y archivos compañeros
│  ├─ tmc-store/      SQLite: esquema, migraciones, importación de historial, caché de APIs
│  ├─ tmc-core/       Bus de eventos, actor de estado, registro de módulos, temas (topics) y deltas
│  ├─ tmc-modules/    Un submódulo por rol (dashboard en F0; resto en fases posteriores)
│  ├─ tmc-net/        Clientes HTTP/ZeroMQ (fases F1+): EDDN, Ardent, Spansh, tick.edcd.io, EDSM, Inara, CAPI
│  ├─ tmc-server/     axum: estáticos de la UI, WebSocket, API REST de acciones, seguridad LAN
│  ├─ tmc-i18n/       Carga de Fluent para Rust
│  └─ tmc-replay/     Binario: reproduce diarios reales como si el juego estuviera en marcha
├─ apps/
│  ├─ desktop/        Tauri 2: ventana principal, bandeja, (overlay en F8)
│  └─ headless/       Mismo núcleo sin ventana (Linux/servidor/depuración)
├─ ui/                Svelte 5 + TS (shell, componentes, módulos, temas)
├─ locales/           es, en, fr, de, pt (.ftl)
├─ tts/               (F7) proceso de voz
└─ docs/
```

Reglas de dependencia (sin ciclos):

| Crate | Depende de |
|---|---|
| `tmc-model` | — (solo serde) |
| `tmc-i18n` | — |
| `tmc-journal` | `tmc-model` |
| `tmc-store` | `tmc-model` |
| `tmc-net` | `tmc-model` |
| `tmc-core` | `tmc-model`, `tmc-store` (fuentes de eventos entran por un trait, no depende de `tmc-journal`/`tmc-net`) |
| `tmc-modules` | `tmc-core`, `tmc-model`, `tmc-i18n` |
| `tmc-server` | `tmc-core`, `tmc-model` |
| `tmc-replay` | `tmc-model` |
| `apps/*` | todos los anteriores (composición) |

### 3.2 Modelo de concurrencia

Un único runtime `tokio` con estas tareas:

1. **Lector del diario** (`tmc-journal`): bucle de sondeo cada 250 ms (1 s si el juego no está activo). Emite `SourceEvent::Journal(JournalEvent)`.
2. **Lector de Status.json**: cada 250 ms compara tamaño+mtime; si cambia, lo relee y emite `SourceEvent::Status(Status)` solo si el contenido difiere del anterior.
3. **Actor de estado** (`tmc-core`): **único dueño** del `GameState` y de los módulos. Recibe eventos por un `mpsc`, aplica, pide a cada módulo que procese el evento y marca *topics* como sucios.
4. **Difusor**: cada 100 ms (máx. 10 Hz) publica en un `broadcast` los topics sucios con su valor completo.
5. **Escritor SQLite**: hilo bloqueante dedicado (`spawn_blocking`) con cola por lotes (commit cada 500 ms o 500 filas).
6. **Importador de historial**: hilo de baja prioridad; si el juego está activo, se limita a ~5 % de un núcleo.
7. **Servidor axum**: sirve UI y WebSockets.

Sin `Mutex` compartidos sobre el estado: todo cambio pasa por el actor (patrón actor = más fácil de razonar y de testear).

---

## 4. Lectura de datos locales (`tmc-journal`)

### 4.1 Localización de la carpeta

1. Ruta configurada por el usuario (si existe).
2. Windows: `SHGetKnownFolderPath(FOLDERID_SavedGames)` + `Frontier Developments\Elite Dangerous` (cubre carpetas redirigidas/OneDrive).
3. Linux: prefijo de Proton de Steam (`~/.steam/steam/steamapps/compatdata/359320/pfx/drive_c/users/steamuser/Saved Games/Frontier Developments/Elite Dangerous`) y variantes de Flatpak; si no se encuentra, se pide al usuario.

### 4.2 Diario (Journal)

- Archivos `Journal.YYYY-MM-DDTHHMMSS.NN.log` (fecha del nombre en hora local; `timestamp` interno en UTC). Se ordenan por nombre.
- **Sondeo, no notificaciones del SO**: Windows no notifica los *appends* de un archivo que otro proceso mantiene abierto. Se mantiene el archivo actual abierto (compartido lectura/escritura) y se lee desde el último *offset* de bytes.
- El watcher del SO (`notify`) solo se usa para detectar la **creación de un archivo de diario nuevo**; además, el sondeo comprueba cada 1 s si hay uno más reciente (por si el evento del SO no llega).
- Solo se consumen **líneas completas** (terminadas en `\n`; tolera CRLF). Una línea parcial se conserva en buffer hasta completarse.
- Líneas de hasta ~40 KB: buffer dinámico, sin límites rígidos.
- JSON inválido → se registra en log con archivo+línea y se descarta.
- Al iniciar: se procesa el **último diario** desde el principio para reconstruir el estado de la sesión actual (como hacen EDMC/EDDiscovery), sin emitir avisos sonoros/visuales por eventos pasados (flag `replaying=true`).

### 4.3 Archivos compañeros

| Archivo | Cuándo se lee |
|---|---|
| `Status.json` | Sondeo 250 ms (tamaño+mtime) |
| `Cargo.json` | Tras evento `Cargo` |
| `Market.json` | Tras evento `Market` |
| `Outfitting.json` | Tras evento `Outfitting` |
| `Shipyard.json` | Tras evento `Shipyard` |
| `ModulesInfo.json` | Tras evento `ModuleInfo` |
| `NavRoute.json` | Tras evento `NavRoute` (y `NavRouteClear` lo vacía) |
| `Backpack.json` | Tras evento `Backpack` |
| `ShipLocker.json` | Tras evento `ShipLocker` |
| `FCMaterials.json` | Tras evento `FCMaterials` |

Lectura robusta: si el archivo está vacío, bloqueado o el JSON es incompleto → reintento hasta 10 veces cada 50 ms; si sigue fallando, se descarta esa actualización y se registra.

### 4.4 Tipado de eventos (`tmc-model`)

- `JournalEvent { timestamp, event: String, kind: EventKind, raw: Box<RawValue> }`.
- `EventKind` es un `enum` con variantes **tipadas** para los eventos que usa algún módulo, y `EventKind::Other` para el resto. **Un evento desconocido nunca es un error.**
- Los campos se deserializan con `#[serde(default)]` y opcionales donde el juego los omite; los campos `_Localised` se conservan para mostrar nombres traducidos por el propio juego.
- La lista de eventos y campos se completa con la documentación oficial v38 **más** los eventos no documentados (colonización, `PowerplayMerits`, `CarrierLocation`, etc.) observados en el código de EDDiscovery y en diarios reales.
- `Status` incluye decodificación de `Flags` (bits 0–31) y `Flags2` (bits 0–22, incl. SCO activo), `GuiFocus`, pips, combustible, etc.

### 4.5 Detección de sesión

- `Fileheader` / `LoadGame` → sesión iniciada; `Shutdown` → finalizada.
- Galaxia: `gameversion` ≥ 4.0 → **Live**; < 4.0 → **Legacy** (Legacy se muestra, pero nunca se envía a EDDN/EDSM/Inara).
- Además, comprobación ligera cada 10 s de si el proceso del juego está vivo (para detectar cierres abruptos sin `Shutdown`).

---

## 5. Estado y módulos (`tmc-core`, `tmc-modules`)

### 5.1 GameState (núcleo común)

Mantenido por el actor, alimentado por el diario y Status:

- **Comandante:** nombre, FID, créditos, préstamo, rangos y progreso (incl. Soldier/Exobiologist), reputación con superpotencias, escuadrón.
- **Nave:** tipo, nombre, ident, `ShipID`, casco, módulos (`Loadout`), combustible principal/depósito, carga (con detalle de `Cargo.json`), rebuy, masa sin carga.
- **Ubicación:** sistema (nombre, `SystemAddress`, coordenadas), cuerpo, estación (nombre, tipo, `MarketID`), estado (atracado, aterrizado, supercrucero, hiperespacio, a pie, SRV, caza, taxi, multitripulación).
- **Status en vivo:** flags decodificados, pips, grupo de fuego, `GuiFocus`, latitud/longitud/altitud/rumbo, estado legal, oxígeno/salud/temperatura a pie.
- **Ruta:** contenido de `NavRoute.json` + destino de `FSDTarget` (saltos restantes).
- **Sesión:** inicio, duración, saltos, distancia, créditos ganados/gastados por categoría.

### 5.2 Contrato de módulo

```rust
pub trait Module: Send {
    /// Identificador estable, p. ej. "dashboard", "exploration".
    fn id(&self) -> &'static str;
    /// Procesa un evento ya aplicado al GameState. Devuelve los topics que cambió.
    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet;
    /// Valor serializable de un topic para la UI.
    fn topic_value(&self, topic: &str) -> Option<serde_json::Value>;
    /// Relevancia actual (para activación automática de vistas).
    fn relevance(&self, state: &GameState) -> Relevance { Relevance::Normal }
    /// Migraciones SQL propias (tablas del módulo), opcional.
    fn migrations(&self) -> &'static [Migration] { &[] }
}
```

- `ModuleCtx` permite: emitir **alertas** (`Alert { severity, key_fluent, args }`), escribir en tablas propias, pedir datos a `tmc-net` (fases futuras).
- Los módulos se registran en una lista estática (`registry.rs`). **Añadir un módulo** = crear su carpeta en `crates/tmc-modules/src/<id>/` + su carpeta en `ui/src/modules/<id>/` + una línea en cada registro. No se toca el núcleo.
- Plugins de terceros en tiempo de ejecución (WASM): **fuera de alcance**; el contrato se diseña para que sea posible más adelante.

### 5.3 Activación automática por contexto

Cada módulo informa su `Relevance` (`Hidden`, `Normal`, `Active`, `Urgent`). La UI puede (si el usuario lo activa) traer al frente la disposición asociada: p. ej. lanzar un prospector → minería; FSS abierto (`GuiFocus`=FSS) → exploración; `UnderAttack` → combate.

---

## 6. Persistencia (`tmc-store`)

- Ubicación: Windows `%LOCALAPPDATA%\TMC\tmc.db`; Linux `$XDG_DATA_HOME/tmc/tmc.db` (crate `directories`).
- SQLite con `journal_mode=WAL`, `synchronous=NORMAL`.
- Migraciones numeradas propias (tabla `schema_version`); cada módulo aporta las suyas con prefijo.

Esquema F0:

```sql
CREATE TABLE journal_file (
  id INTEGER PRIMARY KEY, name TEXT UNIQUE NOT NULL, size INTEGER NOT NULL,
  offset INTEGER NOT NULL, cmdr TEXT, gameversion TEXT, galaxy TEXT, complete INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE event (
  id INTEGER PRIMARY KEY, file_id INTEGER NOT NULL REFERENCES journal_file(id),
  line INTEGER NOT NULL, ts TEXT NOT NULL, name TEXT NOT NULL, cmdr TEXT, json TEXT NOT NULL,
  UNIQUE(file_id, line)
);
CREATE INDEX event_name_ts ON event(name, ts);
CREATE INDEX event_cmdr_ts ON event(cmdr, ts);
CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE api_cache (key TEXT PRIMARY KEY, fetched_at TEXT NOT NULL, ttl_s INTEGER NOT NULL, body BLOB NOT NULL);
```

- **El evento original se guarda siempre** → cualquier estadística o fórmula futura puede recalcularse sobre todo el historial.
- **Importación de historial:** recorre todos los `Journal.*.log`, reanudable por `journal_file.offset`, idempotente (`UNIQUE(file_id,line)`). Progreso visible en la UI. Inserciones en transacciones de 5 000 filas. Si se detecta excesivo tamaño, compresión (zstd) del JSON como mejora posterior, no en F0.
- Soporte de **varios comandantes**: cada evento lleva `cmdr` (del `LoadGame`/`Commander` vigente en ese archivo).

---

## 7. Servidor y protocolo (`tmc-server`)

### 7.1 Red y seguridad

- Por defecto escucha **solo en `127.0.0.1`** (puerto 47320, configurable; si está ocupado, el siguiente libre).
- **Modo LAN** (opcional, desactivado por defecto): escucha en la IP local; requiere **token** aleatorio de 128 bits, mostrado como **código QR** con la URL completa para emparejar tablet/móvil; validación de cabeceras `Host` y `Origin` (anti DNS-rebinding y secuestro de WebSocket). Sin HTTPS en F0 (sin PWA instalable; se documenta).
- La ventana Tauri se conecta al mismo servidor local (un único camino de datos para todas las vistas).

### 7.2 Protocolo WebSocket (`/ws`)

Mensajes JSON:

```jsonc
// servidor → cliente
{ "type": "hello", "version": 1, "locale": "es", "topics": ["core.cmdr", "core.ship", ...] }
{ "type": "snapshot", "topics": { "core.ship": { ... }, "core.location": { ... } } }
{ "type": "update", "seq": 1234, "topics": { "core.status": { ... } } }   // máx 10 Hz, solo topics sucios, valor completo
{ "type": "alert", "id": "...", "severity": "warn", "key": "alert-low-fuel", "args": { "pct": 18 } }
{ "type": "import-progress", "done": 812, "total": 2310 }

// cliente → servidor
{ "type": "subscribe", "topics": ["core.*", "dashboard.*"] }
{ "type": "action", "name": "clipboard.copy-next-system" }
```

- Se envía el **valor completo del topic** (los topics se diseñan pequeños). Más simple y robusto que parches JSON; si un topic crece (p. ej. listas largas), se pagina vía REST.
- REST mínimo: `GET /api/health`, `GET /api/history/events?name=&from=&to=` (paginado), `POST /api/actions/{name}`. Las acciones REST son la base futura de atajos, Stream Deck y VoiceAttack (R15).

---

## 8. Interfaz (`ui/`)

### 8.1 Shell y disposición (pensada para 2560×1080)

- **Barra lateral** estrecha con iconos de módulos + indicador de relevancia.
- **Rejilla de paneles** de 4 columnas en UW (3 en 1920 px, 1 en móvil/tablet vertical). Cada módulo aporta **paneles** (componentes) que se colocan en la rejilla.
- **Disposiciones guardadas** por perfil (p. ej. «Exploración», «Combate») + opción de cambio automático por relevancia.
- **Barra superior fija** con lo esencial siempre visible: sistema, estado (atracado/supercrucero…), combustible, casco, carga, créditos y alertas.

### 8.2 Tema HUD

- Lee `GraphicsConfigurationOverride.xml` (Windows: `%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options\Graphics\`; Linux: dentro del prefijo Proton) y aplica su **matriz de color** a la paleta base naranja del juego → genera variables CSS (`--hud-primary`, `--hud-secondary`, …). Si no existe, paleta por defecto.
- Selector manual de tema (HUD automático / naranja clásico / personalizado) y modo alto contraste.
- Tipografía legible a distancia; contraste AA mínimo.

### 8.3 Rendimiento de la UI

- **Prohibidas** las animaciones CSS infinitas y `requestAnimationFrame` continuos. Transiciones solo en respuesta a cambios (< 200 ms).
- Al minimizar/ocultar la ventana: `IsVisible=false` en WebView2 + `MemoryUsageTargetLevel=Low`, y el WebSocket deja de recibir actualizaciones de alta frecuencia.
- Listas largas con virtualización; gráficas con uPlot.

### 8.4 i18n

- `locales/<lang>/*.ftl` con Fluent; idiomas iniciales: **es** (referencia), en, fr, de, pt.
- Nombres del juego (commodities, módulos, naves, materiales): primero el campo `_Localised` del propio diario; si no existe, tablas propias por idioma generadas a partir de los símbolos de FDevIDs.
- Formatos de número/fecha según locale (`Intl`).

### 8.5 Panel general (módulo `dashboard`, F0)

Paneles: **Comandante** (créditos, rangos con barra de progreso), **Nave** (casco, combustible + depósito, carga, rebuy, módulos dañados), **Ubicación** (sistema, cuerpo, estación, estado de vuelo), **Status en vivo** (escudos, tren, hardpoints, masa bloqueada, calor, interdicción, legalidad), **Ruta** (siguiente sistema, saltos restantes, copiar al portapapeles — R14), **Sesión** (duración, saltos, distancia, créditos ganados) y **Alertas** (combustible bajo, sobrecalentamiento, interdicción, bajo ataque, daño de casco, oxígeno/salud a pie).

---

## 9. Configuración, secretos y logs

- Configuración: `config.toml` en `%APPDATA%\TMC\` / `$XDG_CONFIG_HOME/tmc/` (carpeta del diario, idioma, puerto, LAN, alertas, disposiciones).
- Secretos (Inara, EDSM, tokens CAPI): `keyring` (Windows Credential Manager / Secret Service en Linux).
- **Client ID de CAPI**: nunca en el repo; se inyecta al compilar desde la variable `TMC_CAPI_CLIENT_ID` (archivo `.env` local o secreto de CI). Una compilación sin él funciona con CAPI desactivada.
- Logs: `tracing` + rotación diaria, 7 días, en la carpeta de datos. Nivel `info` por defecto.

---

## 10. Errores y resiliencia

| Situación | Comportamiento |
|---|---|
| Línea de diario corrupta | Log + se ignora |
| Evento desconocido | `EventKind::Other`, se guarda en SQLite |
| Status.json vacío/incompleto | Reintento corto; si falla, se omite esa lectura |
| Carpeta del diario no encontrada | Pantalla de configuración guiada |
| SQLite ocupado/corrupto | Reintento; si es corrupción, se renombra y se reconstruye desde los diarios |
| Pánico en un módulo | Se captura (`catch_unwind` en el actor), se desactiva el módulo y se avisa en la UI; el resto sigue |
| Servicio online caído (F1+) | Se marca como degradado en la UI, se sirve caché con su antigüedad |

---

## 11. Pruebas y calidad

- **Unitarias** por crate (`cargo test`): parser de líneas, decodificación de flags, aplicación de eventos al estado, migraciones.
- **Snapshots** (`insta`) del `GameState` tras procesar diarios de prueba.
- **Fixtures**: diarios reales **anonimizados** (nombre de comandante, FID, nombres de nave sustituidos) en `tests/fixtures/`.
- **`tmc-replay`**: reproduce uno o varios diarios a velocidad configurable escribiendo en una carpeta temporal que imita la del juego (incluido `Status.json`) → prueba de extremo a extremo sin abrir el juego.
- **UI**: Vitest para lógica; prueba de humo con Playwright contra `apps/headless` + replay.
- **Presupuesto de consumo**: script que lanza la app con replay y mide RAM/CPU del árbol de procesos; falla si supera los límites del §1.2.
- **CI (GitHub Actions)**: `fmt`, `clippy -D warnings`, tests y build en Windows y Linux.

---

## 12. Empaquetado

- Instalador **NSIS** generado por Tauri (Windows); `.AppImage`/`.deb` para Linux cuando se active.
- WebView2: modo de instalación *download bootstrapper* (Windows 11 ya lo trae).
- Actualización automática: **diferida** hasta que el repo sea público (el actualizador de Tauri necesita un origen accesible y clave de firma).
- Firma de código: diferida; candidata SignPath (gratuito para OSS) — a revisar por la licencia no comercial; alternativa Microsoft Store.

---

## 13. Hoja de ruta

Cada fase tiene su propia especificación + plan. Esta especificación cubre la arquitectura y **F0**.

| Fase | Contenido | Fuentes principales |
|---|---|---|
| **F0 Cimientos** | Workspace, lectura de diario/Status/compañeros, GameState, SQLite + importación, servidor + protocolo, shell Tauri, UI base, tema HUD, i18n, replay, CI, panel general, instalador | Archivos locales |
| F1 | Navegación (rutas Spansh, alcance de salto), exploración (valores de cuerpos, FSS/DSS), exobiología | Diario, Spansh, EDSM |
| F2 | Cazarrecompensas/combate, minería (prospector, t/h, mejores precios) | Diario, Ardent, Spansh |
| F3 | Comercio, misiones y pasajeros; recepción EDDN | Ardent, Spansh, EDDN |
| F4 | Ingenieros y materiales (desbloqueos, lista de la compra, traders) | Diario, datos estáticos propios |
| F5 | Fleet carrier, colonización, Powerplay 2.0, BGS (tick.edcd.io), a pie | Diario, CAPI, Raven Colonial |
| F6 | Cuentas: envío a EDDN, EDSM, Inara, CAPI (OAuth2 PKCE) | — |
| F7 | Voz (KokoroTTS; sin alemán) | — |
| F8 | Overlay ligero e integrado | — |
| F9 | Crimen y piratería con reglas de **Law & Disorder** (dic. 2026) | Diario |
| F10 | Atajos, Stream Deck, VoiceAttack; build Linux oficial | API de acciones |

> Nota de prioridad (R2): F1–F3 siguen el orden exploración → cazarrecompensas/minería → pasajeros/comercio. El envío a EDDN puede adelantarse a F3 si se desea.

---

## 14. Riesgos y pruebas de concepto pendientes

| Riesgo | Mitigación | Cuándo |
|---|---|---|
| Kokoro: RAM, latencia en CPU, dependencia `espeak-ng` (GPL) | Proceso aparte, carga bajo demanda, caché de frases; spike previo a F7 | Antes de F7 |
| Overlay: transparencia de WebView2 (alfa 0/255 en fondo) y foco | Spike con ventana Tauri transparente + click-through sobre el juego en ventana sin bordes | Antes de F8 |
| CAPI: aprobación de Frontier y formato de *redirect URI* | Solicitud ya enviada por el propietario; *deep link* `tmc://auth` con alternativa loopback | Antes de F6 |
| Documentación oficial del diario incompleta (v38) | Completar con código de EDDiscovery/EDMC y diarios reales; tests con fixtures | Continuo |
| Servicios de terceros frágiles (EDSM, Elite BGS) | Capa de degradación + caché + fuentes alternativas | F1+ |
| Law & Disorder cambia crimen/piratería | Reglas del módulo crimen versionadas por fecha/versión del juego | F9 |
| Linux: diferencias de WebKitGTK | CI en Linux desde F0; UI sin APIs exclusivas de Chromium | Continuo |

---

## 15. Alcance de F0 (criterios de aceptación)

F0 se considera terminada cuando:

1. `cargo test` y la CI pasan en Windows y Linux.
2. Con el juego abierto (o `tmc-replay`), la ventana de TMC muestra el **panel general** actualizado con p95 < 500 ms.
3. La importación del historial completo del propietario termina, es reanudable y muestra progreso.
4. Un navegador de otro dispositivo en la LAN, tras escanear el QR, ve la misma interfaz; sin token es rechazado.
5. La UI está disponible en es/en/fr/de/pt (en F0 basta con es completo y claves presentes en el resto) y usa los colores del HUD del jugador.
6. El script de consumo confirma < 150 MB y < 1 % CPU en reposo.
7. El botón/acción «copiar siguiente sistema» funciona.
8. Se genera un instalador NSIS que instala y arranca la app en un Windows limpio.
9. El repositorio incluye `LICENSE` (PolyForm Noncommercial 1.0.0), aviso de marca y `THIRD_PARTY_NOTICES`.

**Fuera de alcance de F0:** cualquier llamada a servicios online, módulos de rol distintos del panel general, voz, overlay, actualización automática, firma de código.
