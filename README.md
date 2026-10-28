# TMC — The Multipurpose Companion

[![Versión](https://img.shields.io/badge/versión-0.1.0-orange.svg)](https://github.com/Danikuso/TheMultipurposeCompanion/releases)
[![Licencia: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/licencia-PolyForm%20Noncommercial-blue.svg)](LICENSE)
[![Plataforma](https://img.shields.io/badge/plataforma-Windows%2010%2F11%20%7C%20Linux-green.svg)]()
[![Inara Whitelist](https://img.shields.io/badge/Inara-Oficialmente%20Aprobado-success.svg)]()

> **El copiloto y ayudante integral de bajo consumo para *Elite Dangerous*.**  
> Diseñado tanto para los pilotos más novatos como para los comandantes más experimentados.

---

## 🌌 Filosofía: El Ayudante Integral (Nunca un callejón sin salida)

TMC (**The Multipurpose Companion**) nace con un objetivo innegociable: **nunca dejar al comandante bloqueado**.
- Si intentas ingenierizar un módulo y todavía no tienes desbloqueado o invitado al ingeniero, TMC actúa de guía pedagógica: te muestra primero la **secuencia de desbloqueo** (descubrimiento, invitación y donación) y **después** los materiales y cálculos de recetas.
- Sin cálculos mentales: barras de llenado, steppers de progreso, semáforos de existencias y tarjetas métricas de lectura rápida.
- Telemetría en tiempo real a 10 Hz con un consumo inferior a **50 MB de RAM** y **0 % de CPU en reposo**, respetando estrictamente el rendimiento de tu PC durante el combate o la exploración.

---

## ⚡ Características Principales

### 🧭 1. Navegación y Spansh Exact Plotter
- **Física de salto real y automatizada:** Olvídate de exportar tus naves a Coriolis o EDSY. TMC lee en tiempo real la telemetría viva de tu nave (masa sin combustible, depósitos de reserva y principal, masa óptima del FSD con ingeniería y multiplicadores Guardian Booster) y los envía directamente al motor de Spansh Exact Plotter.
- **Autopista de Neutrones:** Planificación optimizada para saltos con sobrecarga de conos de estrellas de neutrones.
- **Rescate por coordenadas 3D:** Si te encuentras en el espacio profundo en sistemas procedimentales no indexados, TMC resuelve automáticamente tu posición por coordenadas estelares tridimensionales.
- **Copiado automático al portapapeles:** Copia el siguiente sistema de la ruta directamente al portapapeles de Windows para pegar en el mapa de la galaxia del juego.

### 🧬 2. Exploración y Exobiología
- **Exploración de sistemas:** Tabla de cuerpos virtualizada, progreso de escaneo FSS, valores estimados de cartografía y consejo inteligente de mapeo de planetas valiosos.
- **Exobiología de campo:** Semáforo de muestreo genético (Muestra 1, 2 y 3), distancia mínima de clonación por especie con brújula y valor total de bio-datos sin vender.

### 🔧 3. Ingenieros y Materiales (`EngineerFlow`)
- **Guía de desbloqueo paso a paso:** Progreso de los 25 ingenieros de la galaxia.
- **Inventario vivo y recetas:** Seguimiento de materiales manufacturados, codificados y elementos en tiempo real.
- **Calculadora de intercambios:** Soluciones óptimas de intercambio con comerciantes de materiales basadas en tus excedentes reales.

### ⚔️ 4. Combate, Piratería y Crimen
- **Seguimiento táctico:** Escaneo de nave objetivo en tiempo real, integridad de casco/escudos y subsistemas.
- **Reglas oficiales de Crimen:** Totalmente adaptado a las normativas de jurisdicción y *Law & Disorder* de Frontier.
- **Massacre Mission Stacking:** Optimizador de misiones de masacre de piratas entre facciones distintas para maximizar créditos por cada baja.

### ⛏️ 5. Minería y Comercio
- **Minería de superficie y núcleo:** Rendimiento en toneladas/hora (t/h), detección de prospectores y valor estimado de la bodega.
- **Comercio Spansh:** Cálculo de rutas comerciales seguras, filtros de plataforma (Grande/Mediana) y edad de precios de mercado.

### 🚀 6. Flota, Fleet Carrier y Powerplay 2.0
- **Seguimiento de Portaaviones:** Salto programado, combustible de Tritio, bodega y finanzas del Carrier.
- **Powerplay 2.0:** Control de méritos, rango de lealtad y estado de las potencias.

### 🌐 7. Integraciones Oficiales Aprobadas
- **Inara InAPI v1:** Whitelist oficial concedida bajo el nombre canónico registrado `The Multipurpose Companion (TMC)`.
- **Frontier CAPI:** Autenticación oficial OAuth2 PKCE para sincronización de flota remota y naves en astilleros.
- **EDSM y EDDN:** Persistencia de credenciales segura y retransmisión comunitaria de eventos astronómicos.

### 🎙️ 8. Copiloto Auditivo (Síntesis de Voz) y VoiceAttack
- **Voz Nativa del Sistema Operativo:** Motor Windows SAPI5/OneCore ultraligero integrado de fábrica (< 2 MB de memoria, 0 dependencias externas).
- **Voz Neuronal KokoroTTS (Opcional):** Inferencia local de alta fidelidad para mayor inmersión.
- **Perfil de VoiceAttack:** Descarga directa de `TMC-Copiloto.vap` desde los Ajustes para control por voz total de tu nave.

### 📱 9. Pantalla Remota LAN (Tablets y Móviles)
- Escanea el **código QR de emparejamiento** desde tu tablet (iPad, Android) o smartphone conectado a la misma red local para utilizar TMC como una botonera y pantalla táctil de cabina independiente.

---

## 📥 Instalación y Primer Uso

### Descarga para Windows
1. Descarga el instalador oficial desde [Releases](https://github.com/Danikuso/TheMultipurposeCompanion/releases):
   - **`TMC_0.1.0_x64-setup.exe`** (Instalador recomendado)
   - O descarga el paquete portable `.zip` si prefieres no instalar nada.
2. Ejecuta el instalador.

> [!NOTE]
> **Aviso de Microsoft Defender SmartScreen:**  
> Al ser un proyecto independiente y de código abierto sin un certificado digital de pago corporativo de Microsoft, Windows mostrará la advertencia azul *"Windows protegió su PC / Editor desconocido"*.  
> Para continuar:
> 1. Haz clic en **«Más información»**.
> 2. Haz clic en **«Ejecutar de todas formas»**.

3. Abre **TMC**: la aplicación detectará automáticamente tu instalación de *Elite Dangerous* y tus archivos de diario (`Saved Games\Frontier Developments\Elite Dangerous`). ¡No necesitas configurar rutas manualmente!

---

## 📖 Manual de Usuario y Documentación

- **Manual en PDF:** Descarga la guía completa para pilotos lista para imprimir o leer offline: [`Manual_TMC_Comandante.pdf`](website/Manual_TMC_Comandante.pdf).
- **Web Oficial:** Explora la documentación interactiva en [`website/index.html`](website/index.html).

---

## 🛠️ Compilación para Desarrolladores

Si prefieres compilar TMC desde el código fuente:

### Requisitos
- **Rust** estable (edición 2024, 1.90+) con herramientas de compilación de C++ (MSVC en Windows).
- **Node.js** 22+ y `npm`.
- **Tauri CLI 2.x** (`cargo install tauri-cli@^2`).

### Comandos de Compilación
```powershell
# 1. Instalar dependencias y compilar la interfaz de usuario
npm.cmd --prefix ui ci
npm.cmd --prefix ui run build

# 2. Ejecutar la suite completa de pruebas
cargo test --workspace
npm.cmd --prefix ui run test

# 3. Compilar el ejecutable Release
cargo build -p tmc-desktop --release

# 4. Generar el instalador NSIS
Push-Location apps/desktop
cargo tauri build
Pop-Location
# Salida: target/release/bundle/nsis/TMC_0.1.0_x64-setup.exe
```

---

## 🔒 Privacidad y Licencia

- **Licencia:** **PolyForm Noncommercial 1.0.0** (Uso personal no comercial). Consulta el archivo [`LICENSE`](LICENSE) para más detalles.
- **Privacidad:** TMC nunca recopila ni transmite datos personales reales ni chats del comandante. Tus credenciales de Inara y EDSM se almacenan cifradas en SQLite local y los tokens de Frontier CAPI usan el protocolo seguro OAuth2 PKCE. La galaxia Legacy está 100 % aislada.
- **Aviso de Marca:** *Elite Dangerous* es una marca registrada de Frontier Developments plc. TMC es un proyecto complementario no oficial desarrollado bajo las directrices comunitarias.
