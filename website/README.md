# TMC — The Multipurpose Companion · Web Oficial

Sitio web oficial, completo e interactivo de **TMC — The Multipurpose Companion** para *Elite Dangerous*.

## 🚀 Cómo visualizar la web

La web es 100% estática, ligera y responsive. Puedes abrirla de dos formas:

### 1. Directamente en tu navegador (Sin servidor)
Haz doble clic sobre [`index.html`](index.html) para abrirlo directamente en Chrome, Edge, Firefox, Brave o Safari (protocolo `file://`).

### 2. Con un servidor web local
Si prefieres servirla con recarga en vivo o en red local:

**Con Python:**
```bash
python -m http.server 8080 --directory website
```
Y abre en tu navegador: `http://localhost:8080`

**Con Node / npx:**
```bash
npx serve website
```

## 🌌 Contenido incluido

- **Header HUD Navegable:** Acceso directo a secciones, búsqueda rápida con `Ctrl + K` y botón de inicio.
- **Hero & Simulador Interactivo de Cabina:** Widget funcional con telemetría en vivo, alternador de 4 modos de juego (Exploración, Combate AX, Minería, Comercio), copia de saltos y prueba de síntesis de voz con Web Speech API.
- **Ventajas y Diferenciales («Por qué TMC»):** Filosofía del ayudante proactivo (nunca un callejón sin salida), Auto-Companion con detección de contexto, copiloto auditivo OneCore/KokoroTTS, arquitectura Rust < 150 MB RAM, pantalla remota LAN vía código QR y tabla comparativa exhaustiva contra EDDiscovery, EDMarketConnector e Inara.
- **Catálogo de los 16 Módulos:** Fichas detalladas con buscador instantáneo y filtrado por categorías de todos los módulos operativos (Dashboard, Nave, Navegación Spansh, Exploración, Exobiología, Combate AX, Minería, Comercio, Misiones Massacre Stacking, Ingeniería EngineerFlow, Fleet Carrier, Colonización, Powerplay 2.0, BGS, Crimen y Cuentas).
- **Normativa de Ergonomía «TMC Glass HUD»:** Explicación técnica de la regla del 33% de información prioritaria básica, eliminación de scroll fantasma y scrollbars pegajosos.
- **Manual de Usuario Oficial:** 6 capítulos completos (Requisitos e Instalación, Detección de Diarios sin bloqueos de disco, Anatomía de Interfaz y Disposiciones, Copiloto TTS y Perfil VoiceAttack `.vap`, Pantalla Remota LAN para Tablets con QR y Privacidad con Aislamiento Legacy).
- **Tutoriales Prácticos de Cabina:** Guías paso a paso para ganar 100M+ Cr en Exobiología, desbloquear ingenieros desde cero con `EngineerFlow`, Massacre Stacking en combate y limpieza de notoriedad en Interstellar Factors.
- **Preguntas Frecuentes (FAQ):** Acordeones interactivos con respuestas a rendimiento, legalidad, Odyssey/Horizons y modo sin conexión.
- **Buscador Rápido Instantáneo:** Diálogo modal accesible con `Ctrl + K` con filtrado dinámico en tiempo real y salto inmediato a la sección.
