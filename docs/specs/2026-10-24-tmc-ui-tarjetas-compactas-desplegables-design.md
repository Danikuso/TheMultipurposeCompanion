# Especificación de diseño: Tarjetas Compactas, Desplegables y Acordeones por Defecto («TMC Glass HUD»)

**Fecha:** 2026-10-24  
**Autor:** Antigravity (a petición del propietario: Danikuso)  
**Estado:** Propuesta normativa oficial pendiente de aprobación  
**Base:** [`docs/design-system-glass-hud.md`](../design-system-glass-hud.md) §4.8 y [`AGENTS.md`](../../AGENTS.md) §0  

---

## 1. Motivación y Filosofía de Cabina

Durante las pruebas de vuelo y configuración en tiempo real, se ha identificado una fricción ergonómica fundamental:
> *«En la parte de ingeniería que no aparezcan los módulos que no se pueden ingenierizar. Y por otro lado, planifica que por defecto las tarjetas vengan compactas. Es decir, No quiero ver la lista de módulos de golpe, me quita mucho espacio. Haz que sea un desplegable. Pues así con todo.»*

En un entorno de simulación espacial como *Elite Dangerous*, el comandante opera con un monitor secundario o una ventana acoplada. Los paneles que despliegan de golpe 20, 30 o 40 filas verticales (módulos de nave, flota remota, inventario completo, listas de misiones) saturan la vista, consumen valioso espacio vertical y obligan a hacer scroll continuo para encontrar la información crítica o interactuar con el resto de herramientas.

### Principios rectores de la solución:
1. **Compacidad por defecto:** Toda vista de colección debe presentarse de forma condensada, ofreciendo acceso inmediato al elemento activo o seleccionado mediante un desplegable (`<select class="glass-select">`) y una tarjeta de resumen.
2. **Filtrado estricto de elementos no accionables:** Los elementos sobre los que no se puede interactuar o que no aportan valor en ese contexto (como módulos de nave que no admiten modificaciones de ingeniería) quedan totalmente excluidos de la vista.
3. **Acordeón bajo demanda (colapsado por defecto):** Si el comandante desea explorar la lista completa o una vista panorámica, dispone de un botón de alternancia (`[ ▼ Ver lista completa ({total}) / ▲ Ocultar lista ]`), que **siempre inicia colapsado** (`expanded = false`).
4. **Preservación total de tests y rendimiento:** Los atributos `data-testid` y las estructuras DOM necesarias para pruebas e interacciones de teclado se mantienen intactas, garantizando cero regresiones y un consumo mínimo de memoria.

---

## 2. El Patrón Normativo: «Selector Compacto + Tarjeta Activa + Acordeón»

Cualquier pantalla o panel de TMC que gestione una colección o lista de elementos debe implementar esta tríada estructural:

```
┌─────────────────────────────────────────────────────────────┐
│ CABECERA DEL PANEL                                          │
├─────────────────────────────────────────────────────────────┤
│ [Módulo / Elemento (N): ▾ MainEngines - G5 Limpio] [▼ Ver] │ ◄── Barra de selector compacto
├─────────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ [MainEngines]  Impulsores 5A                            │ │ ◄── Tarjeta activa compacta
│ │ Aumento de Rendimiento Sucio  G5  Calidad: 100%         │ │
│ └─────────────────────────────────────────────────────────┘ │
├ · · · · · · · · · · · · · · · · · · · · · · · · · · · · · · ┤
│ (Acordeón colapsado por defecto: solo visible al pulsar)    │
│ [▼ Ver lista completa (12)] ─────────────────────────────── │
│   • Impulsores 5A · MainEngines (G5)                        │
│   • Motor de Distorsión 5A · FrameShiftDrive (G4)           │
│   • Distribuidor de Energía 5A · PowerDistributor (G3)      │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 Componentes del patrón:
1. **Barra de Selector (`.compact-selector-bar`):**
   - Un `<select class="glass-select" data-testid="...">` con fondo HUD oscuro, borde cian/naranja sutil y tipografía legible.
   - Las opciones incluyen el nombre comprensible, slot/identificador y estado sintético (ej. `[G5 - Aumento de Rendimiento]`).
2. **Tarjeta Activa de Resumen (`.active-summary-card`):**
   - Muestra la ficha esencial del elemento actualmente seleccionado.
   - Incluye badges semánticos (`GradeBadge`, `badge-blueprint`, `badge-quality`, `badge-stock`).
3. **Acordeón Opcional (`.hud-accordion` o `.module-list.is-collapsed`):**
   - Botón secundario `glass-btn-secondary` que alterna el estado `showList = !showList`.
   - Inicia siempre en `false`. Al abrirse, muestra la lista completa interactiva sin perder los botones de selección directa.

---

## 3. Especificación Módulo por Módulo

### 3.1 Módulo de Ingeniería (`ui/src/modules/engineering/`)
- **`ShipPanel.svelte` (Ya implementado y verificado):**
  - Filtro estricto: `engineerableModules = data.modules.filter(row => row.engineerable)`. Los módulos de bodega, bahías o depósitos sin ingeniería desaparecen.
  - Selector desplegable `<select data-testid="ship-module-select">` con el módulo activo.
  - Tarjeta activa con nombre, slot, plano, grado y calidad.
  - Acordeón colapsado por defecto con botón `[ ▼ Ver lista ({total}) / ▲ Ocultar lista ]`.
- **`MaterialsPanel.svelte`:**
  - Selector desplegable compacto para cambiar rápidamente de categoría (`Materiales de datos / Codificados`, `Manufacturados`, `Materias primas`, `Taquilla`).
  - Resumen métrico superior compacto (`Almacenados: 450 / 900`).
  - Filtro por grado o búsqueda con acordeón de categorías colapsable por defecto.

### 3.2 Módulo de Cuentas y Sincronización (`ui/src/modules/accounts/`)
- **`AccountsPanel.svelte` (Flota remota de Frontier CAPI):**
  - Actualmente muestra la lista de todas las naves de la flota (`data.remote_fleet`).
  - **Transformación:** En lugar de renderizar 20 o 40 tarjetas de naves en cascada, se sustituye por un selector desplegable de naves (`<select data-testid="remote-fleet-select">`) y una tarjeta compacta de la nave seleccionada (modelo, nombre, sistema de estacionamiento, valor de transferencia).
  - Acordeón colapsado para ver la flota completa tabular si el comandante lo solicita.
  - Historial de envíos EDDN / Inara / EDSM: colapsado por defecto, mostrando solo la píldora de última sincronización.

### 3.3 Módulo de Misiones y Pasajeros (`ui/src/modules/missions/`)
- **`MissionsPanel.svelte`:**
  - Selector desplegable de misión activa o agrupación por destino/facción.
  - Tarjeta de misión activa con cuenta atrás de expiración, recompensas y botón de copiar sistema objetivo.
  - Acordeón colapsado para la lista general de misiones secundarias.
- **`MassacrePanel.svelte`:**
  - Selector desplegable del objetivo de masacre activo.
  - Tarjeta de recuento óptimo con semáforo de bajas compartidas.
- **`PassengersPanel.svelte`:**
  - Selector desplegable de cabina/grupo de pasajeros en lugar de volcado de todas las cabinas.

### 3.4 Módulo de Comercio y Minería (`ui/src/modules/trade/` y `mining/`)
- **`MarketPanel.svelte`:**
  - Selector de las mejores oportunidades de compra/venta de la estación actual (`Top 5 por beneficio/t`).
  - Catálogo completo de materias primas colapsado en acordeón por defecto.
- **`CargoPanel.svelte`:**
  - Resumen visual de capacidad (`Bodega: 64 / 128 t`) con selector desplegable de mercancía activa o acordeón para inventarios mixtos de más de 5 tipos.
- **`SellPanel.svelte`:**
  - Tarjeta activa de la mejor estación para vender el cargamento actual, con botón de copiar sistema y acordeón colapsado para rutas alternativas.

### 3.5 Módulo de Fleet Carrier y Colonización (`ui/src/modules/carrier/` y `colonisation/`)
- **`CarrierOverviewPanel.svelte`:**
  - Métricas vitales en tarjetas superiores (Tritio, Salto, Saldo, Mantenimiento semanal).
  - Selector desplegable para servicios activos (Reparación, Reabastecimiento, Astillero, etc.) en lugar de desplegar todas las dependencias en vertical.
- **`ColonisationProgressPanel.svelte`:**
  - Tarjeta compacta del proyecto de colonización activo.
  - Acordeón colapsado para el desglose detallado de materiales de construcción.

### 3.6 Módulo de Powerplay 2.0 y BGS (`ui/src/modules/powerplay_bgs/`)
- **`PowerplayPanel.svelte`:**
  - Selector de potencia/sistema en disputa.
  - Tarjeta de la potencia jurada con retrato oficial conmutable y progreso semanal.
  - Lista de sistemas controlados/reforzados en acordeón colapsado.
- **`BgsPanel.svelte`:**
  - Selector de facción en el sistema actual con tarjeta de influencia, estado (Guerra, Boom, Elecciones) y colapso de facciones menores no prioritarias.

---

## 4. Requisitos de Calidad y Criterios de Aceptación

1. **Paridad de Tests:** Toda la suite de Vitest (394+ tests) debe pasar en verde en cada fase de implementación. Ningún `data-testid` existente debe ser eliminado.
2. **Cero Parpadeo y Rendimiento:** La apertura y cierre de acordeones debe realizarse mediante CSS instantáneo o transiciones breves (`<= 150ms`), sin recalcular geometrías pesadas ni generar re-renders innecesarios.
3. **Persistencia Local Opcional:** Si un comandante prefiere mantener un panel expandido, se respetará el estado durante la sesión actual de vuelo.
4. **i18n Completo:** Textos como `▼ Ver lista ({total})`, `▲ Ocultar lista`, `Módulo ({total})` deben estar localizados en los 5 idiomas (`es`, `en`, `fr`, `de`, `pt`).
