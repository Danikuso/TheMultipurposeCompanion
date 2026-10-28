# TMC — The Multipurpose Companion · Portal de Documentación Web

Sitio web estático de presentación técnica y documentación operativa de **TMC — The Multipurpose Companion**, proyecto de código abierto para *Elite Dangerous*.

## Cómo abrir la página

No requiere herramientas de compilación ni dependencias externas:

1. **Directamente desde el sistema de archivos:** abre [`index.html`](index.html) en cualquier navegador moderno.
2. **Mediante un servidor estático local (opcional):**
   ```bash
   python -m http.server 8080 --directory website
   ```
   O con Node.js:
   ```bash
   npx serve website
   ```

## Estructura del contenido

- **Presentación Técnica y Vista Previa Glass HUD:** Resumen de la arquitectura (Rust + Tauri 2 + Svelte 5), enlaces directos al repositorio en GitHub, instalador de la Release v0.1.0 y Manual de Instrucciones en PDF (`Manual_TMC_Comandante.pdf`), acompañado de una demostración interactiva de cómo se organizan los paneles de telemetría.
- **Arquitectura y Principios de Diseño (`#arquitectura`):** Explicación técnica de la asistencia secuencial sin bloqueos, lectura incremental de diarios en Rust, ergonomía Glass HUD (regla del 33 % y ausencia de scroll fantasma), almacenamiento local SQLite WAL con aislamiento de galaxia Legacy, servidor LAN mediante código QR y naturaleza abierta/auditable del código fuente.
- **Catálogo Técnico de los 16 Módulos (`#modulos`):** Fichas descriptivas con buscador y filtros por categoría para todos los módulos operativos (Panel General, Nave, Navegación con Spansh Exact Plotter y Autopista de Neutrones, Exploración, Exobiología, Combate/AX, Minería, Comercio, Misiones, Ingeniería con `EngineerFlow`, Fleet Carrier, Colonización, Powerplay 2.0, BGS, Crimen y Cuentas con Frontier CAPI/Inara/EDSM/EDDN).
- **Guía Rápida de Configuración y Uso (`#manual`):** Instrucciones resumidas de instalación, anatomía de interfaz (`Auto-companion`, Disposiciones vs. Módulos), conexión LAN QR, síntesis y reconocimiento de voz (SAPI5/KokoroTTS, Whisper AI Local y VoiceAttack `.vap`), vinculación opcional de cuentas y comandos para compilar desde el código fuente.
- **Flujos de Trabajo Paso a Paso (`#flujos`):** Cuatro procedimientos prácticos de operación (muestreo genético en Exobiología, desbloqueo de ingenieros e intercambio de excedentes, seguimiento de misiones por facción emisora y regularización de notoriedad en *Interstellar Factors*).
- **Preguntas Técnicas Frecuentes (`#faq`):** Aclaraciones sobre lectura pasiva de diarios, huella de memoria RAM/CPU, compatibilidad Live 4.0 / Legacy 3.8, funcionamiento sin conexión, soporte en Linux y licencia de código abierto (`PolyForm Noncommercial 1.0.0`).
- **Buscador Instantáneo (`Ctrl + K`):** Índice local para localizar rápidamente cualquier módulo, ajuste o instrucción.
