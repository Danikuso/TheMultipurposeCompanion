# Avisos de terceros

Cada dependencia directa se registra aquí al añadirla (nombre, versión, licencia).
Solo se admiten licencias permisivas (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, CC0).

## Dependencias directas

### serde
- Versión: 1.0.229
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-model

### serde_json
- Versión: 1.0.154
- Licencia: MIT OR Apache-2.0
- Usada por: tmc-model

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
- Usada por: tmc-model, tmc-journal, tmc-store

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
- Usada por: tmc-journal (tareas, temporizadores, canales y pool de E/S)

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
