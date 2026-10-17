# Avisos de terceros

Cada dependencia directa se registra aquí al añadirla (nombre, versión, licencia).
Solo se admiten licencias permisivas (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, CC0).

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
- Usada por: tmc-model, tmc-store

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
