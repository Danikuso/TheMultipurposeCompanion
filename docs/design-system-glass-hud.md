# Guía del Sistema de Diseño: «TMC Glass HUD»

**Estado:** Documento normativo oficial de interfaz para TMC  
**Fecha:** 2026-10-23  
**Aplicabilidad:** Obligatorio para cualquier pantalla, componente o módulo actual y futuro de TMC.

---

## 1. Filosofía de diseño: La Cabina del AYUDANTE (Companion)

TMC no es un simple visor de registros ni un volcado de tablas de base de datos. Es un **copiloto en tiempo real** que se visualiza en una segunda pantalla o monitor ultra-panorámico mientras el comandante pilota su nave en *Elite Dangerous*.

### Principios rectores:
1. **Lectura instantánea de un vistazo (Glanceability):**
   El piloto dispone de fracciones de segundo entre maniobras, saltos o combates. La información crítica debe leerse sin esfuerzo mental ni tener que buscar con la vista entre líneas de texto apelotonado.
2. **Pedagogía proactiva (Nunca un callejón sin salida):**
   Si una opción no está disponible o requiere prerrequisitos (ej. un módulo sin ingenierizar cuyo artesano está bloqueado), se guía primero secuencialmente mediante steppers y tarjetas de desbloqueo, y a continuación se ofrecen los recursos y recetas.
3. **Acción directa a un clic:**
   Si una información conduce a una acción en el juego (viajar a un sistema, saltar, comerciar), debe existir un botón de acción inmediata (`📋 Copiar sistema`, `🔄 Recalcular ruta`, `Fijar objetivo`).
4. **Respeto absoluto al rendimiento:**
   La interfaz se ejecuta junto al juego. El consumo de RAM debe permanecer estrictamente por debajo de **150 MB** y la CPU en reposo por debajo del **1%**. Están terminantemente prohibidas las animaciones continuas por GPU, efectos de desenfoque complejos (`backdrop-filter: blur` pesado en áreas grandes) o bucles de `requestAnimationFrame` innecesarios.

---

## 2. Paleta de colores y Tokens CSS

Todos los estilos deben consumir exclusivamente las variables CSS del sistema definidas en `ui/src/styles/tokens.css`:

### 2.1 Superficies y profundidad
| Token | Valor HEX / RGBA | Uso |
|---|---|---|
| `--hud-bg` | `#090b10` | Fondo global de la aplicación (espacio profundo). |
| `--hud-surface` | `#0f121d` | Fondo principal de paneles HUD (`Panel.svelte`). |
| `--hud-surface-alt` | `#151a29` | Fondos de cabeceras de paneles y barras de navegación. |
| `--hud-surface-card` | `#161c2d` | Superficie de tarjetas interiores, filas y módulos secundarios. |
| `--hud-border` | `rgba(255, 113, 0, 0.25)` | Bordes activos o destacados en naranja HUD. |
| `--hud-border-subtle` | `rgba(255, 255, 255, 0.08)` | Bordes tenues estructurales entre tarjetas y celdas. |

### 2.2 Colores semánticos y de acento
| Token | Valor | Significado / Contexto en el juego |
|---|---|---|
| `--hud-primary` | `#ff7100` | Naranja clásico de cabina de Elite. Elementos primarios, naves, ingeniería y combate estándar. |
| `--hud-cyan` | `#06b6d4` | Escudos protectores, exploración FSS, navegación espacial y mercados. |
| `--hud-success` | `#22c55e` | Éxito, muestras biológicas completadas, estrellas repostables (scoopables), existencias de material cubiertas y márgenes de beneficio altos. |
| `--hud-warning` | `#f59e0b` | Advertencias moderadas, cuerpos con mapeo recomendado, temporizadores < 1h y subsistemas tocados. |
| `--hud-danger` | `#ef4444` | Peligro crítico, objetivos buscados (Wanted), daño cáustico, cascos < 25% y materiales faltantes. |
| `--hud-purple` | `#a855f7` | Señales biológicas, exobiología y fenómenos alienígenas. |
| `--hud-text` | `#f3f4f6` | Texto legible estándar en blanco suave. |
| `--hud-muted` | `#9ca3af` | Etiquetas de contexto, unidades de medida y texto secundario. |

---

## 3. Tipografía y Jerarquía de Información

La jerarquía visual sigue estrictamente este patrón de 3 niveles:

1. **Etiqueta de contexto (Micro-label):**
   - Tamaño: `0.7rem` a `0.75rem` (`10px`–`11px`).
   - Peso y transformación: `font-weight: 700`, `text-transform: uppercase`, `letter-spacing: 0.05em`.
   - Color: `var(--hud-muted)` o el color semántico del bloque.
2. **Valor métrico principal:**
   - Tipografía: `font-family: var(--font-mono)`.
   - Tamaño: `1.2rem` a `1.5rem` (`bold`).
   - Alineación: Alineación a la derecha en datos numéricos o destacada en cabeceras de tarjeta.
   - Variantes: `.gold` (`#fbbf24`), `.green` (`var(--hud-success)`), `.cyan` (`var(--hud-cyan)`), `.danger` (`var(--hud-danger)`).
3. **Subtítulo o detalle contextual:**
   - Tamaño: `0.8rem` (`12px`).
   - Color: `var(--hud-muted)`. Se usa para distancias (`38,4 AL`), tiempos restantes (`⏳ 42 min`) o estado de inventario.

---

## 4. Componentes Estándar del Sistema

### 4.1 Tarjeta Métrica (`.metric-card`)
Bloque autocontenido para presentar métricas de cabina:
```html
<div class="metric-card">
  <span class="metric-label">Valor Datos Actual</span>
  <span class="metric-val gold">12.450.000 Cr</span>
</div>
```

### 4.2 Medidores y Barras de Llenado (`.hud-meter-track` / `.hud-meter-fill`)
Sustituyen a las etiquetas de texto de porcentajes secos. Fondo oscuro redondeado con barra de color semántico:
```html
<div class="hud-meter-track">
  <div class="hud-meter-fill fill-green" style="width: 85%;"></div>
</div>
```
Variantes de color: `.fill-orange`, `.fill-green`, `.fill-cyan`, `.fill-purple`, `.fill-danger`.

### 4.3 Píldoras y Badges Semánticos (`.badge`)
Etiquetas compactas en monospace para clasificar el estado de entidades:
```html
<span class="badge badge-green">✔ REPOSTABLE</span>
<span class="badge badge-cyan">6 SONDAS</span>
<span class="badge badge-danger">SE BUSCA</span>
<span class="badge badge-orange">MÁXIMO</span>
```

### 4.4 Steppers Horizontales de Progreso (`.stepper`)
Muestran flujos de varias etapas (grados de ingeniería G1..G5, muestras exobiológicas 1..3, paradas de ruta comercial A➔B):
```html
<div class="stepper">
  <div class="step-item done"><div class="step-num">✔</div> Paso 1</div>
  <div class="step-divider"></div>
  <div class="step-item active"><div class="step-num">2</div> Paso 2 Activo</div>
  <div class="step-divider"></div>
  <div class="step-item"><div class="step-num">3</div> Paso 3</div>
</div>
```

### 4.5 Filas de Lista Tipo Tarjeta (`.hud-list-row`)
En lugar de tablas HTML planas sin espacio para respirar, los elementos de listas se muestran en filas tipo tarjeta interactiva con hover suave y acento izquierdo opcional:
```html
<div class="hud-list-row highlight">
  <div>
    <div class="row-title">Tierra (Sol 3)</div>
    <div class="row-subtitle">Mundo Telúrico Adecuado • 499 sl</div>
  </div>
  <div class="row-actions">
    <span class="badge badge-green">★ MAPEAR (3,2M Cr)</span>
    <button class="hud-btn btn-sm">Copiar</button>
  </div>
</div>
```

### 4.6 Botones de Acción Directa (`.hud-btn`)
Botones sutiles integrados en el HUD con feedback visual de activación:
```html
<button type="button" class="hud-btn">
  📋 Copiar Sistema
</button>
```

### 4.7 Guía del Ayudante para Desbloqueos (`.companion-unlock-guide`)
Estructura obligatoria cuando se requiere desbloquear prerrequisitos antes de acceder a una función o receta:
1. Cabecera con título de Guía de Ayudante e insignia de estado.
2. Tarjeta de la entidad objetivo (nombre, sistema, botón de copia).
3. Stepper de pasos de desbloqueo ordenados (1. Descubrimiento, 2. Invitación, 3. Tarea/Donación).
4. Sección de recursos o materiales requeridos **tras** el desbloqueo.

### 4.8 Tarjetas Compactas, Desplegables y Acordeones por Defecto (`.compact-by-default`, `.glass-select`)
**Principio del Espacio Vital en Cabina:** En la cabina de un comandante, el espacio vertical es un recurso crítico. Queda expresamente prohibido presentar por defecto listas kilométricas de decenas de filas o tarjetas extendidas que obliguen a hacer scroll continuo o desplacen herramientas esenciales:
1. **Selector Desplegable por Defecto (`<select class="glass-select">`):**
   - Cuando un panel gestiona colecciones de elementos seleccionables (módulos de nave, flota remota, objetivos de misión, materias primas), la selección primaria se realiza mediante un desplegable compacto o chips de selección rápida.
   - El elemento seleccionado se resume inmediatamente en una **Tarjeta Activa Compacta** que muestra solo la información crítica de cabina.
2. **Acordeón Colapsado por Defecto:**
   - Para comandantes que deseen una inspección tabular completa o panorámica, se proporciona un botón de alternancia o acordeón (`[ ▼ Ver lista completa ({total}) / ▲ Ocultar lista ]`).
   - El acordeón debe iniciar **siempre en estado colapsado** (`expanded = false`).
3. **Filtrado Estricto de Elementos Irrelevantes:**
   - Cualquier elemento que no sea aplicable a la vista o sobre el que no se pueda actuar (por ejemplo, módulos que no admiten ingeniería en el panel de naves) debe ser **completamente excluido** del renderizado para evitar ruido y optimizar el rendimiento.

### 4.9 Marco de tarjeta en modo edición (`.card-frame`, `.card-controls`, `.card-content`)
Patrón de la disposición personalizable de tarjetas (F15). Toda tarjeta del `Grid` se renderiza dentro de un `CardFrame`; en modo edición el marco añade controles sin alterar lo que el piloto ve fuera de él:
1. **Barra de controles en el flujo normal:** `.card-controls` (asa de arrastre ⠿, subir/bajar, anchos `1|2|3|▭`, altura `− auto +`, quitar ⊖) va **encima** de `.card-content` dentro del flujo, **nunca como superposición** sobre el título de la tarjeta. Así no tapa ni recorta nada y el título sigue legible mientras se edita.
2. **Altura fija sobre `.card-content`:** la altura elegida (`h`, múltiplos de 40 px entre 160 y 2000) se aplica a `.card-content`, no al marco. El contenido mide lo mismo dentro y fuera de edición (WYSIWYG) y la barra de controles no altera el resultado final. Con `h = null` la altura es automática.
3. **Contenido atenuado e inerte:** mientras se edita, `.card-content` se atenúa (`opacity: 0.55`, `pointer-events: none`) y lleva `inert`, de modo que ningún botón, enlace o campo interno responde ni recibe foco; solo actúan los controles del marco.
4. **`touch-action: none` solo en las asas:** el asa de arrastre y el asa de redimensionado (◢) declaran `touch-action: none` (y `user-select: none`) para que el gesto táctil no desplace la página; el resto del marco conserva el desplazamiento táctil normal.
5. **Anchos limitados por las columnas disponibles:** los botones de ancho `1|2|3|▭` se deshabilitan (`disabled`) cuando su valor supera las columnas visibles de la rejilla; el activo se marca con `aria-pressed`. Subir/bajar se deshabilitan en los extremos de la lista.
6. **Fantasma y destino de soltado:** una tarjeta cuya condición de visibilidad no se cumple se muestra como marco fantasma con marcador «auto» (`.card-placeholder`, `.ph-pill`) para poder reubicarla o quitarla. Durante el arrastre, el marco que recibirá la tarjeta se resalta con un contorno de 2 px (`[data-drop-target]`, `outline` con `--hud-primary`) y la tarjeta arrastrada se eleva (`.dragging`, `z-index` alto y sombra).
7. **Diálogos modales (catálogo de tarjetas y pestaña propia):** `role="dialog"` con `aria-modal="true"`; **Esc se gestiona en fase de captura** (`keydown` en `window` con `capture` y `stopPropagation`) para cerrar solo el diálogo **sin cancelar la edición** en curso; **atrapan el foco** con Tab/Mayús+Tab dentro del diálogo; se cierran con `pointerdown` sobre el propio fondo (no con `click`, para que soltar una selección encima no lo cierre).
8. **`DevicePrompt` no bloqueante:** la petición de registrar el dispositivo es un **banner** dentro del flujo (con botón de descartar ×), nunca un modal; el piloto puede seguir usando la app sin responder. Los errores se muestran con `role="alert"`.
9. **Píldora «sin conexión» corta con `title`:** `GridHeader` muestra una píldora `.offline-pill` de texto mínimo; la explicación completa va en el `title` (regla de longitud controlada de §4.3).
10. **Rendimiento:** sin `requestAnimationFrame` persistente ni listeners en reposo. Durante un arrastre o redimensionado se programa **como máximo un fotograma por `pointermove`**, y solo mientras dura el gesto; al soltar o cancelar (Esc, `oncancel`) se limpian la vista previa y los listeners.

---

## 5. Requisitos Técnicos y Preservación de Contratos

1. **Atributos de test (`data-testid`):**
   Todos los atributos `data-testid` existentes deben preservarse exactamente en sus respectivos elementos para garantizar la compatibilidad con la suite de pruebas unitarias y de extremo a extremo.
2. **Internacionalización (i18n):**
   Ningún texto visible para el usuario puede estar incrustado directamente en el código; todo debe pasar por el sistema de localización Fluent (`locales/{es,en,fr,de,pt}/*.ftl`), garantizando paridad exacta de claves en los 5 idiomas.
3. **Tiempos de transición CSS:**
   Cualquier micro-interacción o transición de color debe estar acotada a un máximo de `150ms ease`. Prohibidos los efectos que generen repintados continuos fuera de la tasa máxima de actualización de 10 Hz de la UI.
