# TMC — The Multipurpose Companion

<p align="center">
  <a href="README.md"><strong>English</strong></a> ·
  <a href="README.es.md"><strong>Español</strong></a> ·
  <a href="README.fr.md"><strong>Français</strong></a> ·
  <a href="README.de.md"><strong>Deutsch</strong></a> ·
  <a href="README.pt.md"><strong>Português</strong></a>
</p>

[![Versión](https://img.shields.io/badge/versi%C3%B3n-0.1.0-orange.svg)](https://github.com/Danikuso/TheMultipurposeCompanion/releases)
[![Licencia: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/licencia-PolyForm%20Noncommercial%201.0.0-blue.svg)](LICENSE)
[![Plataforma](https://img.shields.io/badge/plataforma-Windows%2010%2F11%20%7C%20Linux%20%2F%20Steam%20Deck-green.svg)]()
[![Inara Whitelist](https://img.shields.io/badge/Inara%20InAPI%20v1-Whitelist%20Oficial-success.svg)]()

**TMC (*The Multipurpose Companion*)** es un ayudante de cabina gratuito y de código abierto en tiempo real para ***Elite Dangerous*** (*Live 4.0 / Odyssey / Horizons* y *Legacy 3.8*), construido con **Rust**, **Tauri 2** y **Svelte 5**.

Su objetivo es servir de copiloto integral tanto para pilotos que acaban de empezar como para comandantes veteranos: reúne en una única aplicación ultraligera **10 disposiciones de cabina combinadas**, **16 módulos especializados**, **planificación de rutas**, **guía paso a paso de ingenieros**, **copiloto auditivo y consultas por voz** y **pantalla táctil auxiliar para tablet o móvil**.

---

## Índice de este Documento

- **Recursos para Jugadores:** [Manual de Instrucciones y Tutoriales (PDF)](website/Manual_TMC_Comandante.pdf) · [Web de Presentación (`website/`)](website/index.html)
- **[Parte I — Guía para Jugadores y Usuarios](#parte-i--guía-para-jugadores-y-usuarios)**
  1. [Filosofía y Cualidades Clave](#1-filosofía-y-cualidades-clave)
  2. [Instalación y Puesta en Marcha (Windows y Linux / Steam Deck)](#2-instalación-y-puesta-en-marcha)
  3. [Cómo Funciona la Interfaz (`TopBar`, `Auto-companion` y las 10 Disposiciones)](#3-cómo-funciona-la-interfaz)
  4. [Todas las Funcionalidades: Los 16 Módulos al Detalle](#4-todas-las-funcionalidades-los-16-módulos-al-detalle)
  5. [Copiloto de Voz, Consultas por Micrófono, VoiceAttack y Tablet / Móvil (QR)](#5-voz-micrófono-voiceattack-y-pantalla-en-tablet-o-móvil)
  6. [Personalización en Ajustes y Soporte / Feedback](#6-personalización-en-ajustes-y-soporte)
- **[Parte II — Documentación Técnica para Desarrolladores](#parte-ii--documentación-técnica-para-desarrolladores)**
  7. [Arquitectura del Workspace (12 Crates + 2 Apps + Frontend)](#7-arquitectura-del-workspace)
  8. [Ingesta del Diario, Almacenamiento, Seguridad y Rendimiento](#8-ingesta-del-diario-almacenamiento-seguridad-y-rendimiento)
  9. [Rutas Locales, Argumentos CLI y Variables de Entorno](#9-rutas-locales-argumentos-cli-y-variables-de-entorno)
  10. [Herramientas de Desarrollo (`tmc-replay`, `tmc-headless eval`, Footprint)](#10-herramientas-de-desarrollo-reproducción-y-auditoría)
  11. [Compilación, Pruebas Automatizadas y Empaquetado NSIS](#11-compilación-pruebas-automatizadas-y-empaquetado)
  12. [Contribución, Privacidad y Licencia](#12-contribución-privacidad-y-licencia)

---

# Parte I — Guía para Jugadores y Usuarios

## 1. Filosofía y Cualidades Clave

- **Un guía que nunca te deja bloqueado:** si quieres mejorar un módulo de tu nave pero todavía no conoces o no has desbloqueado al ingeniero necesario, TMC no te muestra un aviso vacío: despliega primero **paso a paso cómo descubrirlo, cómo conseguir su invitación y qué tarea o entrega pide su taller** (leyendo tu avance real del diario), y justo debajo calcula los materiales y trueques necesarios.
- **Claridad visual («TMC Glass HUD») y color sincronizado con tu nave:** cada panel muestra en su parte superior la información esencial con medidores visuales, semáforos y botones de un clic, dejando las tablas extensas plegadas bajo **`Ver información adicional`**. Además, el tema **`HUD automático`** detecta los colores personalizados de la cabina de tu juego para adoptar tu misma paleta visual.
- **Ligero y silencioso en segundo plano:** diseñado para no restar fotogramas ni recursos al juego. Al cerrar la ventana (`✕`), TMC permanece activo junto al reloj del sistema liberando su memoria gráfica mientras mantiene activos los avisos por voz y el copiado automático de saltos.
- **100 % respetuoso con tus datos y con la galaxia Legacy:** funciona localmente desde el primer segundo sin cuentas obligatorias. Si detecta que juegas en la galaxia **Legacy 3.8**, bloquea de forma automática cualquier envío a portales externos (EDDN, EDSM, Inara) para no mezclar ambas partidas.
- **5 idiomas completos:** interfaz, datos de ingeniería y voces disponibles en **Español**, **Inglés**, **Francés**, **Alemán** y **Portugués**.

---

## 2. Instalación y Puesta en Marcha

### Windows 10 / 11 (64 bits)

1. Descarga el instalador **`TMC_0.1.0_x64-setup.exe`** desde la sección [Releases](https://github.com/Danikuso/TheMultipurposeCompanion/releases) (asistente disponible en los 5 idiomas).
2. Ejecuta el instalador y completa los pasos.

> [!NOTE]
> **Aviso preventivo de Windows Defender SmartScreen:**  
> Al ser un proyecto gratuito de código abierto sin certificado corporativo comercial de pago, la primera vez Windows puede mostrar la pantalla azul *«Windows protegió su PC»*. Pulsa en **«Más información»** y a continuación en **«Ejecutar de todas formas»**.

3. Abre **TMC** antes o después de iniciar *Elite Dangerous*. Detectará automáticamente tu carpeta de partida guardada, importará tu historial en segundo plano (`Importando historial: X de Y`) y recordará el tamaño y posición de su ventana entre sesiones.
4. **Bandeja del sistema:** al pulsar `✕` en la ventana, TMC sigue funcionando en el icono junto al reloj de Windows. Haz clic izquierdo en el icono (o clic derecho → **`Abrir TMC`**) para restaurar la ventana, o elige **`Salir`** para cerrarlo por completo.

### Linux y SteamOS (Steam Deck)

- TMC detecta automáticamente la carpeta de diarios de *Elite Dangerous* dentro de los prefijos **Proton** de Steam (tanto en instalaciones nativas como en Flatpak, AppID `359320`).
- Puedes ejecutar la aplicación de escritorio (`tmc-desktop`) o lanzar el servidor ligero sin ventana (`tmc-headless`) para abrir la interfaz desde el navegador o desde una tablet en la misma red Wi-Fi.

---

## 3. Cómo Funciona la Interfaz

### 3.1 Barra Superior (`TopBar`)
Siempre visible en la parte alta de la pantalla:
- **Estado y galaxia:** indicador luminoso de conexión en vivo, insignia **`LIVE 4.0`** o **`LEGACY`** y avisos contextuales (`El juego no está en ejecución` o `Galaxia Legacy: los datos permanecen locales`).
- **6 indicadores instantáneos:** **`Sistema`** actual, **`MODO`** (entre 10 estados: *Atracado, Aterrizado, Espacio normal, Supercrucero, Hiperespacio, A pie, En vehículo de superficie, En caza, En taxi, Multitripulación*), **`Combustible`** (con aviso visual por debajo del 25 %), **`Casco`** (con alerta roja por debajo del 30 %), **`Carga`** (`usada / capacidad`) y **`Créditos`**.
- **Controles rápidos a la derecha:** botón de candado táctico **`🔓` / `🔒`** (con atajo **`Alt + L`** para congelar la pantalla de inmediato), botón de micrófono **`🎙️`** (para consultas habladas) e interruptor **`Auto-companion`**.

### 3.2 Modo Auto-Companion Pro: Inteligencia Predictiva, Candado Táctico y Prioridades
- **Conmutación inteligente y continua:** Con **`Auto-companion` activado**, TMC cambia automáticamente a la disposición más relevante según los acontecimientos en la cabina (combate, prospección de asteroides, atraque en estación, ingeniería, salto de Fleet Carrier, colonización o salto hiperespacial).
- **Bloqueo Táctico de 1 Clic (`Alt + L` / Candado):** En cualquier instante crítico o de consulta intensiva, pulsa el botón del candado en la barra superior o presiona **`Alt + L`** para congelar la pantalla actual. El candado brillará en tono ámbar *Glass HUD* impidiendo que ningún evento automático mueva tu vista hasta que decidas desbloquearlo.
- **Histéresis y Retención Anti-Salto (`linger_seconds`):** Tras salir de un combate, terminar una prospección o cerrar una transacción, la pantalla no cambia bruscamente. Mantiene la cabina activa durante el margen de retención configurado (por defecto 20 segundos, ajustable de 0 a 60 s en Ajustes) para permitirte revisar el botín, daños y estadísticas con calma.
- **Transiciones Predictivas:** TMC se anticipa a los eventos antes de que ocurran:
  - *Interdicción inminente:* si sufres una interdicción o inicias una interdicción en supercrucero, la pantalla conmuta inmediatamente a *Combate* durante la lucha por el vector de escape.
  - *Aproximación a anillos:* al descender hacia anillos o cinturones planetarios portando equipamiento de minería, conmuta proactivamente a *Minería*.
  - *Autorización de atraque:* al recibir autorización de atraque en una estación o puerto espacial, conmuta a *Comercio* preparando el mercado y servicios.
- **Prioridades de Cabina a tu Medida:** En `Ajustes` puedes personalizar el orden de jerarquía de los 9 roles usando sencillas flechas `▲` y `▼`, o pulsar `Restablecer orden estándar` para volver a la configuración táctica recomendada.
- **Feedback Sensorial de Cabina:** Activa el sutil bip acústico de cabina (*Glass HUD Chime*, sintetizado en tiempo real a 880 Hz ➔ 1320 Hz sin archivos pesados) o los avisos de voz del copiloto (TTS) para saber cuándo ha cambiado la pantalla sin apartar la vista de tus instrumentos de vuelo.
- Dentro de la vista de **`Exploración`**, el **`Panel de contexto`** alterna automáticamente entre **`Cuerpos`** y **`Exobiología`** cuando desciendes a la superficie de un mundo con señales biológicas.
- Si seleccionas manualmente otra vista mientras `Auto-companion` está activo (sin el candado puesto), TMC respeta tu elección temporal hasta que cambie la actividad en el juego. Si desactivas `Auto-companion`, la pantalla permanece fija donde la dejes.

### 3.3 Las 10 Disposiciones de Cabina frente a los 16 Módulos
En el menú lateral izquierdo dispones de dos formas de navegar (además del botón **`⚙️ Ajustes`** fijo abajo):

| Disposición combinada | Paneles que reúne en una sola pantalla |
|---|---|
| **1. `Exploración`** | Tira de `Ruta`, `Panel de contexto` (conmutador `Cuerpos` / `Exobiología`), `Planificador de rutas` y `Alcance de salto`. |
| **2. `Nave`** | `Nave` (equipamiento e ingeniería), `Alcance de salto`, `Mi carga` y `Astilleros y Equipamiento`. |
| **3. `Combate`** | `Objetivo`, `Sesión de caza`, `Mi nave` y paneles automáticos de `Zona de conflicto` y `AX` (Thargoides) cuando están activos. |
| **4. `Minería`** | `Objetivos de minería` (bucle triangular y venta óptima), `Prospector`, `Sesión de minería` y `Dónde vender`. |
| **5. `Comercio`** | `Mercado actual`, `Mi carga`, `Sesión de comercio`, `Ruta comercial` y `Buscar mercancía`. |
| **6. `Misiones`** | `Misiones activas`, `Avisos`, `Recompensas de la sesión` y paneles automáticos de `Pila de masacre`, `Pasajeros`, `Turismo` y `Objetivos comunitarios`. |
| **7. `Ingeniería`** | `Materiales`, `Ingenieros`, `Avisos de ingeniería`, `Mi nave modificada`, `Ingenierizar un módulo` (al elegir pieza), `Lista de la compra`, `Intercambios` y `Dónde conseguirlo`. |
| **8. `Fleet Carrier`** | `Estado del Fleet Carrier`, `Secuencia de salto` y `Finanzas y mantenimiento`. |
| **9. `Colonización`** | `Depósito de construcción` y `Suministro y compras`. |
| **10. `Powerplay 2.0`** | `Estado de Powerplay` y `Facciones del sistema` (BGS). |

Debajo de las disposiciones encontrarás la lista de los **16 Módulos individuales** (con un punto luminoso que indica cuáles tienen actividad relevante en ese momento) para abrir cualquiera de ellos de forma dedicada.

---

## 4. Todas las Funcionalidades: Los 16 Módulos al Detalle

### 1. `Panel general` (7 paneles)
- **`Comandante`:** nombre, saldo de créditos y barras de progreso con porcentaje en los 6 rangos de la *Federación de Pilotos* (*Combate, Comercio, Exploración, Soldado, Exobiólogo, CQC*) y los 2 de *Superpotencias* (*Federación, Imperio*).
- **`Nave` y `Ubicación`:** resumen de tu nave activa, sistema estelar, cuerpo celeste más cercano, estación y modo actual.
- **`Estado en vivo`:** 7 testigos instantáneos de cabina (*Escudos, Anclajes de armas, Tren de aterrizaje, Bloqueo de masa, Navegación silenciosa, Recolector de carga* y *Sobrecarga de supercrucero SCO*) junto a las barras de energía `SYS`, `ENG` y `WEP` (0 a 4 pips).
- **`Ruta` y `Sesión`:** siguiente sistema con botón **`Copiar siguiente sistema`**, saltos pendientes, duración de la sesión, saltos completados, años luz recorridos y créditos ganados.
- **`Alertas`:** avisos inmediatos ante *Combustible bajo, Sobrecalentamiento, Interdicción en curso, Bajo ataque, Daño en el casco, Oxígeno bajo, Salud baja* o fallo de un módulo.

### 2. `Nave` (2 paneles: Auditoría técnica y compra de Naves/Equipamiento)
- **Cabecera y medidores (`Nave`):** modelo canónico, nombre, matrícula, masa en vacío, coste de recompra (*rebuy*) y medidores de casco, escudos, depósito principal + reserva y carga.
- **3 pestañas de revisión rápida antes de despegar:**
  - **`🧭 Exploración`:** alcance máximo y lista de comprobación (`✅` / `⚪`) para recolector de combustible, potenciador FSD Guardián, escáner de superficie DSS, hangar SRV y unidad de reparación AFMU.
  - **`⚔️ Combate`:** escudos, calor, reparto `SYS/ENG/WPN`, armas instaladas con munición `[cargador / reserva]` y recuento de utilidades defensivas (*potenciadores de escudo, señuelos, defensas de punto y disipadores térmicos*).
  - **`📦 Carga y Minería`:** capacidad total, bodegas instaladas, refinería, controladores de drones y mercancías a bordo.
- **`Desglose módulo por módulo`:** filtrable por categoría (*Todos, Anclajes, Primarios, Opcionales, Utilidades*) y por texto. Muestra clase `[5A]`, integridad `%`, encendido `ON/OFF`, prioridad `P1..P5` e insignia dorada `G1..G5 ★` que, al pulsarla, despliega la receta, el ingeniero, el efecto experimental y la **comparativa exacta de cada atributo (valor mejorado frente a valor original)**.
- **`Astilleros y Equipamiento` (Buscador de compra en parada única):**
  - **Cesta combinada desde tu ubicación actual:** parte automáticamente de tu sistema actual (incluso en espacio profundo no indexado mediante coordenadas 3D) y permite elegir **1 nave opcional** (48 modelos) y/o **hasta 10 componentes** (153 familias con filtro de categoría, clase `0..8`, calificación `A..I`, montaje *Fijo/Cardán/Torreta* en armas y casco en blindajes) para encontrar las estaciones más cercanas que venden **toda la cesta a la vez**.
  - **Respaldo inteligente por pieza:** si ninguna estación cercana reúne toda la combinación simultáneamente, localiza automáticamente la estación más cercana para cada artículo por separado e identifica qué pieza carece de stock conocido.
  - **Interruptores y salvaguardas de ayudante:** filtro **`−15 % Li Yong-Rui`** (que reduce también tu futuro seguro de recompra), interruptor dedicado **`Stronghold de tu potencia`** para incluir los *Stronghold Carriers* orbitales de tu facción de Powerplay 2.0 (manteniendo excluidos los de potencias rivales y los portaaviones privados salvo que los actives aparte), protección de sistemas con permiso (`Shinrarta Dezhra`, `Sol`, `Achenar`, `Sirius`, `Alioth`…), **semáforo de seguro de recompra** (`Recompra OK` / `Ajusta recompra` / `Saldo insuf.`), desglose individual de precios y botones de un clic **`Copiar`** y **`Ruta`**.

### 3. `Navegación` (3 paneles y doble planificador Spansh)
- **Tira `Ruta`:** muestra tu plan activo de Spansh o, si no hay ninguno, la ruta trazada en el mapa galáctico del juego. Si te desvías, aparece el aviso **`Fuera de ruta`** con botones **`Recalcular`** y **`Borrar plan`**. Detalla en 4 columnas el salto (distancia, clases de estrellas, si **`Se puede repostar`**, combustible restante estimado), exploración (descubierto/no descubierto, visitas previas, neutrones/enana blanca), política (lealtad, gobierno, facción y **`⚠ Estado de peligro`** en guerras o brotes) y economía/seguridad/Powerplay.
- **`Alcance de salto`:** salto actual (con indicador **`Supercargado ×N`** por cono de neutrones o inyección), salto con depósito lleno, combustible actual y consumo máximo por salto.
- **`Planificador de rutas`:** autocompletado de sistemas, salida automática por coordenadas 3D incluso desde sistemas lejanos aún no registrados en bases de datos públicas, y 2 modos:
  - **`⚡ Autopista de neutrones`:** cálculo por estrellas de neutrones con porcentaje de **`Eficiencia (%)`** configurable.
  - **`🗺️ Ruta exacta (Exact Plotter)`:** lee automáticamente el peso, depósitos, carga, motor FSD mejorado y potenciador Guardián de tu nave actual (sin exportar nada a webs externas), con opciones **`Repostar en estrellas escoop`** (marcando paradas `⛽ Sí`) y **`Usar inyecciones FSD`**. Al pulsar **`Aceptar`**, guarda la ruta y copia automáticamente cada próximo destino al portapapeles salto tras salto.

### 4. `Exploración` (`Cuerpos del sistema`)
- **Resumen superior:** estado descubierto/no descubierto del sistema, todas las clases de estrellas presentes (destacando en verde las repostables *O, B, A, F, G, K, M*), progreso de **`Escaneo FSS`**, tarjetas comparativas **`Valor del sistema`** frente a **`Si mapeas los recomendados`**, cuerpos pendientes de identificar y tarjeta **`Datos sin vender`** con alerta roja **`Datos en riesgo`** si tu nave ha sufrido daños.
- **Tabla detallada desplegable:** 27 clases estelares y planetarias, distancia en segundos luz, valor de escaneo y de mapeo, señales `Bio` y `Geo`, marcas (*Terraformable, Aterrizable, Recogible, 1.º descubrimiento, Ya mapeado*) y columna **`Consejo`** (`★ Mapear` según tu umbral en créditos, `Aterrizar (exobiología)` o `No compensa`, junto con la meta de eficiencia en número de sondas DSS).

### 5. `Exobiología` (Asistente de muestreo en superficie para Odyssey)
- **Cabecera del mundo:** gravedad, temperatura, tipo de atmósfera, número de señales biológicas e insignia **`SIN REGISTRAR · ×5 (estimado)`** cuando nadie ha registrado antes ese planeta.
- **Guía de `Muestreo en curso` (`1/3 → 2/3 → 3/3`):** barra y contador en vivo de **`Distancia a la muestra anterior`** comparada con la distancia mínima clonal que exige esa especie (`Te faltan X m`), **indicación direccional relativa y rumbo cardinal** según hacia dónde miras (p. ej. *«la anterior está detrás a tu izquierda (SO)»*), alerta si descartas una muestra a medias y tarjeta de valor **`Sin vender en la nave`** con proyección de ganancias al terminar la especie actual.
- **Tabla `Posibles especies (ordenadas por valor)`:** combina condiciones astrofísicas, tu historial y datos de *Canonn* mostrando pago base, pago `Con ×5`, distancia mínima, probabilidad y géneros confirmados por las sondas DSS.

### 6. `Combate` (5 paneles tácticos y Anti-Xeno)
- **`Objetivo`:** nave seleccionada con emblema, piloto, rango, facción, aviso de alta visibilidad **`LIMPIO · NO DISPARAR`** (verde) o **`BUSCADO · recompensa`** (rojo), barras de escudos y casco, y salud porcentual del **`Subsistema`** apuntado (*Planta de energía, Propulsores, Motor FSD, Armas…*), manteniendo los datos 10 segundos tras perder puntería.
- **`Mi nave`:** estado de escudos, barra de casco en 10 bloques, estado térmico con cronómetro en caso de sobrecalentamiento, medidor visual de pips (`SIS`, `MOT`, `ARM`) y alertas activas.
- **`Sesión de caza`:** bajas, créditos ganados, ritmo por hora, avance del rango de combate, botón **`Reiniciar sesión`**, desglose de **`Vales sin cobrar`** por facción (calculando también el importe neto con **`−25 % en IF`**) y localización del *Interstellar Factors* más cercano.
- **`Zona de conflicto` y `AX` (automáticos):** bando elegido, bajas y bonos en zonas de guerra; alerta de presencia Thargoide, bonos AX y conteo de bajas por variante (*Explorador, Cíclope, Basilisco, Medusa, Hidra, Guadaña, Glaive, Orthrus…*).

### 7. `Minería` (4 paneles: Localizador de Objetivos, Prospector, Sesión y Dónde vender)
- **`Objetivos de minería y venta óptima (Miner's Companion)`:**
  - **Bucle triangular más corto:** calcula en cabina el circuito óptimo completo: desde tu sistema actual ➔ al mejor yacimiento o anillo ➔ a la estación más cercana con el mayor precio de venta para tus minerales.
  - **3 perfiles especializados y chips de preset rápido:**
    - **Láser:** enfocado en platino en anillos metálicos, painite y osmio. Incluye combinaciones con sitios de extracción *Hazardous RES* (+100 % de fragmentos por roca) y *High RES* (+75 % de fragmentos) y solapamientos (*overlaps*) comprobados.
    - **Núcleo profundo (*Core*):** enfocado en musgravita, monazita, alejandrita, ópalos del vacío y grandidierita en anillos rocosos e inertes.
    - **Misiones e Ingenieros:** bromellita (para el desbloqueo del ingeniero Bill Turner en Alioth), tritio (para repostar *Fleet Carriers*) y osmio.
  - **Alerta de riesgo de penalización de demanda (*Demand Tax Risk*):** la economía de *Elite Dangerous* reduce drásticamente el precio pagado si la carga a bordo supera el 25 % de la demanda de la estación de destino. TMC advierte de forma preventiva con semáforo ámbar/rojo en cabina y muestra el umbral de seguridad para no perder créditos.
  - **Venta de bodega actual en 1 clic (*Multi-Sell*):** lee los minerales y metales reales almacenados en tu nave y localiza la estación óptima que compra toda la combinación con el mayor beneficio conjunto.
  - **Catálogo curado offline y sincronización atómica online:** integra de serie más de 40 yacimientos de alta densidad comprobados por la comunidad (*EDTools/Spansh*), funcionando de forma inmediata sin conexión a internet y actualizándose en caliente en segundo plano cuando hay conexión disponible.
  - **Acción directa de cabina:** botones **`📋 Copiar sistema`** y **`🚀 Trazar ruta`** directos en cada tarjeta y en la tabla adaptativa con desplazamiento horizontal fijo.
- **`Prospector`:** nivel de contenido (*bajo/medio/alto*), aviso destacado **`◆ Núcleo: mineral`** para detonación profunda, barra multicolor de composición, lista de minerales resaltando tus favoritos con `★`, veredicto automático **`✔ merece la pena`** o **`✘ no compensa`** según tu umbral en créditos y gráfico comparativo de los **`Últimos 8 asteroides`**.
- **`Sesión de minería`:** duración, toneladas refinadas y ritmo `t/h`, ganancias estimadas y `cr/h`, drones **`Limpets`** restantes y lanzados por tipo, llenado de bodega, desglose por mineral y estimación de las tolvas de **`Refinería`**.
- **`Dónde vender`:** mejor estación global recomendada para toda tu carga y opciones desplegables por cada mineral con estación, sistema (con botón copiar `⎘`), distancia, precio, demanda, tamaño de pista (`S/M/L` con aviso si falta confirmación) y antigüedad del precio.

### 8. `Comercio` (5 paneles)
- **`Mercado actual`:** destaca las mejores oportunidades de las últimas 48 h y despliega el catálogo completo identificando mercancías **`RARA`** (con su distancia óptima de venta), precios de compra, oferta, mejor estación compradora cercana, plataforma y beneficio por tonelada.
- **`Mi carga`:** ocupación de bodega y seguimiento por lote del **`Precio pagado`** medio, etiquetas de *carga de misión* o *carga robada*, comparativa entre vender en la estación actual o en el mejor mercado cercano y consejo directo **`VENDER AQUÍ`** o **`LLEVAR`**.
- **`Ruta comercial`:** optimizador multiparada desde la estación donde estás atracado, detallando qué comprar y vender en cada escala, saltos, beneficio total y copiado automático del próximo destino.
- **`Sesión de comercio` y `Buscar mercancía`:** balance de beneficio neto, `cr/h`, toneladas movidas, mejor operación, avance de rango comercial y buscador con autocompletado para localizar dónde comprar cualquier producto cerca de tu sistema.

### 9. `Misiones` (7 paneles: Contratos, Masacres, Pasajeros, Turismo y CG)
- **`Misiones activas` y `Avisos`:** contratos agrupados por destino marcando `▶ Siguiente parada` con botón **`A la ruta`**, tipo, facción, progreso, cuenta atrás de caducidad (en rojo si queda `< 1 h`) y alertas por desvío de destino o falta de mercancía de misión en bodega.
- **`Pila de masacre`:** agrupa encargos contra una misma banda pirata en barras paralelas por facción emisora (donde cada baja cuenta a la vez en todas las facciones distintas), calculando cuántas bajas reales faltan para cerrar toda la pila y la recompensa total.
- **`Pasajeros`:** plazas ocupadas por clase de cabina (*Económica, Business, Primera, Lujo*), grupos **`VIP`** / **`BUSCADO`** y alerta roja crítica **`⚠ Pasajero buscado a bordo: riesgo al ser escaneado en zonas de seguridad`**.
- **`Turismo`:** ordena automáticamente las balizas turísticas en la **`Ruta óptima`** más corta frente al orden del tablón (mostrando años luz y saltos ahorrados), calcula `cr/al` y `cr/h` respecto a tu media histórica o referencia y emite veredicto **`✔ RENTABLE`** o **`✖ NO COMPENSA`**.
- **`Recompensas de la sesión` y `Objetivos comunitarios`:** resumen de créditos, materiales y cambios de reputación/influencia por facción, más seguimiento en vivo de *Community Goals* (*aportación, nivel, percentil, tiempo restante y recompensa prevista*).

### 10. `Ingeniería` (8 paneles para nave y trajes/armas a pie de Odyssey)
- **`Materiales`:** 4 pestañas (*Materias primas, Fabricados, Codificados* y *Taquilla* de Odyssey dividida en *Artículos, Componentes, Consumibles y Datos*), buscador instantáneo, historial de **`Cambios de la sesión`** por origen (*Misión, Escaneo, Minería, Intercambio, Ingeniería…*) y aviso **`MÁXIMO`** en depósitos llenos.
- **`Ingenieros` y `Mi nave modificada`:** catálogo filtrable (*Todos, Desbloqueados, Pendientes*) de **ingenieros de nave e ingenieros a pie de Odyssey** con rango `★`, distancia y progreso del siguiente requisito; selector rápido de cualquier módulo ingenierizable instalado en tu nave.
- **`Ingenierizar un módulo`:** elige mejora, efecto experimental, grado `G1..G5` y tiradas por grado. Si aún no tienes acceso a ningún ingeniero capaz de aplicar ese grado, muestra primero la **`GUÍA DE AYUDANTE — DESBLOQUEO DEL INGENIERO`** con los 3 pasos secuenciales (*1. Descubrimiento → 2. Invitación → 3. Tarea de Desbloqueo*), sistema con copia rápida y progreso actual, y debajo el coste por grado y la tabla semáforo de materiales (`OK`, **`Cubierto con intercambio`** o `Falta`).
- **`Lista de la compra`, `Intercambios`, `Dónde conseguirlo` y `Avisos`:** guarda varios proyectos, ordénalos por prioridad (**`▲ Subir` / `▼ Bajar`**) para reservar materiales, obtén las conversiones exactas con tus excedentes sin reservar (`Das X → Recibes Y`) y localiza el **Comerciante de materiales** o **Broker de tecnología** (*Humano* y *Guardián*) más cercano junto con guías de recolección.

### 11. `Fleet Carrier` (3 paneles)
- **`Estado del Fleet Carrier`:** matrícula, nombre, insignia `A BORDO`, sistema actual, saltos posibles (500 al/salto), permisos de atraque, nivel de **`Tritio`** combinando depósito (`1.000 t`) + reservas en bodega y reparto de capacidad libre/ocupada.
- **`Secuencia de salto`:** estado operativo y cronómetro `MM:SS` en vivo para la cuenta atrás de salto (15 min) y el enfriamiento posterior (5 min), con sistema destino y botón **`Copiar destino`**.
- **`Finanzas y mantenimiento`:** semáforo financiero (*Saludable, Advertencia, Crítico*), **`Semanas cubiertas`**, saldo del Carrier, mantenimiento semanal, reserva y fecha del próximo cobro.

### 12. `Colonización` (2 paneles)
- **`Depósito de construcción`:** avance porcentual global de la obra, toneladas aportadas y pendientes, cálculo dinámico de **`Viajes estimados`** según las toneladas que caben en la bodega de tu nave actual y tabla desglosada recurso por recurso.
- **`Suministro y compras`:** tarjetas de materiales pendientes ordenadas por urgencia con toneladas, viajes restantes y botón **`Copiar`** para abastecerte rápidamente.

### 13. `Powerplay 2.0` y 14. `BGS`
- **`Estado de Powerplay`:** emblema de potencia y superpotencia, semanas de lealtad, rango actual (`X / 100`), méritos acumulados y restantes para el siguiente nivel, méritos de la sesión, **`Tasa de méritos/hora`** y cuenta atrás al cierre semanal (*Jueves 07:00 UTC*).
- **`Facciones del sistema` (BGS):** facción controladora, estado activo, monitor del **`Tick galáctico`** diario (`Hace X h`, indicando si es reciente o desactualizado) y tabla de todas las facciones locales con barras de influencia, estados, lealtad y gobierno.

### 15. `Crimen y Notoriedad` (5 tarjetas tácticas)
- **`Estado Legal y Notoriedad`:** medidor de 0 a 10 con temporizador de enfriamiento (2 horas de vuelo activo por punto) que detecta y muestra **`⏸️ Decaimiento pausado (atracado en estación)`** cuando estás en puerto.
- **`Guía de Limpieza de Nombre`:** avisa si los contactos neutrales están bloqueados por notoriedad o, cuando estás a 0, muestra directamente el **`Interstellar Factor más cercano`** con distancia, plataforma y botón **`Copiar sistema`**.
- **`Contrabando y Mercados Negros`:** lista mercancías robadas en bodega, localiza el **`Mercado Negro más cercano`** e incluye una guía en 3 pasos de **`Consejos de Aproximación Sigilosa`** (alineación a 8–10 km, navegación silenciosa / disipador a `< 4 km` y entrada ágil).
- **`Desglose de Facciones y Multas` y `Seguridad del Sistema`:** multas y recompensas por jurisdicción + nivel de seguridad local con tiempo estimado de respuesta policial.

### 16. `Cuentas` (Sincronización comunitaria opcional)
- **`Elite Dangerous Data Network (EDDN)`:** comparte precios, astilleros y saltos de forma anónima con la red comunitaria y recibe actualizaciones de mercados cercanos en tu radio configurado.
- **`Elite Dangerous Star Map (EDSM)`:** guarda tu clave API de forma persistente para sincronizar cuaderno de bitácora, rangos y créditos.
- **`Inara`:** aplicación aprobada oficialmente en la **Whitelist de Inara** bajo el nombre **`The Multipurpose Companion (TMC)`**, con agrupación inteligente por lotes (máx. 2 envíos cada 60 s) y botón de prueba de conexión.
- **`Frontier Companion API (CAPI)`:** inicio de sesión directo y seguro en la web oficial de Frontier (con opción desplegable para pegar la URL si tu navegador bloquea el retorno automático) y visor interactivo de **`Flota remota almacenada`** con todas tus naves aparcadas en otros sistemas, su estación, valor y distancia en años luz.

---

## 5. Voz, Micrófono, VoiceAttack y Pantalla en Tablet o Móvil

### 5.1 Copiloto Auditivo (Avisos hablados / TTS)
En `Ajustes` → **Copiloto de voz y alertas de cabina** puedes elegir entre:
- **`Nativo del sistema operativo (Ultraligero)`:** utiliza al instante las voces instaladas en tu sistema sin descargas adicionales.
- **`Motor neuronal KokoroTTS (Voz IA cinematográfica)`:** 10 voces naturales locales en varios idiomas con suspensión automática en reposo para no ocupar memoria (y respaldo automático a la voz del sistema si no está instalado).
- Incluye control de **volumen**, **velocidad**, botón **`🔊 Probar voz de copiloto`** y **5 canales independientes** (*Combate y alertas críticas*, *Navegación y repostaje estelar*, *Exploración y exobiología*, *Minería y prospección* y *Fleet Carrier*).

### 5.2 Asistente por Micrófono (9 consultas habladas en 5 idiomas)
En `Ajustes` → **Asistencia por voz e integración** puedes activar el reconocimiento mediante **`Whisper AI Local`** (~39 MB, 100 % offline y privado, descargable con un clic desde el propio panel) o el motor del sistema, en **Escucha continua** o **Pulsar para hablar** (botón `🎙️` de la barra superior). Puedes preguntarle con voz natural por:
1. **Estado general de la nave** (casco, escudos y combustible).
2. **Nivel de combustible** (porcentaje y toneladas).
3. **Siguiente salto estelar** (dice cuál es tu próximo destino).
4. **Copiar siguiente sistema** (lo anuncia y lo **copia al portapapeles** listo para pegar en el mapa galáctico).
5. **Señales biológicas en el sistema** (recuento de bioseñales detectadas).
6. **Resumen de materiales fijados** (avance de tu lista de ingeniería).
7. **Estado legal y multas** (notoriedad, multas y recompensas).
8. **Factor interestelar más cercano** (dice estación, sistema y distancia, y **copia el sistema al portapapeles**).
9. **Mercado negro más cercano** (dice estación, sistema y distancia, y **copia el sistema al portapapeles**).

### 5.3 Integración con `VoiceAttack`
TMC detecta automáticamente si tienes *VoiceAttack* instalado y en ejecución, genera en tu equipo el perfil actualizado **`TMC-Copiloto.vap`** con las órdenes preconfiguradas y ofrece el botón **`📥 Descargar perfil (.vap)`** en Ajustes listo para importar.

### 5.4 Pantalla Táctil en Tablet o Móvil (`Código QR`)
Si juegas con un solo monitor o en realidad virtual, entra en `Ajustes`, activa **`Acceso por red local`** y escanea con la cámara de tu tablet o teléfono (conectado al mismo Wi-Fi) el **`Código QR de emparejamiento`**. Se abrirá la cabina completa en el navegador móvil sin instalar nada; incluso al tocar «Copiar sistema» en la tablet, se copiará directamente en el portapapeles de tu ordenador de juego.

---

## 6. Personalización en Ajustes y Soporte

- **Idioma y Tema visual:** selección entre **5 idiomas** (`es`, `en`, `fr`, `de`, `pt`) y **3 temas** (**`HUD automático`** sincronizado con los colores de tu cabina del juego, **`Naranja clásico`** y **`Alto contraste`**).
- **Auto-Companion Pro:** reordenación táctica de prioridades para los 9 roles (`▲` / `▼`) con botón de restablecimiento, deslizador de histéresis (0–60 s con visualizador numérico en vivo), interruptor de transiciones predictivas y casillas de feedback sensorial (bip sintético *Glass HUD Chime* y locución TTS del copiloto).
- **Preferencias operativas:** copiado automático del siguiente salto, interruptor general de `Auto-companion`, acceso por red local, interruptor global de privacidad **`Consultar servicios online (Spansh, EDSM, Canonn)`** (desactívalo para trabajar 100 % sin conexión), **`Umbral de «Mapear» (cr)`** y **`Eficiencia predeterminada del planificador (%)`**.
- **Economía, Turismo e Ingeniería:** lista editable de hasta 30 **`Minerales de interés`** (con 11 preconfigurados), **`Umbral de valor de minería (cr)`**, **`Radio de búsqueda de venta (al)`**, **`Radio de datos EDDN (al)`**, **`Referencia de turismo (cr/al)`** y **`Tiradas de ingeniería por grado (G1–G5)`**.
- **Canal directo de Soporte y Feedback:** en `Ajustes` → **Soporte, Feedback y Comunidad**, pulsa **`🚀 Reportar problema o sugerencia en GitHub`** para abrir un ticket en [GitHub Issues](https://github.com/Danikuso/TheMultipurposeCompanion/issues) con la versión, idioma y sistema operativo prellenados (sin incluir datos personales).

---

# Parte II — Documentación Técnica para Desarrolladores

## 7. Arquitectura del Workspace

El proyecto está organizado como un workspace modular en **Rust estable (edición 2024)** con responsabilidades estrictamente acotadas y sin dependencias cíclicas, acompañado de un frontend en **Svelte 5 + TypeScript + Vite** y catálogos **Fluent** para internacionalización:

| Ruta | Responsabilidad técnica |
|---|---|
| `crates/tmc-model` | Tipos de dominio fuertemente tipados, eventos del diario, estado volátil, tópicos de suscripción, diagnósticos estrictos de cabina (`app.toggle_lock`, `app.set_auto_priorities`, etc.) y configuración (`auto_companion_priorities`, `auto_companion_linger_seconds`, predictivo y avisos). |
| `crates/tmc-journal` | Descubrimiento multiplataforma de carpetas (Win32 `FOLDERID_SavedGames`, Proton/Flatpak en Linux), ingesta incremental por *byte offset* de `Journal.*.log` y sondeo tolerante a escrituras parciales de `Status.json` + 9 archivos `.json` compañeros. |
| `crates/tmc-store` | Persistencia en **SQLite (modo WAL)** (`rusqlite`), migraciones aditivas, historial de exploración/comercio/listas de compra, almacén local de credenciales y autocuración ante corrupción. |
| `crates/tmc-i18n` | Carga y resolución de catálogos **Project Fluent** (`locales/{es,en,fr,de,pt}/*.ftl`) con test automatizado de paridad total de claves respecto a `es`. |
| `crates/tmc-net` | Clientes HTTP asíncronos con limitadores de tasa, caché y degradación offline: *Spansh* (Neutron Plotter, Exact Plotter `/api/generic/route`, búsqueda combinada de astilleros/equipamiento `POST /api/stations/search`, búsqueda de yacimientos/anillos `/api/bodies/search` y mercados de venta óptima con bucle triangular, y rescate espacial `/api/nearest`), *Ardent*, *Canonn*, *tick.edcd.io*, *EDSM*, *Inara InAPI v1* (lotes máx. 2 req/min) y *Frontier CAPI* (OAuth2 S256 PKCE). |
| `crates/tmc-eddn` | Emisor HTTP POST de esquemas oficiales a *EDDN* (`journal/1`, `commodity/3`, `outfitting/2`, `shipyard/2`) con sal local seudonimizada y suscriptor ZeroMQ filtrando mercados por radio espacial. |
| `crates/tmc-tts` | Cola priorizada de locución con deduplicación y *cooldowns* (`TtsQueue`), motor nativo de Windows (SAPI5 / OneCore `< 2 MB` RAM) y proceso worker neuronal desacoplado (`Kokoro-82M` ONNX con suspensión tras 60 s de inactividad). |
| `crates/tmc-modules` | Lógica aislada por rol para los 16 módulos de juego (incluyendo el catálogo de compra `outfitting_catalog.json` en `ship` y el catálogo de yacimientos de alta densidad `mining_hotspots.json` con hot-swap atómico en `mining`), cálculo de relevancia contextual con histéresis temporal anti-salto (`linger_seconds`), disparadores predictivos (`DockingGranted`, aproximación a anillos/cinturones con instrumental minero, e interdicciones) y proyecciones hacia la interfaz. |
| `crates/tmc-core` | Actor central de estado, enrutamiento de eventos, generador táctico `VoiceAdvisor` y resolvedor léxico multilingüe `VoiceIntentResolver` (9 intenciones vocales). |
| `crates/tmc-server` | Servidor HTTP/REST y WebSocket (`/ws`) sobre **axum**, *coalescing* de tópicos cada 100 ms (máx. 10 act/s), generador de perfil `.vap` para *VoiceAttack*, protección DNS-rebinding/CSRF y autenticación LAN por token de 128 bits (`HttpOnly; SameSite=Strict`). |
| `crates/tmc-app` | Composición de servicios, arranque, importación histórica con *throttling* adaptativo y bucle de supervisión. |
| `crates/tmc-replay` | Herramienta CLI para reproducción temporal controlada de sesiones (`play`) y anonimización determinista de diarios reales (`anonymize`) para fixtures. |
| `apps/desktop` | Aplicación nativa **Tauri 2** (`TMC.exe`), gestión de instancia única, persistencia de ventana, bandeja del sistema (*System Tray*) y optimización de memoria WebView2. |
| `apps/headless` | Servidor autónomo sin ventana gráfica y motor de auditoría matemática (`tmc-headless eval`). |
| `ui/` | Interfaz reactiva en **Svelte 5** (runes), TypeScript, sistema de diseño *TMC Glass HUD*, bloqueo táctico (`Alt + L` / candado en `TopBar`), reordenación dinámica de prioridades en `Settings`, síntesis sonora de cabina (Web Audio API), listas virtualizadas de altura dinámica y reconocimiento vocal offline en WebAssembly (`Whisper` vía `@huggingface/transformers`). |
| `locales/` | Archivos `.ftl` sincronizados en `es` (referencia), `en`, `fr`, `de` y `pt`. |

---

## 8. Ingesta del Diario, Almacenamiento, Seguridad y Rendimiento

- **Lectura no intrusiva por desplazamiento de bytes (*byte offset*):** las notificaciones del sistema operativo solo se emplean para detectar la rotación a un archivo `Journal.*.log` nuevo; la lectura línea a línea avanza por offset sin bloquear descriptores de archivo de `EliteDangerous64.exe`. Los archivos volátiles (`Status.json`, `Cargo.json`, `Market.json`, `Outfitting.json`, `Shipyard.json`, `ModulesInfo.json`, `NavRoute.json`, `Backpack.json`, `ShipLocker.json`, `FCMaterials.json`) aplican hasta 10 reintentos de 50 ms ante escrituras truncadas.
- **Resiliencia y autocuración:** líneas JSON malformadas o eventos futuros desconocidos nunca detienen el actor. Si `tmc.db` presenta corrupción al iniciar, se respalda como `tmc.db.corrupt-YYYYMMDDHHMMSS`, se recrea el esquema limpio y se reimporta el historial de diarios. Si `config.toml` es inválido, se preserva en `config.toml.bak` y se restauran valores seguros.
- **Seguridad del servidor local:** escucha por defecto exclusivamente en `127.0.0.1:47320` (buscando puerto libre en `47320..=47340` si está ocupado), validando cabeceras `Host` y `Origin`. Al activar `lan = true` (`0.0.0.0`), toda petición externa a loopback exige el token aleatorio de sesión de 128 bits entregado en el código QR.
- **Presupuesto estricto de recursos (requisito verificable):** `< 150 MB` de RAM total con ventana abierta (habitualmente ~35–112 MB), `< 1 %` de CPU en reposo, cero bucles `requestAnimationFrame` continuos y máximo 10 actualizaciones/s por WebSocket. Al cerrar a la bandeja en Windows, invoca `COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW` y `SetIsVisible(false)`.

---

## 9. Rutas Locales, Argumentos CLI y Variables de Entorno

### Rutas predeterminadas
- **Diarios del juego:**
  - Windows: `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous\` (y colores HUD en `%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options\Graphics\GraphicsConfigurationOverride.xml`).
  - Linux / SteamOS: prefijos Proton de AppID `359320` en `~/.steam/steam/...`, `~/.local/share/Steam/...` o `~/.var/app/com.valvesoftware.Steam/...`.
- **Configuración:** `%APPDATA%\TMC\config.toml` (Windows) / `~/.config/tmc/config.toml` (Linux).
- **Base de datos SQLite y logs rotativos (máx. 7 días):** `%LOCALAPPDATA%\TMC\tmc.db` y `tmc*.log` (Windows) / `~/.local/share/tmc/tmc.db` (Linux).

### Argumentos de línea de comandos (`tmc-desktop` y `tmc-headless`)
- `--journal-dir <carpeta>`: fuerza la lectura de diarios y archivos `.json` desde un directorio específico.
- `--config <archivo>`: utiliza un archivo `config.toml` alternativo.
- `--ui-dir <carpeta>`: sirve los activos compilados de la interfaz web desde una ruta personalizada.

### Variables de entorno opcionales
- `TMC_CAPI_CLIENT_ID`: identificador de aplicación OAuth2 registrado en el portal de desarrolladores de Frontier para sobrescribir la configuración local de CAPI (nunca debe subirse ningún secreto al repositorio; usar `.env` local ignorado por Git).
- `VOICEATTACK_PATH` / `VOICEATTACK_DIR`: ruta personalizada de instalación de *VoiceAttack* si reside fuera de las unidades estándar.

---

## 10. Herramientas de Desarrollo, Reproducción y Auditoría

```powershell
# 1. Ejecutar el servidor + núcleo sin ventana gráfica (ideal para Linux remoto o desarrollo UI)
cargo run -p tmc-headless -- --journal-dir tests/fixtures/journal

# 2. Reproducir un diario de prueba en tiempo acelerado (10x) hacia una carpeta temporal
cargo run -p tmc-replay -- play --from tests/fixtures/journal/basic_session.log --to $env:TEMP\tmc-sim --speed 10

# 3. Anonimizar un diario real antes de convertirlo en fixture (elimina amigos, redacta chats y sustituye CMDR/FID)
cargo run -p tmc-replay -- anonymize --from C:\Ruta\Journal.log --to tests\fixtures\journal\nuevo_fixture.log

# 4. Evaluador científico sobre diarios reales sin imprimir ningún dato personal (8 oráculos matemáticos + tiempos SQLite)
cargo run -p tmc-headless -- eval --timings

# 5. Medir huella real de memoria RAM y CPU en reposo frente al presupuesto (< 150 MB RAM, < 1 % CPU)
pwsh scripts/measure-footprint.ps1 -Exe target/release/TMC.exe -JournalDir tests/fixtures/journal
```

El subcomando `tmc-headless eval` audita contra diarios reales (sin exponer PII en consola) la precisión del alcance FSD (`±0,05 al`), valores de Universal Cartographics (`±5 %`), predicción exobiológica y multiplicador `×5`, conciliación de vales de combate, refinado minero, coste medio comercial, bajas en pilas de masacre e inventario de materiales de ingeniería.

---

## 11. Compilación, Pruebas Automatizadas y Empaquetado

### Requisitos previos
- **Rust** estable (`1.90+`, edición 2024) con toolchain nativo (MSVC Build Tools en Windows; `libwebkit2gtk-4.1-dev` en Linux para escritorio).
- **Node.js** `22+` y `npm`.
- **Tauri CLI v2** (`cargo install tauri-cli@^2`) para generar el instalador.

### Flujo completo de verificación y build

```powershell
# 1. Instalar dependencias del frontend y compilar la interfaz estática (Svelte 5 + Vite)
npm.cmd --prefix ui ci
npm.cmd --prefix ui run build

# 2. Calidad y pruebas del frontend (TypeScript / svelte-check + Vitest)
npm.cmd --prefix ui run check
npm.cmd --prefix ui run test

# 3. Calidad y pruebas del backend en Rust (formato, clippy sin warnings y > 1.400 tests del workspace)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 4. Desarrollo con recarga de interfaz en caliente (proxy Vite -> 127.0.0.1:47320)
# Terminal 1: cargo run -p tmc-headless -- --journal-dir <carpeta>
# Terminal 2: npm.cmd --prefix ui run dev

# 5. Compilar binarios Release
cargo build -p tmc-desktop --release
cargo build -p tmc-headless --release

# 6. Empaquetar instalador NSIS multiidioma para Windows
Push-Location apps/desktop
cargo tauri build
Pop-Location
# Salida: target/release/bundle/nsis/TMC_0.1.0_x64-setup.exe
```

> **Nota sobre higiene de compilación:** utiliza siempre el directorio estándar `target/` del workspace (no definas `CARGO_TARGET_DIR` adicionales para evitar duplicar decenas de gigabytes de artefactos).

---

## 12. Contribución, Privacidad y Licencia

- **Licencia:** distribuido bajo **[PolyForm Noncommercial 1.0.0](LICENSE)** (permite consultar, auditar, modificar, compilar y compartir el software de forma gratuita para fines personales, educativos o comunitarios sin ánimo de lucro). Queda prohibido incorporar código o conjuntos de datos con licencias incompatibles como GPL/AGPL; toda dependencia externa debe registrarse en [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
- **Reglas para contribuciones:**
  - Ninguna cadena visible en la interfaz o en voz debe escribirse fija en el código: toda etiqueta se declara en `locales/{es,en,fr,de,pt}/*.ftl` (`es` es el idioma de referencia validado por `cargo test -p tmc-i18n`).
  - Nunca subas claves API, tokens ni diarios reales sin pasar previamente por `tmc-replay anonymize` y revisión manual.
  - Consulta el sistema de diseño de cabina en [`docs/design-system-glass-hud.md`](docs/design-system-glass-hud.md).
- **Privacidad:** TMC no incluye telemetría propia ni analíticas de usuario. Las credenciales opcionales se almacenan únicamente en la base de datos local del equipo, Frontier CAPI utiliza OAuth2 PKCE directo contra Frontier Store y EDDN emplea un identificador seudonimizado con sal aleatoria local.
- **Aviso legal:** *Elite Dangerous*, *Horizons* y *Odyssey* son marcas registradas de Frontier Developments plc. TMC es una herramienta comunitaria independiente no afiliada ni respaldada oficialmente por Frontier Developments.
