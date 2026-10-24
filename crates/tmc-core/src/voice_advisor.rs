//! Asesor táctico de cabina (`VoiceAdvisor`) para TMC.
//!
//! Analiza eventos del diario de navegación, telemetría de combate, escaneos
//! de exploración y estados de la nave para generar avisos de voz contextuales (`VoiceCue`).

use serde_json::Value;
use std::collections::BTreeSet;
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;
use tmc_model::status::{Flags, Status};
use tmc_model::tts::{TtsChannel, TtsPriority, VoiceCue};

/// Lista canónica de minerales de alto valor para alertas de prospección.
const VALUABLE_MINERALS: &[&str] = &[
    "platinum",
    "painite",
    "tritium",
    "voidopal",
    "lowtemperaturediamond",
    "alexandrite",
    "benitoite",
    "grandidierite",
    "musgravite",
    "serendibite",
    "monazite",
];

/// Generador táctico de avisos de cabina.
#[derive(Debug, Default)]
pub struct VoiceAdvisor {
    /// Estado anterior del escudo (`true` si estaba arriba).
    last_shields_up: Option<bool>,
    /// Último porcentaje de integridad estructural del casco conocido (0..100).
    last_hull_pct: Option<f64>,
    /// Si la nave estaba sobrecalentada en la última lectura.
    last_overheating: bool,
    /// Si la nave estaba sufriendo interdicción en la última lectura.
    last_interdicted: bool,
    /// Último estado legal conocido de la nave ("Clean", "Wanted", "IllegalCargo", etc.).
    last_legal_state: Option<String>,
    /// Si la nave porta actualmente mercancía robada en bodega.
    has_illegal_cargo: bool,
    /// Nivel de notoriedad estimado para detectar transiciones a limpia.
    last_notoriety: u8,
    /// Lista de especies o muestras ya avisadas en el sistema actual para evitar duplicidad.
    pub processed_samples: BTreeSet<String>,
}

impl VoiceAdvisor {
    /// Crea un nuevo asesor de cabina.
    pub fn new() -> Self {
        Self::default()
    }

    /// Procesa un evento del sistema y devuelve cualquier aviso de voz que deba ser emitido.
    pub fn process_event(&mut self, ev: &SourceEvent, _state: &GameState) -> Vec<VoiceCue> {
        let mut cues = Vec::new();

        match ev {
            SourceEvent::Journal {
                ev: je, replaying, ..
            } => {
                // Durante una reproducción o importación histórica no se emite audio de cabina
                if *replaying {
                    return cues;
                }

                let raw_val: Value = serde_json::from_str(je.raw.get()).unwrap_or_default();
                self.process_journal_event(&je.name, &raw_val, &mut cues);
            }
            SourceEvent::Status(status) => {
                self.process_status(status, &mut cues);
            }
            _ => {}
        }

        cues
    }

    fn process_journal_event(&mut self, name: &str, event: &Value, cues: &mut Vec<VoiceCue>) {
        match name {
            // 1. Navegación: Salto completado y estrella repostable / no repostable
            "FSDJump" => {
                if let Some(system) = event.get("StarSystem").and_then(Value::as_str) {
                    let star_class = event
                        .get("StarClass")
                        .and_then(Value::as_str)
                        .unwrap_or_default();

                    let is_scoopable = matches!(
                        star_class.to_ascii_uppercase().as_str(),
                        "O" | "B" | "A" | "F" | "G" | "K" | "M"
                    );

                    let cue_key = if is_scoopable {
                        "tts-nav-jump-scoopable"
                    } else {
                        "tts-nav-jump-unscoopable"
                    };

                    cues.push(
                        VoiceCue::new(TtsPriority::Tactical, TtsChannel::Navigation, cue_key)
                            .with_arg("system", system)
                            .with_cooldown("nav_jump"),
                    );

                    // Alerta de contrabando si portamos carga ilegal y el sistema tiene seguridad alta o media
                    if self.has_illegal_cargo {
                        let sec = event
                            .get("SystemSecurity")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        let is_high_or_med = sec.contains("High")
                            || sec.contains("high")
                            || sec.contains("Medium")
                            || sec.contains("medium");
                        if is_high_or_med {
                            cues.push(
                                VoiceCue::new(
                                    TtsPriority::Tactical,
                                    TtsChannel::Combat,
                                    "tts-crime-illegal-cargo",
                                )
                                .with_cooldown("illegal_cargo_jump"),
                            );
                        }
                    }
                }
            }

            // 2. Exploración: Detección de bio-señales en cuerpos del sistema
            "FSSBodySignals" => {
                if let Some(signals) = event.get("Signals").and_then(Value::as_array) {
                    for sig in signals {
                        let sig_type = sig.get("Type").and_then(Value::as_str).unwrap_or_default();
                        if sig_type.contains("Biological")
                            || sig_type.contains("$SAA_SignalType_Biological")
                        {
                            let count = sig.get("Count").and_then(Value::as_u64).unwrap_or(1);
                            cues.push(
                                VoiceCue::new(
                                    TtsPriority::Tactical,
                                    TtsChannel::Exploration,
                                    "tts-nav-bio-signals",
                                )
                                .with_arg("count", count.to_string())
                                .with_cooldown("bio_signals"),
                            );
                            break;
                        }
                    }
                }
            }

            // 3. Exobiología: Progreso de toma de muestras biológicas (1/3, 2/3, 3/3)
            "ScanOrganic" => {
                let scan_type = event
                    .get("ScanType")
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                let genus = event
                    .get("Genus_Localised")
                    .or_else(|| event.get("Genus"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                match scan_type {
                    "Log" => {
                        cues.push(
                            VoiceCue::new(
                                TtsPriority::Tactical,
                                TtsChannel::Exploration,
                                "tts-exo-sample-progress",
                            )
                            .with_arg("current", "1")
                            .with_arg("total", "3")
                            .with_arg("species", genus),
                        );
                    }
                    "Sample" => {
                        cues.push(
                            VoiceCue::new(
                                TtsPriority::Tactical,
                                TtsChannel::Exploration,
                                "tts-exo-sample-progress",
                            )
                            .with_arg("current", "2")
                            .with_arg("total", "3")
                            .with_arg("species", genus),
                        );
                    }
                    "Analyse" => {
                        cues.push(
                            VoiceCue::new(
                                TtsPriority::Tactical,
                                TtsChannel::Exploration,
                                "tts-exo-sample-complete",
                            )
                            .with_arg("species", genus),
                        );
                    }
                    _ => {}
                }
            }

            // 4. Combate / Daño de casco crítico
            "HullDamage" => {
                if let Some(health) = event.get("Health").and_then(Value::as_f64) {
                    let pct = (health * 100.0).round();
                    let was = self.last_hull_pct.unwrap_or(100.0);
                    // Avisar si cae por debajo del 50% o del 25%
                    if (was > 50.0 && pct <= 50.0) || (was > 25.0 && pct <= 25.0) {
                        cues.push(
                            VoiceCue::new(
                                TtsPriority::Emergency,
                                TtsChannel::Combat,
                                "tts-combat-hull-warning",
                            )
                            .with_arg("pct", pct.to_string())
                            .with_cooldown("hull_warning"),
                        );
                    }
                    self.last_hull_pct = Some(pct);
                }
            }

            // 5. Combate: Estado de escudos por evento explícito
            "ShieldState" => {
                if let Some(shields_up) = event.get("ShieldsUp").and_then(Value::as_bool) {
                    self.handle_shields_transition(shields_up, cues);
                }
            }

            // 6. Minería: Detección de asteroide con mineral valioso
            "ProspectedAsteroid" => {
                if let Some(materials) = event.get("Materials").and_then(Value::as_array) {
                    for mat in materials {
                        let name_raw = mat
                            .get("Name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_lowercase()
                            .replace(['$', ';', '_', ' '], "");

                        let proportion =
                            mat.get("Proportion").and_then(Value::as_f64).unwrap_or(0.0);

                        // Si la proporción es >= 0.20 (20%) y es un mineral valioso
                        if proportion >= 20.0 || (0.20..=1.0).contains(&proportion) {
                            let pct_val = if proportion <= 1.0 {
                                (proportion * 100.0).round() as i64
                            } else {
                                proportion.round() as i64
                            };

                            let is_valuable =
                                VALUABLE_MINERALS.iter().any(|m| name_raw.contains(m));
                            if is_valuable {
                                let display_name = mat
                                    .get("Name_Localised")
                                    .or_else(|| mat.get("Name"))
                                    .and_then(Value::as_str)
                                    .unwrap_or("Mineral");

                                cues.push(
                                    VoiceCue::new(
                                        TtsPriority::Tactical,
                                        TtsChannel::Mining,
                                        "tts-mining-valuable-find",
                                    )
                                    .with_arg("material", display_name)
                                    .with_arg("pct", pct_val.to_string())
                                    .with_cooldown("valuable_asteroid"),
                                );
                                break;
                            }
                        }
                    }
                }
            }

            // 7. Fleet Carrier: Aviso de salto inminente
            "CarrierJumpRequest" => {
                cues.push(
                    VoiceCue::new(
                        TtsPriority::Tactical,
                        TtsChannel::Carrier,
                        "tts-carrier-jump-imminent",
                    )
                    .with_cooldown("carrier_jump"),
                );
            }

            // 8. Crimen y Notoriedad: Asesinato u homicidio
            "CommitCrime" => {
                let crime_type = event
                    .get("CrimeType")
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                if crime_type.to_ascii_lowercase().contains("murder") {
                    self.last_notoriety = self.last_notoriety.saturating_add(1);
                    cues.push(
                        VoiceCue::new(
                            TtsPriority::Emergency,
                            TtsChannel::Combat,
                            "tts-crime-notoriety-gain",
                        )
                        .with_cooldown("notoriety_gain"),
                    );
                } else if let Some(fine) = event.get("Fine").and_then(Value::as_u64) {
                    cues.push(
                        VoiceCue::new(
                            TtsPriority::Informational,
                            TtsChannel::Combat,
                            "tts-crime-fine-incurred",
                        )
                        .with_arg("amount", fine.to_string())
                        .with_cooldown("fine_incurred"),
                    );
                }
            }

            // 9. Contrabando: Detección de mercancías robadas en bodega
            "Cargo" => {
                if let Some(inv) = event.get("Inventory").and_then(Value::as_array) {
                    let has_stolen = inv
                        .iter()
                        .any(|item| item.get("Stolen").and_then(Value::as_u64).unwrap_or(0) > 0);
                    if has_stolen && !self.has_illegal_cargo {
                        cues.push(
                            VoiceCue::new(
                                TtsPriority::Tactical,
                                TtsChannel::Combat,
                                "tts-crime-illegal-cargo",
                            )
                            .with_cooldown("illegal_cargo"),
                        );
                    }
                    self.has_illegal_cargo = has_stolen;
                }
            }

            // 10. Interdicción policial
            "Interdicted" => {
                let interdictor = event
                    .get("Interdictor")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let is_police = interdictor.to_ascii_lowercase().contains("police")
                    || interdictor.to_ascii_lowercase().contains("security")
                    || interdictor.to_ascii_lowercase().contains("enforcer")
                    || interdictor.to_ascii_lowercase().contains("patrol")
                    || interdictor.to_ascii_lowercase().contains("viper");
                if is_police {
                    cues.push(
                        VoiceCue::new(
                            TtsPriority::Emergency,
                            TtsChannel::Combat,
                            "tts-crime-police-interdiction",
                        )
                        .with_cooldown("interdiction_police"),
                    );
                }
            }

            // 11. Pago de recompensas / Limpieza de notoriedad
            "PayBounties" if self.last_notoriety > 0 => {
                self.last_notoriety = 0;
                cues.push(
                    VoiceCue::new(
                        TtsPriority::Informational,
                        TtsChannel::Combat,
                        "tts-crime-notoriety-cleared",
                    )
                    .with_cooldown("notoriety_cleared"),
                );
            }

            _ => {}
        }
    }

    fn process_status(&mut self, status: &Status, cues: &mut Vec<VoiceCue>) {
        let is_in_main_ship = status.flags.contains(Flags::IN_MAIN_SHIP);

        // Solo evaluar alertas de nave si estamos en la nave principal
        if is_in_main_ship {
            // Escudos arriba / abajo
            let shields_up = status.flags.contains(Flags::SHIELDS_UP);
            self.handle_shields_transition(shields_up, cues);

            // Sobrecalentamiento (> 100% temperatura)
            let is_overheating = status.flags.contains(Flags::OVERHEATING);
            if is_overheating && !self.last_overheating {
                let temp_pct = status
                    .temperature
                    .map(|t| (t * 100.0).round() as i64)
                    .unwrap_or(100);

                cues.push(
                    VoiceCue::new(
                        TtsPriority::Emergency,
                        TtsChannel::Combat,
                        "tts-combat-heat-warning",
                    )
                    .with_arg("pct", temp_pct.to_string())
                    .with_cooldown("heat_warning"),
                );
            }
            self.last_overheating = is_overheating;

            // Alerta de interdicción en curso
            let is_interdicted = status.flags.contains(Flags::BEING_INTERDICTED);
            if is_interdicted && !self.last_interdicted {
                cues.push(
                    VoiceCue::new(
                        TtsPriority::Emergency,
                        TtsChannel::Combat,
                        "tts-crime-police-interdiction",
                    )
                    .with_cooldown("interdiction"),
                );
            }
            self.last_interdicted = is_interdicted;

            // Alerta de estado legal (contrabando o búsqueda y captura)
            if let Some(ref ls) = status.legal_state
                && self.last_legal_state.as_deref() != Some(ls.as_str())
            {
                if ls == "IllegalCargo" {
                    cues.push(
                        VoiceCue::new(
                            TtsPriority::Tactical,
                            TtsChannel::Combat,
                            "tts-crime-illegal-cargo",
                        )
                        .with_cooldown("illegal_cargo"),
                    );
                } else if ls == "Wanted" {
                    cues.push(
                        VoiceCue::new(
                            TtsPriority::Emergency,
                            TtsChannel::Combat,
                            "tts-crime-wanted",
                        )
                        .with_cooldown("status_wanted"),
                    );
                } else if ls == "Clean" && self.last_notoriety > 0 {
                    self.last_notoriety = 0;
                    cues.push(
                        VoiceCue::new(
                            TtsPriority::Informational,
                            TtsChannel::Combat,
                            "tts-crime-notoriety-cleared",
                        )
                        .with_cooldown("notoriety_cleared"),
                    );
                }
            }
            self.last_legal_state = status.legal_state.clone();
        }
    }

    fn handle_shields_transition(&mut self, shields_up: bool, cues: &mut Vec<VoiceCue>) {
        if let Some(was_up) = self.last_shields_up {
            if was_up && !shields_up {
                // ¡Alerta! Escudos caídos
                cues.push(
                    VoiceCue::new(
                        TtsPriority::Emergency,
                        TtsChannel::Combat,
                        "tts-combat-shields-down",
                    )
                    .with_cooldown("shields_down"),
                );
            } else if !was_up && shields_up {
                // Escudos restaurados
                cues.push(
                    VoiceCue::new(
                        TtsPriority::Tactical,
                        TtsChannel::Combat,
                        "tts-combat-shields-up",
                    )
                    .with_cooldown("shields_up"),
                );
            }
        }
        self.last_shields_up = Some(shields_up);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tmc_model::events::parse_line;
    use tmc_model::status::Flags2;

    fn journal_ev(line: &str) -> SourceEvent {
        SourceEvent::Journal {
            ev: parse_line(line).expect("Línea de diario debe parsear correctamente"),
            file: "journal.log".into(),
            line: 1,
            replaying: false,
        }
    }

    #[test]
    fn nav_jump_generates_scoopable_and_unscoopable_cues() {
        let mut advisor = VoiceAdvisor::new();
        let state = GameState::default();

        // 1. Salto a estrella repostable (KGB FOAM)
        let ev_scoopable = journal_ev(
            r#"{"timestamp":"2026-10-24T12:00:00Z","event":"FSDJump","StarSystem":"Sol","StarClass":"G","SystemAddress":123,"StarPos":[0.0,0.0,0.0]}"#,
        );
        let cues = advisor.process_event(&ev_scoopable, &state);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text_key, "tts-nav-jump-scoopable");
        assert_eq!(cues[0].args.get("system").unwrap(), "Sol");
        assert_eq!(cues[0].priority, TtsPriority::Tactical);

        // 2. Salto a estrella no repostable (tipo T Tauri o Neutrón)
        let ev_unscoopable = journal_ev(
            r#"{"timestamp":"2026-10-24T12:05:00Z","event":"FSDJump","StarSystem":"Jackson's Lighthouse","StarClass":"N","SystemAddress":456,"StarPos":[1.0,2.0,3.0]}"#,
        );
        let cues2 = advisor.process_event(&ev_unscoopable, &state);
        assert_eq!(cues2.len(), 1);
        assert_eq!(cues2[0].text_key, "tts-nav-jump-unscoopable");
        assert_eq!(cues2[0].args.get("system").unwrap(), "Jackson's Lighthouse");
    }

    #[test]
    fn exobiology_sample_progression() {
        let mut advisor = VoiceAdvisor::new();
        let state = GameState::default();

        // Muestra 1
        let sample1 = journal_ev(
            r#"{"timestamp":"2026-10-24T12:00:00Z","event":"ScanOrganic","ScanType":"Log","Genus":"Stratum"}"#,
        );
        let c1 = advisor.process_event(&sample1, &state);
        assert_eq!(c1.len(), 1);
        assert_eq!(c1[0].text_key, "tts-exo-sample-progress");
        assert_eq!(c1[0].args.get("current").unwrap(), "1");

        // Muestra 2
        let sample2 = journal_ev(
            r#"{"timestamp":"2026-10-24T12:05:00Z","event":"ScanOrganic","ScanType":"Sample","Genus":"Stratum"}"#,
        );
        let c2 = advisor.process_event(&sample2, &state);
        assert_eq!(c2.len(), 1);
        assert_eq!(c2[0].args.get("current").unwrap(), "2");

        // Muestra 3 completada
        let sample3 = journal_ev(
            r#"{"timestamp":"2026-10-24T12:10:00Z","event":"ScanOrganic","ScanType":"Analyse","Genus":"Stratum"}"#,
        );
        let c3 = advisor.process_event(&sample3, &state);
        assert_eq!(c3.len(), 1);
        assert_eq!(c3[0].text_key, "tts-exo-sample-complete");
    }

    #[test]
    fn combat_shields_and_overheat_transitions() {
        let mut advisor = VoiceAdvisor::new();
        let state = GameState::default();

        // Estado inicial: escudos arriba
        let mut status = Status {
            timestamp: Utc::now(),
            flags: Flags::IN_MAIN_SHIP | Flags::SHIELDS_UP,
            flags2: Flags2::empty(),
            pips: None,
            fire_group: None,
            gui_focus: Default::default(),
            fuel: None,
            cargo: None,
            legal_state: None,
            latitude: None,
            longitude: None,
            altitude: None,
            heading: None,
            body_name: None,
            planet_radius: None,
            balance: None,
            destination: None,
            oxygen: None,
            health: None,
            temperature: Some(0.4),
            gravity: None,
            selected_weapon: None,
        };
        advisor.process_event(&SourceEvent::Status(status.clone()), &state);

        // Transición: escudos caídos
        status.flags.remove(Flags::SHIELDS_UP);
        let cues = advisor.process_event(&SourceEvent::Status(status.clone()), &state);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text_key, "tts-combat-shields-down");
        assert_eq!(cues[0].priority, TtsPriority::Emergency);

        // Transición: sobrecalentamiento
        status.flags.insert(Flags::OVERHEATING);
        status.temperature = Some(1.25);
        let cues_heat = advisor.process_event(&SourceEvent::Status(status), &state);
        assert_eq!(cues_heat.len(), 1);
        assert_eq!(cues_heat[0].text_key, "tts-combat-heat-warning");
        assert_eq!(cues_heat[0].args.get("pct").unwrap(), "125");
    }

    #[test]
    fn mining_valuable_asteroid_cue() {
        let mut advisor = VoiceAdvisor::new();
        let state = GameState::default();

        let asteroid_ev = journal_ev(
            r#"{"timestamp":"2026-10-24T12:00:00Z","event":"ProspectedAsteroid","Materials":[{"Name":"Indite","Proportion":12.0},{"Name":"$platinum_name;","Name_Localised":"Platinum","Proportion":35.4}]}"#,
        );

        let cues = advisor.process_event(&asteroid_ev, &state);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text_key, "tts-mining-valuable-find");
        assert_eq!(cues[0].args.get("material").unwrap(), "Platinum");
        assert_eq!(cues[0].args.get("pct").unwrap(), "35");
        assert_eq!(cues[0].channel, TtsChannel::Mining);
    }

    #[test]
    fn carrier_jump_request_cue() {
        let mut advisor = VoiceAdvisor::new();
        let state = GameState::default();

        let carrier_ev = journal_ev(
            r#"{"timestamp":"2026-10-24T12:00:00Z","event":"CarrierJumpRequest","Body":"Sol 3","SystemName":"Sol"}"#,
        );

        let cues = advisor.process_event(&carrier_ev, &state);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text_key, "tts-carrier-jump-imminent");
        assert_eq!(cues[0].channel, TtsChannel::Carrier);
    }

    #[test]
    fn crime_and_security_voice_cues() {
        let mut advisor = VoiceAdvisor::new();
        let state = GameState::default();

        // 1. CommitCrime con Murder emite ganancia de notoriedad
        let murder_ev = journal_ev(
            r#"{"timestamp":"2026-10-24T12:00:00Z","event":"CommitCrime","CrimeType":"murder","Faction":"Independent Pilots","Bounty":10000}"#,
        );
        let cues = advisor.process_event(&murder_ev, &state);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text_key, "tts-crime-notoriety-gain");
        assert_eq!(cues[0].priority, TtsPriority::Emergency);

        // 2. CommitCrime con multa menor emite fine incurred
        let fine_ev = journal_ev(
            r#"{"timestamp":"2026-10-24T12:05:00Z","event":"CommitCrime","CrimeType":"recklessWeaponsDischarge","Faction":"Federation","Fine":500}"#,
        );
        let cues2 = advisor.process_event(&fine_ev, &state);
        assert_eq!(cues2.len(), 1);
        assert_eq!(cues2[0].text_key, "tts-crime-fine-incurred");
        assert_eq!(cues2[0].args.get("amount").unwrap(), "500");

        // 3. Cargo con mercancía robada
        let cargo_ev = journal_ev(
            r#"{"timestamp":"2026-10-24T12:10:00Z","event":"Cargo","Inventory":[{"Name":"gold","Count":10,"Stolen":4}]}"#,
        );
        let cues3 = advisor.process_event(&cargo_ev, &state);
        assert_eq!(cues3.len(), 1);
        assert_eq!(cues3[0].text_key, "tts-crime-illegal-cargo");
        assert_eq!(cues3[0].priority, TtsPriority::Tactical);

        // 4. Salto a sistema de alta seguridad portando carga ilegal
        let jump_ev = journal_ev(
            r#"{"timestamp":"2026-10-24T12:15:00Z","event":"FSDJump","StarSystem":"Achenar","StarClass":"A","SystemSecurity":"$SYSTEM_SECURITY_high;","SystemAddress":789,"StarPos":[1.0,1.0,1.0]}"#,
        );
        let cues4 = advisor.process_event(&jump_ev, &state);
        // Debe incluir aviso de carga ilegal
        assert!(
            cues4
                .iter()
                .any(|c| c.text_key == "tts-crime-illegal-cargo")
        );

        // 5. Interdicción policial
        let interdict_ev = journal_ev(
            r#"{"timestamp":"2026-10-24T12:20:00Z","event":"Interdicted","Submitted":false,"Interdictor":"Federal Security Service","IsPlayer":false,"Faction":"Federation"}"#,
        );
        let cues5 = advisor.process_event(&interdict_ev, &state);
        assert_eq!(cues5.len(), 1);
        assert_eq!(cues5[0].text_key, "tts-crime-police-interdiction");

        // 6. Limpieza de notoriedad al pagar recompensas
        let pay_bounties_ev = journal_ev(
            r#"{"timestamp":"2026-10-24T12:25:00Z","event":"PayBounties","Amount":10000,"Faction":"Independent Pilots"}"#,
        );
        let cues6 = advisor.process_event(&pay_bounties_ev, &state);
        assert_eq!(cues6.len(), 1);
        assert_eq!(cues6[0].text_key, "tts-crime-notoriety-cleared");
    }
}
