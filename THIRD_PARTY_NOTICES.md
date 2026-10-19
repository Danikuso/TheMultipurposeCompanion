# Avisos de terceros

Cada dependencia directa se registra aquí al añadirla (nombre, versión, licencia).
Se registran las licencias admitidas por el proyecto (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, CC0 y las alternativas declaradas abajo). Una dependencia con licencia dual se utiliza bajo la alternativa compatible indicada.

## Dependencias directas

### serde
- Versión: 1.0.229
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-model, tmc-core

### serde_json
- Versión: 1.0.154
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-model, tmc-core

### chrono
- Versión: 0.4.45
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-model, tmc-store, tmc-net

### bitflags
- Versión: 2.13.2
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-model

### thiserror
- Versión: 2.0.21
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-model, tmc-store

### tracing
- Versión: 0.1.44
- Licencia: MIT
- Usada por: tmc-model, tmc-journal, tmc-store, tmc-core

<!-- Plantilla por dependencia:
### nombre
- Versión: x.y.z
- Licencia: MIT OR Apache-2.0
-->

### windows
- Versión: 0.61.3 (declarada compatible con 0.61.1)
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-journal (solo Windows; localización de Saved Games mediante Known Folders)

### tempfile
- Versión: 3.27.0
- Licencia: MIT OR Apache-2.0
- Usada por: tests de tmc-journal y tmc-store

### tokio
- Versión: 1.53.3
- Licencia: MIT
- Usada por: tmc-journal y tmc-core (tareas, temporizadores, canales y pool de E/S)

### notify
- Versión: 8.2.0
- Licencia: CC0-1.0
- Usada por: tmc-journal (aviso de creación de archivos; sondeo como respaldo)

### sysinfo
- Versión: 0.39.6
- Licencia: MIT
- Usada por: tmc-journal (detección del proceso del juego)

### roxmltree
- Versión: 0.21.1 (serie compatible declarada: 0.21)
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-journal (lectura XML del tema HUD)

### rusqlite
- Versión: 0.40.2
- Licencia: MIT
- Usada por: tmc-store (SQLite embebida mediante feature bundled; SQLite es de dominio público)

### uuid
- Versión: 1.28.0
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-core (identificadores UUID v4 de alertas)

### fluent-bundle
- Versión: 0.16.0
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-i18n (formato Fluent y fallback)

### fluent-syntax
- Versión: 0.12.0
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-i18n (enumeración de claves mediante el AST de Fluent)

### unic-langid
- Versión: 0.9.6
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-i18n (identificadores de idioma)

### rust-embed
- Versión: 8.13.0
- Licencia: MIT
- Usada por: tmc-i18n (recursos de traducción compartidos), tmc-server (feature `embed-ui`)

### axum
- Versión: 0.8.9
- Licencia: MIT
- Usada por: tmc-server (servidor HTTP y WebSocket)

### qrcode
- Versión: 0.14.1
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-server (generación de código QR SVG para emparejamiento LAN)

### tower
- Versión: 0.5.3
- Licencia: MIT
- Usada por: tests de tmc-server (`ServiceExt::oneshot`)

### tokio-tungstenite
- Versión: 0.30.0
- Licencia: MIT
- Usada por: tests de tmc-server (cliente WebSocket de prueba)

### futures-util
- Versión: 0.3.34
- Licencia: MIT OR Apache-2.0
- Usada por: tests de tmc-server (streams/sinks de WebSocket)

### hyper
- Versión: 1.12.0
- Licencia: MIT
- Usada por: tests de tmc-server (conexión HTTP/1 con upgrade sobre canal en memoria)

### hyper-util
- Versión: 0.1.21
- Licencia: MIT
- Usada por: tests de tmc-server (adaptador TokioIo / TowerToHyperService)

### toml
- Versión: 1.1.8+spec-1.1.0
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-app (lectura y escritura de `config.toml`)

### directories
- Versión: 6.0.0
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-app (rutas estándar de configuración y datos en Windows/Linux)

### arboard
- Versión: 3.7.0
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-app (copia del siguiente sistema de ruta al portapapeles)

### tracing-subscriber
- Versión: 0.3.23
- Licencia: MIT
- Usada por: tmc-app (formateo y filtrado de logs)

### tracing-appender
- Versión: 0.2.5
- Licencia: MIT
- Usada por: tmc-app (rotación diaria de archivos de log)

### anyhow
- Versión: 1.0.104
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-app, tmc-headless, tmc-replay (errores de arranque y ejecución)

### insta
- Versión: 1.49.0
- Licencia: Apache-2.0
- Usada por: tests de tmc-model (snapshots de estado final sobre fixtures reales)

### tauri
- Versión: 2.12.4
- Licencia: Apache-2.0 OR MIT
- Usada por: tmc-desktop (ventana principal, icono de bandeja y runtime de escritorio)

### tauri-build
- Versión: 2.7.3
- Licencia: Apache-2.0 OR MIT
- Usada por: tmc-desktop (script de compilación de recursos y capacidades de Tauri)

### tauri-plugin-single-instance
- Versión: 2.5.4
- Licencia: Apache-2.0 OR MIT
- Usada por: tmc-desktop (instancia única y enfoque de la ventana existente)

### tauri-plugin-window-state
- Versión: 2.5.0
- Licencia: Apache-2.0 OR MIT
- Usada por: tmc-desktop (persistencia de posición, tamaño y estado maximizado de la ventana)

### webview2-com
- Versión: 0.39.1
- Licencia: MIT
- Usada por: tmc-desktop (solo Windows; reducción de consumo de WebView2 al minimizar)

### windows-core
- Versión: 0.62.2
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-desktop (solo Windows; trait `Interface::cast` para `ICoreWebView2_19`)

## Dependencias directas de la UI (`ui/package.json`)

### @fluent/bundle
- Versión: ^0.18.0
- Licencia: Apache-2.0
- Usada por: ui (formateo de traducciones Fluent en navegador)

### svelte
- Versión: ^5.20.0
- Licencia: MIT
- Usada por: ui (componentes reactivos de interfaz)

### vite
- Versión: ^6.2.0
- Licencia: MIT
- Usada por: ui (empaquetado y servidor de desarrollo)

### @sveltejs/vite-plugin-svelte
- Versión: ^5.0.0
- Licencia: MIT
- Usada por: ui (integración de Svelte 5 con Vite)

### svelte-check
- Versión: ^4.1.0
- Licencia: MIT
- Usada por: ui (verificación estática de tipos en plantillas Svelte)

### typescript
- Versión: ~5.7.0
- Licencia: Apache-2.0
- Usada por: ui (tipado estático TypeScript)

### vitest
- Versión: ^3.0.0
- Licencia: MIT
- Usada por: tests de ui (ejecutor de pruebas unitarias)

### jsdom
- Versión: ^26.0.0
- Licencia: MIT
- Usada por: tests de ui (entorno DOM simulado para Vitest)

### @playwright/test
- Versión: ^1.51.0
- Licencia: Apache-2.0
- Usada por: tests e2e de ui (prueba de humo extremo a extremo)

## Dependencias Rust de red y TLS (F1)

### reqwest
- Versión: 0.13.5 (`default-features = false`; características `rustls`, `form`, `gzip`, `http2`, `charset`)
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-net (cliente HTTP de Spansh, EDSM y Canonn)

### rustls (y pila TLS transitiva de reqwest)
- Versión: rustls 0.23.46, rustls-webpki 0.103.15, rustls-platform-verifier 0.7.1, hyper-rustls 0.27.10, tokio-rustls 0.26.6, aws-lc-rs 1.18.2, aws-lc-sys 0.46.0
- Licencia: rustls e hyper-rustls Apache-2.0 OR ISC OR MIT; rustls-webpki ISC; aws-lc-rs ISC AND (Apache-2.0 OR ISC); aws-lc-sys ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0); rustls-platform-verifier y tokio-rustls MIT OR Apache-2.0.
- Usada por: tmc-net (TLS de reqwest; proveedor criptográfico aws-lc-rs)

### Otras dependencias transitivas de reqwest (revisión C19)
- Árbol revisado con `cargo metadata` + `cargo tree -p tmc-net -e normal` (Windows). Todas permisivas: MIT, Apache-2.0, ISC, BSD-3-Clause, Zlib, 0BSD, Unlicense, BSL-1.0 (solo como alternativa en `ryu`, que también es Apache-2.0) y Unicode-3.0 (tablas ICU de `icu_*`, `idna`, `url`; licencia permisiva del Consorcio Unicode con aviso). Sin GPL/AGPL/CC-BY(-SA).
- Incluye, entre otras: hyper 1.12.0, hyper-util 0.1.21, h2 0.4.20, http 1.5.0, tower-http 0.6.11, url 2.5.8, idna 1.1.0, encoding_rs 0.8.42 ((Apache-2.0 OR MIT) AND BSD-3-Clause), flate2 1.1.10, miniz_oxide 0.9.1, async-compression 0.4.51, serde_urlencoded 0.7.1, subtle 2.6.1 (BSD-3-Clause), untrusted 0.9.0 (ISC), ipnet 2.12.2, mime 0.3.17.
- Usadas por: tmc-net

## Revisión de dependencias transitivas (2026-10-18)

Evidencia: `cargo metadata --offline --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc`; recorrido de `resolve.nodes` desde los 12 miembros del workspace, incluyendo dependencias de compilación y pruebas. Resultado: 404 paquetes externos activos en Windows; ninguna licencia ausente y todas las dependencias directas con un aviso nominal en este documento. El cierre de dependencias normales/de compilación de `tmc-net` contiene 151 paquetes externos y no requiere GPL, AGPL ni CC-BY(-SA). La revisión corresponde a este `Cargo.lock` y plataforma; no sustituye una auditoría de otras plataformas o futuras actualizaciones.

### self_cell (transitiva de Fluent)
- Versión: 1.3.0.
- Declaración del paquete: `Apache-2.0 OR GPL-2.0-only`. **TMC utiliza la alternativa Apache-2.0**; no la GPL. El paquete instalado incluye `LICENSE-APACHE`, verificado junto con su `Cargo.toml`.
- Es la única coincidencia textual de GPL en las expresiones de licencia del árbol Windows revisado; no representa una obligación de usar GPL.

### Otras transitivas con licencia propia
- `clipboard-win` 5.4.1 y `error-code` 3.4.0: BSL-1.0 (**Boost Software License 1.0**).
- `cssparser` 0.37.0, `cssparser-macros` 0.7.1, `dtoa-short` 0.3.5, `option-ext` 0.2.0 y `selectors` 0.38.0: MPL-2.0; no se modifica su código.
- Los avisos de las pilas Unicode y criptográfica se conservan con sus expresiones compuestas indicadas arriba. Las expresiones antiguas con `/` en los manifiestos se registran como las declara cada paquete; no se infieren licencias nuevas.

## Datos estáticos propios de F1

La procedencia declarada en los archivos y la decisión de construcción están documentadas en [spike_licencias_f1.md](docs/research/notes/spike_licencias_f1.md), sin incorporar datasets de terceros. Los cuatro archivos llevan versión `2026-10-18` y licencia de datos propios del proyecto **PolyForm Noncommercial 1.0.0**:

| Archivo | Contenido y procedencia declarada | Verificación |
|---|---|---|
| `crates/tmc-modules/data/fsd.json` | 75 entradas FSD; hechos del juego transcritos por TMC, con procedencia por entrada. | Solo las entradas marcadas `verified: true` tienen la verificación registrada; el resto conserva `false`. Los ajustes con el mismo historial no constituyen validación independiente. |
| `crates/tmc-modules/data/species.json` | 117 especies, valores y CCR por género; hechos del juego transcritos por TMC. | Procedencia y estado `verified` explícitos; nombres localizados desde el diario y latinos como respaldo, según decisión del propietario. |
| `crates/tmc-modules/data/bio_rules.json` | 90 reglas redactadas por TMC a partir de condiciones conocidas del juego; estructura propia. | Condiciones y límites pendientes de contraste con historial; referencia adicional en `docs/research/notes/bio_rules_f1.md`. |
| `crates/tmc-modules/data/probes.json` | 7 bandas de radio con estimación de sondas; hechos transcritos por TMC. | Entradas `verified: false`; la UI identifica la estimación. |

Las menciones de SrvSurvey en `bio_rules.json` explican su exclusión como fuente; no son código ni datos importados. Las búsquedas en `crates/`, `ui/src/` y `locales/` solo encuentran esas declaraciones de procedencia. No se incorporan tablas de wikis, Coriolis, EDSY ni datos de Canonn. Canonn se usa solo como servicio de red (véase «Canonn» más abajo); sus fixtures son sintéticos.

## Servicio de red de exobiología (capa 3)

### Canonn (servicio de red GPL-3.0, no incorporado)
- Uso: consultas HTTP de solo lectura desde `tmc-net` (`getSystemPoi` por nombre de sistema y `codex/ref` de Biología) para refinar la predicción de especies con observaciones de otros comandantes. Activo desde 2026-10-19 con **permiso de Canonn** obtenido por el propietario ([spike_canonn_f1.md](docs/research/notes/spike_canonn_f1.md)).
- Licencia de sus datos y código: GPL-3.0. TMC lo utiliza **únicamente por red**; no se copia ni se enlaza su código ni se incorporan sus datos al repositorio (los fixtures son sintéticos). Las respuestas solo se guardan en la caché local con caducidad.
- Privacidad: solo salen el nombre del sistema y la categoría `Biology`; nunca nombre de comandante, FID ni otros datos del jugador. Límite propio de 1 petición/s.
- Opt-out: se desactiva con las consultas a servicios en línea; desactivado no se realiza ninguna petición. No se consulta en la galaxia Legacy ni durante el replay del diario.

## Servicios de red y datos de F2 (combate y minería)

### Ardent API (servicio de red AGPL, no incorporado)
- Uso: consultas HTTP de solo lectura desde `tmc-net` (mercados que compran un mineral, estaciones más cercanas con servicios) para «Dónde vender» e instalación de interés más cercana.
- Licencia del servicio: AGPL. TMC lo utiliza **únicamente por red** como cliente; **no se copia ni se enlaza ni se distribuye** su código ni se incorporan sus datos al repositorio. Las respuestas solo se guardan en la caché local de TMC con caducidad, y los fixtures de pruebas son sintéticos.
- Opt-out: el servicio puede desactivarse en los ajustes de minería/servicios; desactivado, TMC muestra «sin datos» sin realizar peticiones. No se envían datos de la galaxia Legacy.
- En compilaciones de depuración existe un transporte de pruebas que redirige Ardent a `http://127.0.0.1:` para los tests e2e (nunca activo en release).

### Dependencias nuevas en F2
- No se añaden dependencias de terceros nuevas (`Cargo.lock` y `ui/package.json` sin cambios de paquetes respecto a `main`). Solo se declaran dependencias internas o ya registradas: `chrono` (MIT OR Apache-2.0) ahora también en `tmc-core` y `tmc-net`, y `tmc-net` (crate propio) en `apps/headless`, solo para el transporte e2e de depuración.

### Datos estáticos propios de F2

Versión 1, licencia de datos propios del proyecto **PolyForm Noncommercial 1.0.0**; sin datasets de terceros (ni GPL/AGPL).

| Archivo | Contenido y procedencia declarada | Verificación |
|---|---|---|
| `crates/tmc-modules/data/ship_pads.json` | 47 naves con tamaño de plataforma mínimo; hechos del juego transcritos por TMC (pantalla de astillero) con procedencia por entrada. | Todas `verified: false`; con tamaño dudoso se toma la plataforma mayor (más conservador). Contraste con los `Loadout` del propietario mediante `eval`. |
| `crates/tmc-modules/data/mining_yield.json` | Estimación propia de toneladas por asteroide según contenido (Low/Medium/High). | Estimación de TMC, no datos de terceros; contraste en `eval` (sin casos en el historial del propietario: INSUFICIENTE). |

La constante de naves thargoides de `tmc-model` (AX) es una lista pequeña **no verificada** escrita por TMC (ver «Desviaciones» del plan de F2).

### flate2
- Versión: 1.1.9
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-eddn
