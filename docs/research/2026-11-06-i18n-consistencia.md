# Coherencia de las traducciones de TMC

Revisión solicitada por el propietario y ampliada con su indicación expresa «Recogible es Repostable». Mantenimiento de traducciones y corrección de defectos de i18n; no añade funcionalidades, servicios ni dependencias. Rama fase/f16-guia-ingenieros. F16/T6 mantiene su medición interactiva pendiente.

## Cobertura

Leídos los 105 recursos iniciales mediante revisión por idiomas (skill review; informes del 2026-11-05) y contrastados con el código consumidor. Inicialmente 1981 mensajes por idioma. Se revisan claves, variables, atributos, UTF-8, duplicados, resolución de referencias y plurales con Fluent real. Se amplía el alcance a los textos fijados en español en componentes, necesarios para que cambiar el idioma tenga efecto uniforme.

| Idioma | Mensajes finales | Mensajes existentes corregidos | Mensajes añadidos | Nombres pendientes conservados |
|---|---:|---:|---:|---:|
| es | 2078 | 166 | 97 | 85 |
| en | 2078 | 31 | 97 | 0 |
| fr | 2078 | 147 | 97 | 85 |
| de | 2078 | 105 | 97 | 85 |
| pt | 2078 | 136 | 97 | 118 |

Las cifras cuentan mensajes Fluent, no líneas; un mensaje con varias ramas de plural cuenta una vez. Los nombres pending no son una ausencia de clave ni un error de sintaxis.

## Correcciones aplicadas

- Repostable / No repostable en español; se elimina Recogible y estrellas escoop. Misma terminología en exploración, navegación, guía y locución. Unidades al/ly/Lj según idioma y valores numéricos de nave con el formateador de la aplicación.
- Planos, drones, portanaves, plataformas, intermediario tecnológico y existencias en español. Equivalentes coherentes por idioma (plan/Blaupause/projeto; drones/Drohnen). Se corrigen el estado de facción, la escotilla de carga, la navegación silenciosa y el significado de supercrucero frente a hiperespacio.
- Francés: En règle y Sans loi, materiales Bruts, Modifier un module, ravitaillement stellaire, drones, points chauds, aire d’atterrissage y acentos de nombres importados.
- Portugués: eliminación de cuando/operaciones, acentos en nombres de efectos, refuerzo de casco genérico sin añadir metaaleación, recompra no limite y puntos de concentración. Las ramas de cero conservan el valor y el plural natural; no muestran una unidad cuando llegan cero.
- Rangos: 84 etiquetas por idioma y ocho familias. Se corrigen también las etiquetas inglesas antiguas de CQC y exobiología, manteniendo sus índices numéricos. La guía reutiliza las claves del comandante y el objetivo de combate traduce los valores conocidos del diario; los valores futuros desconocidos se conservan.
- Nombres y ranuras de módulos: se elimina el español fijado en shipFormatters; se reutiliza el catálogo de ingeniería y se conservan tamaño/clase, montajes, capacidad y símbolos desconocidos. La munición de fragmentación deja de confundirse con el cañón genérico por el orden del reconocimiento de nombres.
- Textos visibles y ayudas extraídos a Fluent: nave, módulos, minería, cuentas, BGS, colonización, portanaves, misiones, mercados, ruta activa, ajustes, actualizador, voz, grados y emblemas. Los selectores y desplegables comparten textos de lista y encabezados. No se modifican IDs, acciones ni solicitudes de búsqueda.

## Procedencia y excepciones

Las etiquetas de rango se extraen de [EDCD/EDDI, revisión d3b964e](https://github.com/EDCD/EDDI/tree/d3b964ea7c8bb959ad6537f55b308f293a326905/DataDefinitions/Properties), Apache-2.0, fuente ya utilizada por el proyecto. scripts/data/rank-localization.json fija 40 URLs y SHA-256. No se incorpora código C#. Diez nombres franceses ausentes se completan editorialmente en TMC; también se normaliza la etiqueta de ausencia de rango. Estas decisiones están separadas del snapshot y no se hacen pasar por traducciones oficiales de Frontier. Atribución ampliada en THIRD_PARTY_NOTICES.md.

Se preservan sistemas, estaciones, ingenieros, facciones, marcas, identificadores y nombres de objetos del juego sin una localización confirmada, junto con los atributos pending del catálogo. Se conservan símbolos y siglas técnicas (G, P, FID, CMDR, unidades SI), nombres nativos del selector de idiomas y datos remotos desconocidos. La igualdad con el inglés por sí sola no demuestra un defecto de traducción.

Los tres informes iniciales del 2026-11-05 son diagnósticos previos a las correcciones; este documento registra la decisión final. No se ha cambiado la variante regional global de portugués ni se atribuye a Frontier una traducción editorial propia.

## Verificación

Estado al cerrar la revisión: Node datos/importador 17 PASS, importador --check PASS, sync-rank-locales --check PASS, fmt y clippy --workspace --all-targets -- -D warnings PASS; cargo test --workspace 1622 PASS / 3 ignorados, incluidos los 18 tests de i18n. Playwright guía/layout 24/24 PASS. Check UI final: 0 errores/avisos; 709 tests PASS; build PASS. Tras el último ajuste de CSS, revisión visual de 20 casos, check y build PASS; Playwright 24/24 repetido sobre los assets finales PASS.

Revisión visual sintética: 20 vistas de siete paneles reales, cinco idiomas y anchuras 375/1280/1920; 21 vistas de la guía. Sin controles fuera del marco ni desbordamiento horizontal. Capturas de nave española estrecha, objetivo francés, cuentas alemanas y minería portuguesa inspeccionadas; se corrigieron además los decimales de nave y el plural portugués de cero. Una comprobación adicional con tarjeta de comandante de 300 px dentro de ventana de 1280 detectó insignias fuera de su casilla. Corregido con flex-wrap y ajuste de línea, sin alterar posiciones guardadas. Cinco vistas adicionales confirman el arreglo y comprueban el marco interno de cada rango. Scripts reproducibles: ui/scripts/verify-locale-panels.mjs y verify-engineer-guide.mjs, con Vite en 47680. Evidencias en la carpeta i18n-review de las visualizaciones de esta conversación.

Cierre de QA: corpus final de 2078 mensajes por idioma, 585 mensajes existentes corregidos y 97 claves nuevas en cada idioma (textos antes fijados en el código). Se conserva la separación entre nombres pendientes y confirmados. Servidores de QA cerrados; salidas temporales de Playwright eliminadas; limpieza del perfil dev de Cargo antes del release. Ejecutable de entrega y hash: se registrarán tras compilar, sin iniciar la GUI en una sesión de fondo.
