# Especificación de diseño: Rediseño visual global de la aplicación TMC (Design System «TMC Glass HUD»)

**Fecha:** 2026-10-22  
**Autor:** Antigravity (a propuesta del usuario/propietario: Danikuso)  
**Estado:** Propuesta de especificación global pendiente de aprobación  
**Base:** [`docs/specs/2026-10-22-tmc-f4-ui-rediseno-opcion-2-design.md`](2026-10-22-tmc-f4-ui-rediseno-opcion-2-design.md) y arquitectura base TMC §3.1

---

## 1. Motivación y objetivos

El rediseño visual de Ingeniería y Materiales basado en la **Opción 2 (Flujo por Objetivos & Receta)** transformó listas densas de texto plano en una experiencia visualmente jerarquizada, intuitiva y rápida de consultar mientras se pilota en Elite Dangerous:
- Steppers de progreso horizontal por fases/grados.
- Barras visuales de llenado y semáforos de existencias / suficiencia.
- Tarjetas de resumen métrico con tipografía monospace grande y etiquetas superiores compactas.
- Acciones integradas directas (copiar sistema, consultar comerciante más cercano sin cambiar de vista).

El objetivo de esta especificación es **extender este mismo lenguaje estético y principios ergonómicos a toda la aplicación** (Shell, Navegación, Exploración, Exobiología, Combate, Minería, Comercio, Misiones y Pasajeros), logrando que TMC se sienta como un sistema operativo de cabina moderno, unificado, legible de un vistazo y de respuesta inmediata.

---

## 2. Sistema de diseño «TMC Glass HUD»

El rediseño se basa en un conjunto de componentes reutilizables y tokens unificados en `ui/src/styles/`:

### 2.1 Superficies y colores semánticos
- **Fondo base (`--hud-bg`):** `#090b10` (espacio profundo).
- **Superficie de panel (`--hud-surface`):** `#0f121d` con borde sutil `rgba(255, 255, 255, 0.08)` y borde superior de acento (3px).
- **Superficie de tarjeta/fila (`--hud-surface-card`):** `#161c2d` con hover en `#1d243a`.
- **Acentos por contexto:**
  - Naranja primario (`--hud-primary` `#ff7100`): General, naves, ingeniería y combate humano.
  - Cian (`--hud-cyan` `#06b6d4` / `--hud-secondary` `#00a2ff`): Escudos, FSS, navegación y mercados.
  - Verde esmeralda (`--hud-success` `#22c55e`): Muestras exobiológicas completas, estrellas scoopables, beneficios y existencias cubiertas.
  - Ámbar (`--hud-warning` `#f59e0b`): Mapeos recomendados, alertas temporales, subsistemas dañados.
  - Rojo (`--hud-danger` `#ef4444`): Amenazas AX, objetivos buscados, integridad crítica de casco, materiales faltantes.
  - Púrpura / Violeta (`--hud-purple` `#a855f7`): Señales biológicas y exobiología.

### 2.2 Jerarquía tipográfica y micro-estructuras
- **Etiquetas de contexto:** `0.72rem`, mayúsculas (`text-transform: uppercase`), espaciado entre letras `letter-spacing: 0.06em`, color atenuado (`#9ca3af`).
- **Valores métricos destacados:** `font-family: var(--font-mono)`, `font-size: 1.25rem` a `1.5rem`, `font-weight: 700`, números tabulares alineados a la derecha cuando corresponda.
- **Medidores visuales (`.hud-meter-track` / `.hud-meter-fill`):** Tracks redondeados de 6 a 10px con degradado semántico suave según el porcentaje.
- **Píldoras y badges (`.badge`):** Fondos translúcidos (`rgba(..., 0.15)`) con borde fino (`1px solid rgba(..., 0.4)`), texto nítido en mayúsculas mono.
- **Steppers horizontales (`.stepper`):** Pasos numerados o con icono de verificación (`✔`), líneas de unión divisoras y estado activo destacado.

---

## 3. Especificación módulo por módulo

### 3.1 Shell Global (`TopBar.svelte`, `Sidebar.svelte`, `Panel.svelte`)
1. **`TopBar.svelte`:**
   - Sustituir los pares de texto plano por mini-módulos integrados con pastillas de información.
   - Combustible y casco representados con micro-barras de progreso con lectura porcentual numérica monospace.
   - Selector Auto-companion rediseñado como interruptor tipo cápsula táctil con indicador luminoso de estado (`Activo / Manual`).
   - Saldo de créditos destacado en tipografía ámbar/dorada legible a distancia.
2. **`Sidebar.svelte`:**
   - Iconografía SVG nítida y unificada para cada layout y módulo.
   - Separación clara entre Layouts (flujos inteligentes multimodales) y Vistas directas.
   - Indicador de pulso luminoso en las pastillas cuando un módulo tenga relevancia urgente o activa.
3. **`Panel.svelte`:**
   - Cabecera de panel estilizada con soporte de acento contextual (`accent-cyan`, `accent-purple`, `accent-green`).

---

### 3.2 Navegación, Exploración y Exobiología
1. **`RouteStrip.svelte`:**
   - Ficha de cabina: sistema origen, salto activo en tamaño grande con badge verde scoopable (`K (REPOSTABLE)`), distancia del salto, combustible estimado y saltos restantes.
   - Botón directo de «Copiar sistema» con feedback visual de copiado y botón de «Recalcular ruta».
2. **`SystemBodiesPanel.svelte`:**
   - Cabecera con tarjetas de valor: «Valor Datos Actual» y «Valor con Mapeo Recomendado» en dígitos destacados.
   - Sustitución de la tabla cruda por lista estructurada de cuerpos celestes:
     - Nombre del cuerpo y tipo con icono de astro.
     - Distancia en sl.
     - Badges de interés directo: `MAPEAR ★ (Cr)`, `6 SONDAS`, `TERRAFORMABLE`, `ATERRIZABLE`, `3 BIO`, `2 GEO`.
     - Atenuación de cuerpos no rentables para reducir fatiga visual.
3. **`ExobiologyPanel.svelte`:**
   - Tarjeta de especie activa con valor de muestra, bonificación de primera muestra y medidor de distancia genética (`520 m / 500 m ✔`).
   - Stepper horizontal de muestras (`Muestra 1 ✔` ➔ `Muestra 2 ✔` ➔ `Muestra 3`).
   - Lista de especies completadas en el cuerpo con badge verde y créditos totales.

---

### 3.3 Combate y Operaciones AX
1. **`TargetPanel.svelte`:**
   - Tarjeta de objetivo con nombre, nave, badge de estado (`SE BUSCA $285.000 Cr` en rojo o `LIMPIO` en verde).
   - Barras duales de gran formato: Escudo (Cian) y Casco (Naranja/Rojo) con porcentaje en números grandes.
   - Grilla de 3 subsistemas críticos prioritarios (Planta de energía, Impulsores, Motor FSD) con medidores individuales.
2. **`HuntingPanel.svelte` & `ConflictZonePanel.svelte`:**
   - Tarjetas de bonos de combate acumulados en la sesión y desglose por facciones en barras proporcionales.
   - Barra de equilibrio de la zona de conflicto (Progreso aliado vs enemigo).
3. **`AxPanel.svelte`:**
   - Tarjeta de amenaza Thargoide (Clase de Interceptor, amenaza, enjambre de drones).
   - Indicador visual de corazones activos/destruidos (`CORAZÓN 1 ✕`, `CORAZÓN 2 ✕`, `CORAZÓN 3 ACTIVO`).
   - Alerta visual destacada de daño cáustico y escudo de plasma.

---

### 3.4 Minería
1. **`ProspectorPanel.svelte`:**
   - Porcentaje restante del asteroide en número grande y barra superior.
   - Composición mineral en barras horizontales ordenadas por valor económico en el mercado galáctico.
   - Insignia destacada en asteroides ricos o con núcleos de voladura (Core mining).
2. **`MiningSessionPanel.svelte`:**
   - Rendimiento de la sesión: toneladas refinadas, créditos acumulados y tasa de extracción (t/h y Cr/h).
3. **`SellPanel.svelte`:**
   - Tarjeta de las 3 mejores estaciones de venta cercanas con precio por tonelada, demanda, distancia, pad requerido y botón «Copiar sistema».

---

### 3.5 Comercio
1. **`MarketPanel.svelte`:**
   - Buscador de mercancías en cabecera con filtros por margen o categoría.
   - Filas de mercancía con precios de compra/venta y beneficio proyectado por tonelada destacado en badge verde (`+18.450 Cr/t`).
2. **`CargoPanel.svelte`:**
   - Capacidad de bodega en barra visual (`128 / 720 t`) y desglose de beneficio estimado al descargar.
3. **`TradeRoutePanel.svelte`:**
   - Stepper de ruta comercial de bucle: Paso A (Comprar X en Sol) ➔ Paso B (Vender X y comprar Y en Alpha Centauri).
   - Tarjetas de beneficio por viaje completo y rendimiento estimado por hora.

---

### 3.6 Misiones y Pasajeros
1. **`MissionsPanel.svelte`:**
   - Misiones agrupadas por sistema/estación de destino con cabecera de grupo y botón «Añadir a ruta».
   - Tarjeta de misión con tipo, facción, temporizador dinámico con semáforo de expiración (`>2h` verde, `<1h` ámbar, `<15m` rojo) y recompensa destacada.
2. **`MassacrePanel.svelte`:**
   - Optimización de apilamiento de misiones de masacre pirata (Pirate Massacre Stacking): barras de progreso de bajas separadas por facción contratante emisora.
3. **`PassengersPanel.svelte` & `TourismPanel.svelte`:**
   - Ocupación de cabinas en cuadrícula visual y progreso del itinerario turístico parada por parada.

---

## 4. Requisitos técnicos y no regresión

1. **Integridad de tests:**
   - Mantener íntegros todos los atributos `data-testid` actuales (ej. `topbar-system`, `system-name`, `body-row`, `body-marks`, `market-station`, `missions-group`, etc.).
   - Los 378 tests de Vitest y las pruebas Playwright E2E deben continuar pasando en verde.
2. **Rendimiento:**
   - Sin animaciones infinitas de GPU ni bucles `requestAnimationFrame`.
   - Transiciones CSS limitadas a `150ms ease`.
   - Consumo < 150 MB RAM y < 1% CPU idle en reposo.
3. **i18n:**
   - Todas las cadenas visibles extraídas a `locales/{es,en,fr,de,pt}/*.ftl`.
