# Catálogos de ingeniería: segunda búsqueda web

Fecha: 2026-10-22. Encargo del propietario: buscar en Internet, incluidas wikis y APIs, la información que falta para completar F4. Investigación de solo lectura; ningún catálogo de esta búsqueda se ha incorporado al producto.

## Resultado y propuesta concreta

Se encuentra un candidato A que el spike inicial no había evaluado: **EDDiscovery/EliteDangerousCore**. Sus archivos de recetas, ingenieros y mapas auxiliares llevan una cabecera explícita Apache-2.0, además de la licencia del repositorio. Se propone usar esos archivos como fuente A para sustituir las 2 recetas, 1 experimental y 5 ingenieros provisionales. El propietario aprobó explícitamente el cambio de B a A en esta sesión: «Aprobar EDDiscovery como fuente A». No se propone incorporar código de ejecución de EDDiscovery ni recursos gráficos.

Revisión fijada: [`7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7`](https://github.com/EDDiscovery/EliteDangerousCore/commit/7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7), publicada el 2026-10-22 a las 09:30:22 UTC. La actualidad del commit no certifica cada receta; las entradas importadas seguirían con `verified:false` hasta contrastarlas con el juego.

Archivos candidatos, todos fijados a esa revisión:

- [RecipesEngineering.cs](https://github.com/EDDiscovery/EliteDangerousCore/blob/7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7/EliteDangerous/FrontierData/Items/RecipesEngineering.cs): recetas por módulo/grado, ingenieros, experimentales, variantes preingenierizadas y recetas de a pie. Cabecera Apache-2.0 del equipo EDDiscovery.
- [Engineers.cs](https://github.com/EDDiscovery/EliteDangerousCore/blob/7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7/EliteDangerous/FrontierData/Items/Engineers.cs): **38 ingenieros**, sistemas, coordenadas, talleres y requisitos. Cabecera Apache-2.0.
- [MCMRType.cs](https://github.com/EDDiscovery/EliteDangerousCore/blob/7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7/EliteDangerous/FrontierData/Items/MCMRType.cs): correspondencia de abreviaturas de ingredientes con materiales. Cabecera Apache-2.0.
- [Modules.cs](https://github.com/EDDiscovery/EliteDangerousCore/blob/7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7/EliteDangerous/FrontierData/Items/Modules.cs) y [RecipeFDName.cs](https://github.com/EDDiscovery/EliteDangerousCore/blob/7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7/EliteDangerous/FrontierData/FDName/RecipeFDName.cs): identificadores y tipos; revisar todos los archivos que realmente use el conversor.
- [LICENSE](https://github.com/EDDiscovery/EliteDangerousCore/blob/7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7/LICENSE): Apache-2.0. Conservar licencia, atribución y aviso de modificación en los datos derivados y `THIRD_PARTY_NOTICES.md`.

## Cobertura medida y límites de la propuesta

Auditoría exploratoria sobre el archivo fijado: 1.167 líneas constructoras de recetas, incluidas otras categorías. Un patrón que reconoce únicamente el constructor de blueprint con un tipo de módulo encuentra 924 filas; 895 tienen ingredientes e ingeniero, con 83 claves y 46 tipos de módulo. Son **filas por grado**, no 895 blueprints distintos. Hay otros constructores de experimentales y recetas de a pie que ese recuento no mide. No es todavía una auditoría de completitud.

En ese subconjunto hay **105 filas con coste en Merc Coins**. La fuente contiene contenido de Operations y separa recetas de variantes que no se pueden fabricar. El modelo actual de F4 solo representa ingredientes de materiales: no se debe importar un coste parcial como si fuera el total, ni etiquetar las variantes preingenierizadas como recetas ordinarias. Antes de implementar la importación, debe explicitarse en el plan cómo representar y mostrar ese coste, conforme al alcance «todos los blueprints» de la spec.

También hay que preservar el tipo de módulo al resolver los ingenieros: la fuente asigna ingenieros/grados por receta **y módulo**, mientras que el catálogo provisional de TMC solo asocia blueprint con grado máximo. Que dos módulos compartan clave del diario y receta no implica que los mismos ingenieros puedan modificar ambos al mismo grado. La conversión requiere una auditoría de estas colisiones y una propuesta de interfaz antes de modificar consumidores.

Ejemplo comprobado en la revisión fijada: `misc_shielded`, Lori Jameson, llega a G4 para Auto Field-Maintenance Unit/Fuel Scoop/Life Support/Refinery y a G3 para Cargo Scanner/Kill Warrant Scanner/Wake Scanner. Un único `max_grade` por clave produciría una recomendación incorrecta para alguno de esos módulos.

La receta `engine_dirty` y el experimental `special_engine_overloaded` de la fuente difieren de las cantidades provisionales actuales. No basta con añadir filas: será necesario actualizar las expectativas calculadas a mano, fixtures sintéticos y pruebas dependientes en el mismo cambio. Las pruebas actuales acreditan la implementación con sus datos provisionales, no la exactitud de esos datos frente al juego.

## Otras fuentes revisadas

| Fuente primaria | Resultado |
|---|---|
| [Almanac-Core, SOURCES](https://github.com/Elite-Dangerous-Almanac/Almanac-Core/blob/main/data/ships/SOURCES.md) | Documenta 129 identidades de blueprint y 89 de efectos/especiales. Sus comprobaciones en el juego cubren identidades, no todas las cantidades. Explica que los costes derivan de Coriolis y otras fuentes; el MIT del código no vuelve a licenciar esos datos. Útil para auditar cobertura y distinguir variantes. |
| [EDSY, eddb.js](https://github.com/taleden/EDSY/blob/master/eddb.js) | Base modificada el 2026-10-20 según su cabecera. Código CC BY-NC 4.0; datos/lógica del juego siguen siendo de Frontier, con enlace a autorización de uso. No es un dataset MIT/Apache. El enlace a las reglas de Frontier devolvió mantenimiento durante esta revisión. |
| [Frameshift](https://github.com/tannermidd/frameshift) | Anuncia más de 1.100 recetas y licencia MIT; su propia página atribuye el catálogo a EDEngineer. No resuelve la procedencia ya detectada en el primer spike. |
| [Elite Dangerous Field Manual](https://edfieldmanual.com/wiki/Engineering_Blueprints) | Wiki CC BY-SA 4.0; la página revisada solo enumera tres blueprints de FSD. No importar tablas bajo la decisión vigente de la spec §9. |
| [EDDataAPI](https://github.com/EDDataAPI/eddata-api/blob/main/API_OVERVIEW.md) | Datos de estaciones/mercados/servicios; no se encuentra un endpoint de catálogo de recetas en su documentación. Un indicador de taller no proporciona recetas ni requisitos. |
| [EDDI](https://github.com/EDCD/EDDI/blob/develop/DataDefinitions/Blueprint.cs) | Apache-2.0 en LICENSE.md; identificadores y acceso a plantillas de ingredientes. No se ha auditado el origen y cobertura de esas plantillas; no se propone como primera opción frente a las cabeceras explícitas de EDDiscovery. |
| [elite-engineers](https://github.com/gregmalcolm/elite-engineers) | MIT, prototipo con copyright 2016. No demuestra cobertura del sistema actual; no usar como catálogo completo. |
| [SDK de Junzuki](https://github.com/CMDR-Junzuki/Elite-Dangerous-3rd-Party-SDK) | Código MIT que agrupa fuentes como Coriolis/FDevIDs; revisar las condiciones de cada origen, no tratar el envoltorio MIT como una nueva licencia de sus tablas. |

Las wikis y APIs se han buscado tal como pidió el propietario. No se han copiado tablas GPL/AGPL ni CC BY-SA, ni se ha añadido ningún servicio externo a TMC. El conector de GitHub no devolvió contenido tras dos intentos; se usaron páginas públicas y la API pública de GitHub sin credenciales para leer archivos fijados. Las descargas exploratorias permanecen en `.tmp/catalog-research/`, fuera del seguimiento Git.

## Próximo paso

**Mapa auxiliar de IDs, aprobado por el propietario:** [EDDI `DataDefinitions/Engineer.cs`, revisión `d3b964ea7c8bb959ad6537f55b308f293a326905`](https://github.com/EDCD/EDDI/blob/d3b964ea7c8bb959ad6537f55b308f293a326905/DataDefinitions/Engineer.cs). La auditoría de solo lectura encuentra 38 pares ID/nombre, 38 IDs únicos y correspondencia **38/38 por nombre exacto** con `Engineers.cs` de EDDiscovery; cero ausencias y cero alias necesarios. `LICENSE.md` y `LicenseFile.txt` de esa revisión declaran Apache-2.0. Se propone importar únicamente esos pares con atribución para conservar las identidades actuales, sin copiar sus recetas ni especialidades. El propietario aprobó por separado este mapa auxiliar y ejecutar la enmienda técnica; todavía no se ha incorporado al producto.

1. **Hecho:** el propietario aprueba el cambio de fuente B a A para los archivos Apache de EDDiscovery. Documentar la atribución al importar.
2. Preparar la enmienda concreta del plan para cobertura completa, coste en Merc Coins, identificadores compartidos y grados por módulo/ingeniero; no inventar ni omitir costes.
3. Tras aprobación de las interfaces que cambien, conversión reproducible con revisión por fila, auditoría de cobertura y pruebas de recetas reales. Mantener `verified:false` y nombres sin traducción conocida como pendientes.
4. Continuar entretanto Tasks 26–27; F4 no se declara completa con los catálogos provisionales.
