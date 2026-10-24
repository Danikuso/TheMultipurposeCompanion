# Plan de ejecución: Rediseño visual de Ingeniería y Materiales (Opción 2 — Flujo por Objetivos & Receta)

**Fecha:** 2026-10-22  
**Spec de referencia:** [`docs/specs/2026-10-22-tmc-f4-ui-rediseño-opcion-2-design.md`](../specs/2026-10-22-tmc-f4-ui-rediseño-opcion-2-design.md)  
**Rama de trabajo:** `fase/f4-ingenieros-materiales`  
**Estado:** Preparado para ejecución

---

## Tareas de ejecución

### Tarea 1: Textos e i18n en los 5 idiomas
- **Archivos:** `locales/{es,en,fr,de,pt}/engineering.ftl`
- **Contenido:**
  - Claves para el stepper: `engineering-step-completed`, `engineering-step-target`, `engineering-step-rolls`.
  - Claves para el semáforo de materiales: `engineering-mat-ready`, `engineering-mat-missing`, `engineering-mat-stock`.
  - Claves para el asistente de intercambio integrado: `engineering-trade-solution-title`, `engineering-trade-give`, `engineering-trade-receive`, `engineering-trade-trader-at`, `engineering-copy-system`.
  - Claves para filtros y estado de materiales/ingenieros: `engineering-search-placeholder`, `engineering-filter-all`, `engineering-filter-unlocked`, `engineering-filter-pending`, `engineering-reserved-count`, `engineering-category-capacity`.
- **Verificación:** Test `engineering_ftl_keys_match_all_locales` en Rust workspace.
- **Commit:** `feat(i18n): añadir cadenas para el rediseño de flujo de ingeniería`

---

### Tarea 2: Rediseño visual de `MaterialsPanel.svelte`
- **Archivos:** `ui/src/modules/engineering/MaterialsPanel.svelte`
- **Cambios:**
  - Barra de ocupación total de la categoría activa (ej. `842 / 2100`) con medidor horizontal.
  - Buscador reactivo por texto (nombre de material).
  - Cada fila de material gana:
    - Indicador visual de grado (G1 a G5) con estilo acorde a su rareza.
    - Barra horizontal de llenado (`have / max`) con transición suave.
    - Badge destacado de `MÁXIMO` si `count >= max`.
    - Indicación de unidades reservadas si está en objetivos.
  - Conservar íntegros los atributos `data-testid` (`tab-Raw`, `tab-Manufactured`, etc.) y la sección de taquilla de a pie.
- **Verificación:** `npm --prefix ui test` (tests de materiales en `tests/engineering.test.ts`), `npm --prefix ui run check`.
- **Commit:** `feat(ui): barras de llenado y buscador visual en panel de materiales`

---

### Tarea 3: Rediseño visual de `EngineersPanel.svelte`
- **Archivos:** `ui/src/modules/engineering/EngineersPanel.svelte`
- **Cambios:**
  - Filtro por estado: pestañas `Todos`, `Desbloqueados`, `Pendientes`.
  - Tarjetas de ingeniero con badges de color semántico (Verde = Desbloqueado, Amarillo = Invitación, etc.).
  - Nivel de acceso formateado con estrellas (`G5 ★★★★★`) y distancia destacada.
  - Inspector lateral: caja de próximo paso con barra de donación (`tenemos / necesitamos`) y cuadrícula de capacidades agrupada por tipo de módulo.
  - Preservar `data-testid="engineers-ship"`, `data-testid="engineer-details"`, `data-testid="engineer-next-step"`, etc.
- **Verificación:** `npm --prefix ui test`, `npm --prefix ui run check`.
- **Commit:** `feat(ui): tarjetas visuales y filtros de estado en panel de ingenieros`

---

### Tarea 4: Stepper por grados y asistente de intercambio en `EngineerFlow.svelte` y `ShoppingPanel.svelte`
- **Archivos:** `ui/src/modules/engineering/EngineerFlow.svelte`, `ui/src/modules/engineering/ShoppingPanel.svelte`
- **Cambios:**
  - Stepper visual horizontal de grados (G1..G5):
    - Grados ya aplicados en el módulo marcados como `Completado`.
    - Grado objetivo destacado con tiradas estimadas y coste.
  - Semáforo de ingredientes: indicador verde (`✔`) cuando hay existencias, rojo/ámbar cuando faltan unidades.
  - Asistente de intercambio integrado: si faltan materiales, muestra la tarjeta de solución directa con material a entregar del excedente, comerciante más cercano (sistema, estación y distancia) y botón de copiar sistema al portapapeles.
  - Tarjeta de ingeniero recomendado en cabecera con taller, distancia y botón de copiar ruta.
  - Preservar todos los `data-testid` de selección, tiradas y acciones (`pin_goal`, etc.).
- **Verificación:** `npm --prefix ui test`, `npm --prefix ui run check`, `npm --prefix ui run build`.
- **Commit:** `feat(ui): stepper de grados y asistente de intercambio integrado en flujo de ingeniería`

---

### Tarea 5: Verificación completa y compilación del binario
- **Acciones:**
  - Ejecutar suite completa de tests de UI (`npm --prefix ui test`) y verificación estática (`npm --prefix ui run check`).
  - Ejecutar suite de workspace Rust (`cargo test --workspace`).
  - Compilar `ui/dist` (`npm --prefix ui run build`).
  - Compilar binario debug `cargo build -p tmc-desktop`.
  - Actualizar `status.md`.
- **Commit:** `chore: verificación y compilación de la UI rediseñada de ingeniería`
