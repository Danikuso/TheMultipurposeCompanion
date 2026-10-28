# Reglas de predicción de exobiología (`bio_rules.json`) — F1

Fecha: 2026-10-18 · Archivo: `crates/tmc-modules/data/bio_rules.json` · Código: `crates/tmc-modules/src/exobiology/predict.rs`

## Cómo se han producido

- Han sido redactadas para TMC con **conocimiento propio de la mecánica del juego**: tipo de cuerpo, tipo de atmósfera, gravedad baja (~0,27 g; ~0,15 g en Tubus), tramos de temperatura, vulcanismo y, en Electricae Pluma, la clase de la estrella principal.
- **No** se han consultado ni copiado los `bio-criteria` de SrvSurvey (GPL), ni las tablas de la wiki de Fandom (CC-BY-SA), ni los datos de Canonn (`biostats`/`codex`, GPL-3.0). Ver `spike_licencias_f1.md` y `spike_canonn_f1.md`.
- Cada regla lleva en `source` la base específica de su género y la nota «redactada por TMC con conocimiento propio del juego; no derivada de SrvSurvey ni copiada de wikis; umbrales a verificar con tmc-headless eval».
- Licencia: datos propios del proyecto (PolyForm Noncommercial 1.0.0).

## Formato y semántica

- `{ version, source, license, rules: [{ species, body_class?, atmosphere_type?, gravity_min?, gravity_max?, temp_min?, temp_max?, pressure_max?, volcanism?, star_class?, source }] }`.
- Si falta una cláusula, no restringe; las cláusulas se combinan con AND; varias reglas de la misma especie, con OR. Si el perfil del cuerpo no tiene el dato que exige una cláusula, la regla no casa.
- `volcanism`: `"any"`, `"none"` o una lista de palabras clave (basta con que aparezca una en el texto `Volcanism` del diario). `star_class`: un patrón acabado en `*` se compara por prefijo (`"D*"` = cualquier enana blanca).

## Cobertura

- 90 reglas, una por especie, para los 15 géneros de Odyssey: Aleoida, Bacterium, Cactoida, Clypeus, Concha, Electricae, Fonticulua, Frutexa, Fumerola, Fungoida, Osseus, Recepta, Stratum, Tubus y Tussock.
- Sin reglas: los géneros de Horizons (Amphora Plant, Anemone, Bark Mound, Brain Tree, Crystalline Shards, Sinuous Tubers). Con DSS se listan sus especies como «desconocida» (nunca se inventan); sin DSS no se proponen.

## Límites conocidos

- Son **heurísticas amplias**: prima la cobertura sobre la precisión. Habrá falsos positivos (p. ej. varias especies de Tussock/Bacterium a la vez) y falsos negativos en los rangos estrechos de temperatura.
- No se modelan: composición atmosférica en %, materiales, distancia a la estrella (Clypeus Speculumi > 2500 ls), cercanía a nebulosas (Electricae Radialem, Bark Mound), estrellas locales distintas de la principal, ni las variantes de color.
- La presión solo se admite como máximo (`pressure_max`) y apenas se usa.

## Plan de verificación

1. Con `tmc-headless eval` (cuando exista) sobre el historial del propietario: para cada cuerpo con señales biológicas y alguna especie muestreada (`ScanOrganic` Analyse), comprobar si esa especie está entre los candidatos «Alta» o «Media». **Criterio de aceptación §10.6: ≥ 70 % de los cuerpos.**
2. Revisar los fallos por género: ajustar los umbrales (temperatura, gravedad, atmósfera) en el JSON, sin tocar código, y subir `version`.
3. Registrar en el registro de este documento cada ajuste, con la fecha y la cifra del eval antes y después.
4. La capa de historial compensa con el uso: con ≥ 3 casos en la misma clave de perfil, una especie sin regla llega a «Media».

## Registro

- 2026-10-18 — versión inicial (90 reglas, 15 géneros). Sin eval todavía.
