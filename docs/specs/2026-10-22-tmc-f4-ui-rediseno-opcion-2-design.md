# Especificación de diseño: Rediseño visual de Ingeniería y Materiales (Opción 2 — Flujo por Objetivos & Receta)

**Fecha:** 2026-10-22  
**Autor:** Antigravity (aprobado por el propietario: Danikuso)  
**Estado:** Propuesta aprobada por el propietario («La opción 2»)  
**Base:** [`docs/specs/2026-10-21-tmc-f4-ingenieros-materiales-design.md`](2026-10-21-tmc-f4-ingenieros-materiales-design.md)

---

## 1. Motivación y objetivo

Tras compilar y probar la aplicación en tiempo real con Elite Dangerous en ejecución, el propietario señaló que la información en los paneles de ingeniería y materiales debe ser **mucho más clara y entendible**. 

La implementación inicial de F4 cumplió todos los requisitos técnicos y contratos de datos, pero la presentación en pantalla consistía en listas de texto plano con baja jerarquía visual, lo que dificultaba entender rápidamente:
1. El estado del módulo que se está ingenierizando (qué grados faltan, qué materiales se necesitan y cuántos faltan).
2. Qué materiales están listos en bodega vs. cuáles faltan y cómo resolverlos mediante intercambio.
3. Qué ingeniero es el recomendado para el plano activo y a qué distancia se encuentra.
4. El nivel de llenado de cada material en el inventario y cuáles han alcanzado el tope de capacidad.

De entre las tres alternativas de diseño prototipadas en HTML, el propietario ha seleccionado formalmente la **Opción 2: Flujo por Objetivos & Receta**, cuyo principio rector es centrar la experiencia en la meta concreta del comandante («ingenierizar este módulo») guiándole paso a paso desde el Grado 1 al Grado 5 con soluciones integradas de materiales, intercambios y rutas.

---

## 2. Requisitos funcionales y visuales

### 2.1 Flujo de ingenierización y lista de la compra (`EngineerFlow.svelte` & `ShoppingPanel.svelte`)

1. **Stepper visual de progreso por grados (G1 a G5):**
   - Para cualquier módulo seleccionado o objetivo fijado, mostrar una tira horizontal de pasos con tarjetas para cada grado aplicable.
   - Si el módulo de la nave ya posee un grado completado (ej. G1..G3), ese paso se marca visualmente con fondo sutil, texto verde y marca de completado (`100 % Completo`).
   - El grado objetivo activo (ej. G4 o G5) se resalta con borde de acento, indicando el número estimado de tiradas (según la configuración `engineering_rolls`).
   - Cada tarjeta de grado detalla sus ingredientes con su estado de inventario.

2. **Semáforo y badge de disponibilidad de materiales:**
   - Para cada ingrediente de la receta:
     - **Listo / Cubierto:** Si el inventario dispone de las unidades requeridas, se muestra con indicador verde (`✔`), cantidad `tenemos / necesitamos` y cantidad total en bodega.
     - **Faltante:** Si faltan unidades, se destaca con color de aviso (rojo/ámbar), indicando exactamente cuántas unidades faltan (`¡Faltan X!`).

3. **Asistente de intercambio integrado en la receta:**
   - En lugar de obligar al usuario a consultar un panel de intercambios separado, si a la receta le faltan materiales, el flujo muestra una caja de solución directa:
     - Qué material del excedente propio se puede entregar.
     - Tasa de intercambio y resultado (`Entregas X de [Material A] ➔ Recibes Y de [Material B]`).
     - Ubicación del comerciante de materiales del tipo correspondiente más cercano (nombre de estación, sistema y distancia en AL).
     - Botón directo para copiar el sistema al portapapeles (compatible con el portapapeles de TMC).

4. **Tarjeta de Ingeniero Recomendado:**
   - En la cabecera de la receta, mostrar el ingeniero óptimo para este plano y grado objetivo:
     - Nombre del ingeniero.
     - Estado (*Desbloqueado*, *Invitación*, etc.).
     - Taller, sistema y distancia en años luz desde la posición actual de la nave.
     - Botón directo de «Copiar ruta».

---

### 2.2 Inventario de Materiales (`MaterialsPanel.svelte`)

1. **Barras horizontales de llenado por material:**
   - Sustituir la fría cadena de texto `X / Y` por una barra de progreso visual proporcional a la capacidad máxima de la categoría/grado (ej. 300 para G1, 250 para G2, 200 para G3, 150 para G4, 100 para G5).
   - Colores de acento por grado (G1 Gris, G2 Azul, G3 Púrpura, G4 Ámbar, G5 Naranja dorado).

2. **Marcador de MÁXIMO:**
   - Si un material alcanza su capacidad máxima (`have >= max`), se muestra un badge destacado de alerta roja/ámbar (`MÁXIMO / LLENO`) para avisar de inmediato que no se pueden recoger más unidades y conviene intercambiarlo.

3. **Marcador de RESERVADO:**
   - Si parte o la totalidad de las existencias de un material están asignadas a objetivos de la lista de la compra, se muestra la indicación `Reservado: N` para evitar que el comandante los gaste por error.

4. **Buscador rápido por texto:**
   - Campo de entrada para filtrar al instante por nombre de material sin peticiones de red ni recargas.

5. **Medidor de ocupación de categoría:**
   - Indicador de capacidad total acumulada en la categoría activa (ej. `Raw: 842 / 2.100`) con mini barra de estado.

---

### 2.3 Directorio de Ingenieros (`EngineersPanel.svelte`)

1. **Filtros rápidos por estado:**
   - Pestañas para alternar entre: *Todos*, *Desbloqueados*, *Pendientes de acceso*.

2. **Tarjetas de ingeniero con jerarquía clara:**
   - Badges semánticos: Verde para Desbloqueado, Amarillo para Invitación, Azul para Descubierto, Gris para Bloqueado.
   - Nivel de acceso con estrellas/indicador (`G5 ★★★★★`).
   - Distancia euclídea destacada en tipografía monoespaciada con unidades `al`.
   - Chips con las especialidades principales en la tarjeta (`FSD G5`, `Propulsores G3`, etc.).

3. **Detalle e inspector del ingeniero:**
   - Próximo paso destacado en una caja específica (requisito de invitación o donación pendiente con barra de progreso `tenemos / necesitamos`).
   - Cuadrícula de capacidades agrupada por tipo de módulo (Motor de distorsión, Propulsores, etc.) con sus grados máximos.

---

## 3. Restricciones técnicas y de calidad

1. **Compatibilidad estricta de contratos:**
   - Ningún cambio en las firmas de `tmc-server`, endpoints REST ni topics WebSocket (`engineering.*`).
   - Se preservan íntegros los atributos `data-testid` requeridos por las suites de pruebas existentes de Vitest y Playwright.

2. **Rendimiento:**
   - Sin animaciones infinitas en CSS ni bucles `requestAnimationFrame`.
   - El consumo de memoria y CPU debe mantenerse dentro de los límites presupuestarios (< 150 MB RAM y < 1% CPU idle).

3. **Internacionalización (i18n):**
   - Todas las cadenas nuevas o textos de estado deben declararse en los 5 idiomas oficiales (`es`, `en`, `fr`, `de`, `pt`) en `locales/*/engineering.ftl`.
