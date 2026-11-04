# Manuales del comandante - TMC 1.2.0

Cinco ediciones completas y equivalentes, con 21 capítulos, portada e índice interactivo: **23 páginas por idioma**. Describen la aplicación publicada para Windows y señalan que el soporte oficial de Linux sigue en desarrollo.

| Idioma | PDF |
|---|---|
| Español | [Manual del comandante](Manual_TMC_Comandante.pdf) |
| English | [Commander's manual](TMC_Commander_Manual_EN.pdf) |
| Français | [Manuel du commandant](Manuel_TMC_Commandant_FR.pdf) |
| Deutsch | [Kommandantenhandbuch](TMC_Kommandantenhandbuch_DE.pdf) |
| Português | [Manual do comandante](Manual_TMC_Comandante_PT.pdf) |

También están disponibles en los archivos de la [release v1.2.0](https://github.com/Danikuso/TheMultipurposeCompanion/releases/tag/v1.2.0).

## Contenido y mantenimiento

Las fuentes vigentes están en `source/{es,en,fr,de,pt}.json`. Cada edición conserva los mismos identificadores de capítulo, cuatro pasos y dos explicaciones por tema. Los nombres de pantallas se obtienen de los archivos Fluent del idioma correspondiente. `manual_print.html` es una fuente histórica anterior y no genera estas ediciones.

El texto cubre instalación, interfaz, personalización de la disposición de tarjetas, Auto-companion, nave y equipamiento, navegación, exploración, exobiología, ingeniería, combate, misiones, minería, comercio, estado legal, Fleet Carrier, colonización, Powerplay/BGS, cuentas, voz, red local, actualizaciones y soporte.

Al revisar las fuentes se deben contrastar las funciones con la versión publicada. Una estimación de valor, demanda, distancia, materiales o bonificación no se presenta como una garantía. Las claves, URLs OAuth y enlaces de emparejamiento nunca se incluyen como ejemplos reales.

## Generación

Usar Python 3.11 o posterior con las dependencias opcionales de `requirements.txt` o el runtime documental de Codex:

```powershell
python docs/manual/build_manuals.py
```

El generador escribe los cinco PDF en `output/pdf/`, valida la paridad de capítulos, exige 23 páginas y añade idioma PDF, índice enlazado y marcadores. Utiliza Segoe UI del sistema cuando está disponible y la fuente Vera de ReportLab como alternativa.

Antes de distribuir: comprobar que todo el texto de cada capítulo aparece en su página, renderizar las 115 páginas con Poppler, revisar márgenes y acabado y comprobar los cinco idiomas. Solo después copiar los PDF a `docs/manual/`; la copia española de `website/Manual_TMC_Comandante.pdf` debe coincidir por SHA256.

## Publicación

La release conserva el nombre español estable `Manual_TMC_Comandante.pdf` para no romper enlaces. Se publican cinco PDF distintos, con etiquetas de idioma; no se añade otro alias duplicado del español. Añadir o revisar estos manuales no cambia los instaladores ni la versión de la aplicación.

En esta edición se han verificado por documento: 23 páginas, 22 marcadores, contenido completo de los 21 capítulos, idioma PDF y límites de texto. Las cinco ediciones se han revisado visualmente tras renderizarlas.
