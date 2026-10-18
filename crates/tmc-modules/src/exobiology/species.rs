//! Species values and genus colony distances from project-owned factual data.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct SpeciesTable {
    pub source: String,
    pub license: String,
    pub version: String,
    pub genera: Vec<GenusInfo>,
}

#[derive(Debug, Deserialize)]
pub struct GenusInfo {
    #[serde(rename = "genus")]
    pub symbol: String,
    pub name: String,
    pub ccr_m: u32,
    pub provenance: String,
    pub date: String,
    pub verified: bool,
    pub species: Vec<SpeciesInfo>,
}

#[derive(Debug, Deserialize)]
pub struct SpeciesInfo {
    #[serde(rename = "species")]
    pub symbol: String,
    #[serde(skip)]
    pub genus: String,
    pub name: String,
    pub value: u64,
    pub provenance: String,
    pub date: String,
    pub verified: bool,
}

impl SpeciesTable {
    pub fn genus(&self, symbol: &str) -> Option<&GenusInfo> {
        self.genera.iter().find(|genus| genus.symbol == symbol)
    }
    pub fn species(&self, symbol: &str) -> Option<&SpeciesInfo> {
        self.genera
            .iter()
            .flat_map(|genus| &genus.species)
            .find(|species| species.symbol == symbol)
    }
    pub fn species_of_genus(&self, genus: &str) -> &[SpeciesInfo] {
        self.genus(genus)
            .map_or(&[], |entry| entry.species.as_slice())
    }

    pub(crate) fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut table: Self = serde_json::from_str(json)?;
        for genus in &mut table.genera {
            for species in &mut genus.species {
                species.genus.clone_from(&genus.symbol);
            }
        }
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::species_table;

    #[test]
    fn species_table_has_ccr_for_every_genus_and_values_positive() {
        let table = species_table();
        assert!(!table.genera.is_empty());
        for genus in &table.genera {
            assert!(genus.ccr_m > 0);
            assert!(!table.species_of_genus(&genus.symbol).is_empty());
            for species in table.species_of_genus(&genus.symbol) {
                assert!(species.value > 0);
                assert_eq!(species.genus, genus.symbol);
            }
        }
    }

    #[test]
    fn bacterium_ccr_is_500_and_osseus_800() {
        let table = species_table();
        assert_eq!(
            table
                .genus("$Codex_Ent_Bacterial_Genus_Name;")
                .unwrap()
                .ccr_m,
            500
        );
        assert_eq!(
            table.genus("$Codex_Ent_Osseus_Genus_Name;").unwrap().ccr_m,
            800
        );
    }

    #[test]
    fn species_lookup_matches_parent_genus_and_unknown_is_empty() {
        let table = species_table();
        let genus = table.genera.first().unwrap();
        let species = table.species_of_genus(&genus.symbol).first().unwrap();
        let found = table.species(&species.symbol).unwrap();
        assert_eq!(found.name, species.name);
        assert_eq!(found.genus, genus.symbol);
        assert_eq!(found.value, species.value);
        assert!(table.genus("$Unknown;").is_none());
        assert!(table.species("$Unknown;").is_none());
        assert!(table.species_of_genus("$Unknown;").is_empty());
    }

    #[test]
    fn embedded_table_is_singleton_and_records_entry_provenance() {
        let table = species_table();
        assert!(std::ptr::eq(table, species_table()));
        assert!(!table.source.is_empty());
        assert!(table.license.contains("PolyForm"));
        assert!(!table.version.is_empty());
        for genus in &table.genera {
            assert!(!genus.provenance.is_empty());
            assert_eq!(genus.date.len(), 10);
            for species in &genus.species {
                assert!(!species.provenance.is_empty());
                assert_eq!(species.date.len(), 10);
            }
        }
    }

    #[test]
    fn parser_keeps_latin_name_and_populates_genus_without_external_names() {
        // Synthetic symbols and amounts exercise parsing, not game data.
        let table = SpeciesTable::from_json(
            r#"{
          "source":"synthetic", "license":"test", "version":"test",
          "genera":[{"genus":"$Test_Genus;", "name":"Testus", "ccr_m":42,
            "provenance":"synthetic", "date":"2026-10-18", "verified":false,
            "species":[{"species":"$Test_Species;", "name":"Testus fictus", "value":17,
              "provenance":"synthetic", "date":"2026-10-18", "verified":true}]}]
        }"#,
        )
        .unwrap();
        let genus = table.genus("$Test_Genus;").unwrap();
        assert!(!genus.verified);
        let species = table.species("$Test_Species;").unwrap();
        assert_eq!(species.genus, "$Test_Genus;");
        assert_eq!(species.name, "Testus fictus");
        assert_eq!(species.value, 17);
        assert!(species.verified);
    }

    #[test]
    fn species_symbols_are_unique_and_every_genus_has_species() {
        let table = species_table();
        let mut seen = std::collections::HashSet::new();
        for genus in &table.genera {
            assert!(!genus.species.is_empty(), "{} has no species", genus.symbol);
            for species in &genus.species {
                assert!(seen.insert(species.symbol.as_str()), "{}", species.symbol);
            }
        }
        let genera: std::collections::HashSet<_> = table
            .genera
            .iter()
            .map(|genus| genus.symbol.as_str())
            .collect();
        assert_eq!(genera.len(), table.genera.len());
    }

    #[test]
    fn known_values_match_codex_symbols() {
        let table = species_table();
        let tectonicas = table.species("$Codex_Ent_Stratum_07_Name;").unwrap();
        assert_eq!(tectonicas.name, "Stratum Tectonicas");
        assert_eq!(tectonicas.value, 19_010_800);
        assert_eq!(tectonicas.genus, "$Codex_Ent_Stratum_Genus_Name;");
        // Fixture SellOrganicData: Bacterium Aurasus pays 1 000 000 base.
        let aurasus = table.species("$Codex_Ent_Bacterial_01_Name;").unwrap();
        assert_eq!(aurasus.name, "Bacterium Aurasus");
        assert_eq!(aurasus.value, 1_000_000);
        assert_eq!(
            table.genus("$Codex_Ent_Stratum_Genus_Name;").unwrap().ccr_m,
            500
        );
    }

    #[test]
    fn malformed_table_returns_error() {
        assert!(SpeciesTable::from_json("{bad json}").is_err());
    }
}
