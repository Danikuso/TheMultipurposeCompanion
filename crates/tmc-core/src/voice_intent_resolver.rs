//! Intérprete semántico de intenciones de voz contra el estado de juego (Fase 10).

use tmc_model::crime::NearbyFacility;
use tmc_model::state::GameState;
use tmc_model::status::Flags;
use tmc_model::voice_intent::{VoiceCommandResponse, VoiceIntent};

/// Contexto auxiliar de resolución proveniente de módulos de la sesión.
#[derive(Debug, Clone, Default)]
pub struct VoiceResolverContext {
    pub locale: String,
    pub nearest_interstellar_factor: Option<NearbyFacility>,
    pub nearest_black_market: Option<NearbyFacility>,
    pub total_fines: u64,
    pub total_bounties: u64,
    pub notoriety: u8,
    pub bio_signal_bodies_count: u32,
}

/// Normaliza una cadena de texto para comparación léxica tolerante.
pub fn normalize_query(query: &str) -> String {
    let lower = query.to_lowercase();
    let mut normalized = String::with_capacity(lower.len());

    for ch in lower.chars() {
        match ch {
            'á' | 'à' | 'ä' | 'â' | 'ã' => normalized.push('a'),
            'é' | 'è' | 'ë' | 'ê' => normalized.push('e'),
            'í' | 'ì' | 'ï' | 'î' => normalized.push('i'),
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' => normalized.push('o'),
            'ú' | 'ù' | 'ü' | 'û' => normalized.push('u'),
            'ñ' => normalized.push('n'),
            'ç' => normalized.push('c'),
            '?' | '¿' | '!' | '¡' | '.' | ',' | ';' | ':' | '"' | '\'' => {}
            _ => normalized.push(ch),
        }
    }

    normalized
}

/// Analiza la cadena normalizada y determina la intención principal.
pub fn parse_intent(query: &str) -> VoiceIntent {
    let q = normalize_query(query);

    // 1. Acciones de copia priorizadas
    if (q.contains("copi") || q.contains("copy") || q.contains("kopier"))
        && (q.contains("siguiente")
            || q.contains("proximo")
            || q.contains("salto")
            || q.contains("sistema")
            || q.contains("next")
            || q.contains("jump")
            || q.contains("sprung")
            || q.contains("saut"))
    {
        return VoiceIntent::CopyNextSystem;
    }

    // 2. Interstellar Factor
    if q.contains("factor")
        || q.contains("interstellar")
        || q.contains("fator")
        || (q.contains("limpiar") && (q.contains("multa") || q.contains("recompensa")))
        || (q.contains("pagar") && (q.contains("multa") || q.contains("recompensa")))
        || (q.contains("clear") && (q.contains("fine") || q.contains("bount")))
        || (q.contains("pay") && (q.contains("fine") || q.contains("bount")))
    {
        return VoiceIntent::NearestInterstellarFactor;
    }

    // 3. Mercado Negro
    if q.contains("negro")
        || q.contains("black market")
        || q.contains("marche noir")
        || q.contains("schwarzmarkt")
        || q.contains("contraband")
        || q.contains("robado")
        || q.contains("stolen")
    {
        return VoiceIntent::NearestBlackMarket;
    }

    // 4. Crimen y multas
    if q.contains("multa")
        || q.contains("recompensa")
        || q.contains("notoriedad")
        || q.contains("notoriety")
        || q.contains("buscado")
        || q.contains("wanted")
        || q.contains("fine")
        || q.contains("bount")
        || q.contains("gesucht")
        || q.contains("recherche")
        || q.contains("procurado")
    {
        return VoiceIntent::CrimeStatus;
    }

    // 5. Siguiente salto / ruta
    if q.contains("siguiente salto")
        || q.contains("proximo salto")
        || q.contains("siguiente sistema")
        || q.contains("proximo sistema")
        || q.contains("next jump")
        || q.contains("next system")
        || q.contains("prochain saut")
        || q.contains("nachster sprung")
        || q.contains("proximo salto")
        || q.contains("a donde vamos")
        || q.contains("donde vamos")
        || q.contains("destino")
        || q.contains("destination")
    {
        return VoiceIntent::NextJump;
    }

    // 6. Bio-señales y exobiología
    if q.contains("bio")
        || q.contains("biolog")
        || q.contains("exobiolog")
        || q.contains("senales de vida")
    {
        return VoiceIntent::BioSignals;
    }

    // 7. Combustible
    if q.contains("combustible")
        || q.contains("fuel")
        || q.contains("carburant")
        || q.contains("treibstoff")
        || q.contains("gasolina")
        || q.contains("tanque")
    {
        return VoiceIntent::FuelStatus;
    }

    // 8. Materiales de ingeniería
    if q.contains("material") || q.contains("materiaux") || q.contains("materialien") {
        return VoiceIntent::MaterialsSummary;
    }

    // 9. Estado de nave
    if q.contains("estado")
        || q.contains("nave")
        || q.contains("ship")
        || q.contains("vaisseau")
        || q.contains("schiff")
        || q.contains("casco")
        || q.contains("hull")
        || q.contains("escudo")
        || q.contains("shield")
        || q.contains("danos")
        || q.contains("health")
    {
        return VoiceIntent::ShipStatus;
    }

    VoiceIntent::Unknown(query.to_string())
}

/// Estructura que resuelve intenciones de voz contra el `GameState`.
pub struct VoiceIntentResolver;

impl VoiceIntentResolver {
    /// Resuelve una consulta de voz y devuelve la respuesta estructurada.
    pub fn resolve(
        query: &str,
        state: &GameState,
        ctx: &VoiceResolverContext,
    ) -> VoiceCommandResponse {
        let intent = parse_intent(query);
        let locale = ctx.locale.as_str();

        match intent {
            VoiceIntent::ShipStatus => Self::resolve_ship_status(state, locale),
            VoiceIntent::NextJump => Self::resolve_next_jump(state, locale),
            VoiceIntent::CopyNextSystem => Self::resolve_copy_next_system(state, locale),
            VoiceIntent::NearestInterstellarFactor => {
                Self::resolve_interstellar_factor(ctx, locale)
            }
            VoiceIntent::NearestBlackMarket => Self::resolve_black_market(ctx, locale),
            VoiceIntent::CrimeStatus => Self::resolve_crime_status(state, ctx, locale),
            VoiceIntent::BioSignals => Self::resolve_bio_signals(ctx, locale),
            VoiceIntent::FuelStatus => Self::resolve_fuel_status(state, locale),
            VoiceIntent::MaterialsSummary => Self::resolve_materials(locale),
            VoiceIntent::Unknown(ref raw) => Self::resolve_unknown(raw, locale),
        }
    }

    fn resolve_ship_status(state: &GameState, locale: &str) -> VoiceCommandResponse {
        let hull = state
            .ship
            .as_ref()
            .and_then(|s| s.hull_health)
            .map(|h| (h * 100.0).round() as i64)
            .unwrap_or(100);

        let shields_up = state
            .status
            .as_ref()
            .map(|s| s.flags.contains(Flags::SHIELDS_UP))
            .unwrap_or(true);

        let fuel_pct = state
            .ship
            .as_ref()
            .and_then(|s| {
                let cur = s.fuel_main?;
                let cap = s.fuel_capacity_main?;
                if cap > 0.0 {
                    Some(((cur / cap) * 100.0).round() as i64)
                } else {
                    None
                }
            })
            .unwrap_or(100);

        let spoken_text = match locale {
            "en" => format!(
                "Hull at {} percent, shields {}, fuel at {} percent.",
                hull,
                if shields_up { "online" } else { "offline" },
                fuel_pct
            ),
            "fr" => format!(
                "Coque à {} pour cent, boucliers {}, carburant à {} pour cent.",
                hull,
                if shields_up { "actifs" } else { "inactifs" },
                fuel_pct
            ),
            "de" => format!(
                "Rumpf bei {} Prozent, Schilde {}, Treibstoff bei {} Prozent.",
                hull,
                if shields_up { "aktiv" } else { "deaktiviert" },
                fuel_pct
            ),
            "pt" => format!(
                "Casco em {} por cento, escudos {}, combustível em {} por cento.",
                hull,
                if shields_up { "ativos" } else { "inativos" },
                fuel_pct
            ),
            _ => format!(
                "Casco al {} por ciento, escudos {}, combustible al {} por ciento.",
                hull,
                if shields_up { "activos" } else { "caídos" },
                fuel_pct
            ),
        };

        VoiceCommandResponse {
            intent: VoiceIntent::ShipStatus,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({
                "hull_pct": hull,
                "shields_up": shields_up,
                "fuel_pct": fuel_pct
            }),
        }
    }

    fn resolve_next_jump(state: &GameState, locale: &str) -> VoiceCommandResponse {
        let next_sys = state
            .route
            .next_system
            .as_deref()
            .or_else(|| state.route.hops.first().map(|h| h.star_system.as_str()));

        let spoken_text = match next_sys {
            Some(sys) => match locale {
                "en" => format!("Next jump in route: {sys}."),
                "fr" => format!("Prochain saut sur votre itinéraire : {sys}."),
                "de" => format!("Nächster Sprung auf der Route: {sys}."),
                "pt" => format!("Próximo salto na rota: {sys}."),
                _ => format!("Siguiente salto en ruta: {sys}."),
            },
            None => match locale {
                "en" => "No navigation route currently plotted.".into(),
                "fr" => "Aucun itinéraire de navigation tracé actuellement.".into(),
                "de" => "Derzeit ist keine Navigationsroute festgelegt.".into(),
                "pt" => "Nenhuma rota de navegação definida no momento.".into(),
                _ => "No hay ninguna ruta de navegación fijada en este momento.".into(),
            },
        };

        VoiceCommandResponse {
            intent: VoiceIntent::NextJump,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({ "next_system": next_sys }),
        }
    }

    fn resolve_copy_next_system(state: &GameState, locale: &str) -> VoiceCommandResponse {
        let next_sys = state
            .route
            .next_system
            .as_deref()
            .or_else(|| state.route.hops.first().map(|h| h.star_system.as_str()));

        match next_sys {
            Some(sys) => {
                let spoken_text = match locale {
                    "en" => format!("System {sys} copied to clipboard."),
                    "fr" => format!("Système {sys} copié dans le presse-papiers."),
                    "de" => format!("System {sys} in die Zwischenablage kopiert."),
                    "pt" => format!("Sistema {sys} copiado para a área de transferência."),
                    _ => format!("Sistema {sys} copiado al portapapeles."),
                };

                VoiceCommandResponse {
                    intent: VoiceIntent::CopyNextSystem,
                    action_executed: Some("clipboard.copy-next-system".into()),
                    spoken_text,
                    clipboard_copied: Some(sys.to_string()),
                    data: serde_json::json!({ "copied_system": sys }),
                }
            }
            None => {
                let spoken_text = match locale {
                    "en" => "No system in route to copy.".into(),
                    "fr" => "Aucun système sur l'itinéraire à copier.".into(),
                    "de" => "Kein System auf der Route zum Kopieren vorhanden.".into(),
                    "pt" => "Nenhum sistema na rota para copiar.".into(),
                    _ => "No hay ningún sistema en ruta para copiar.".into(),
                };

                VoiceCommandResponse {
                    intent: VoiceIntent::CopyNextSystem,
                    action_executed: None,
                    spoken_text,
                    clipboard_copied: None,
                    data: serde_json::Value::Null,
                }
            }
        }
    }

    fn resolve_interstellar_factor(
        ctx: &VoiceResolverContext,
        locale: &str,
    ) -> VoiceCommandResponse {
        if let Some(ref facility) = ctx.nearest_interstellar_factor {
            let spoken_text = match locale {
                "en" => format!(
                    "Nearest Interstellar Factor: {} in system {} at {:.1} light years. System copied to clipboard.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
                "fr" => format!(
                    "Interstellar Factor le plus proche : {} dans le système {} à {:.1} années-lumière. Système copié.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
                "de" => format!(
                    "Nächster Interstellar Factor: {} im System {} in {:.1} Lichtjahren. System kopiert.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
                "pt" => format!(
                    "Interstellar Factor mais próximo: {} no sistema {} a {:.1} anos-luz. Sistema copiado.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
                _ => format!(
                    "Factor interestelar más cercano: {} en sistema {} a {:.1} años luz. Sistema copiado al portapapeles.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
            };

            VoiceCommandResponse {
                intent: VoiceIntent::NearestInterstellarFactor,
                action_executed: Some("crime.copy_system".into()),
                spoken_text,
                clipboard_copied: Some(facility.system_name.clone()),
                data: serde_json::json!({
                    "station": facility.station_name,
                    "system": facility.system_name,
                    "distance_ly": facility.distance_ly
                }),
            }
        } else {
            let spoken_text = match locale {
                "en" => "No Interstellar Factor facilities found in nearby systems.".into(),
                "fr" => "Aucun Interstellar Factor détecté dans les systèmes voisins.".into(),
                "de" => "Kein Interstellar Factor in nahegelegenen Systemen gefunden.".into(),
                "pt" => "Nenhum Interstellar Factor encontrado em sistemas próximos.".into(),
                _ => {
                    "No se han detectado instalaciones de factor interestelar en sistemas cercanos."
                        .into()
                }
            };

            VoiceCommandResponse {
                intent: VoiceIntent::NearestInterstellarFactor,
                action_executed: None,
                spoken_text,
                clipboard_copied: None,
                data: serde_json::Value::Null,
            }
        }
    }

    fn resolve_black_market(ctx: &VoiceResolverContext, locale: &str) -> VoiceCommandResponse {
        if let Some(ref facility) = ctx.nearest_black_market {
            let spoken_text = match locale {
                "en" => format!(
                    "Nearest Black Market: {} in system {} at {:.1} light years. System copied to clipboard.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
                "fr" => format!(
                    "Marché Noir le plus proche : {} dans le système {} à {:.1} années-lumière. Système copié.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
                "de" => format!(
                    "Nächster Schwarzmarkt: {} im System {} in {:.1} Lichtjahren. System kopiert.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
                "pt" => format!(
                    "Mercado Negro mais próximo: {} no sistema {} a {:.1} anos-luz. Sistema copiado.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
                _ => format!(
                    "Mercado negro más cercano: {} en sistema {} a {:.1} años luz. Sistema copiado al portapapeles.",
                    facility.station_name, facility.system_name, facility.distance_ly
                ),
            };

            VoiceCommandResponse {
                intent: VoiceIntent::NearestBlackMarket,
                action_executed: Some("crime.copy_system".into()),
                spoken_text,
                clipboard_copied: Some(facility.system_name.clone()),
                data: serde_json::json!({
                    "station": facility.station_name,
                    "system": facility.system_name,
                    "distance_ly": facility.distance_ly
                }),
            }
        } else {
            let spoken_text = match locale {
                "en" => "No Black Markets detected in nearby systems.".into(),
                "fr" => "Aucun Marché Noir détecté dans les systèmes voisins.".into(),
                "de" => "Kein Schwarzmarkt in nahegelegenen Systemen entdeckt.".into(),
                "pt" => "Nenhum Mercado Negro detectado em sistemas próximos.".into(),
                _ => "No se han detectado mercados negros en sistemas cercanos.".into(),
            };

            VoiceCommandResponse {
                intent: VoiceIntent::NearestBlackMarket,
                action_executed: None,
                spoken_text,
                clipboard_copied: None,
                data: serde_json::Value::Null,
            }
        }
    }

    fn resolve_crime_status(
        state: &GameState,
        ctx: &VoiceResolverContext,
        locale: &str,
    ) -> VoiceCommandResponse {
        let is_wanted = state
            .status
            .as_ref()
            .and_then(|s| s.legal_state.as_deref())
            .map(|ls| ls == "Wanted")
            .unwrap_or(false);

        let spoken_text = if ctx.notoriety > 0 {
            match locale {
                "en" => format!(
                    "Warning: Notoriety level {}. You must fly in space to dissipate it before clearing fines.",
                    ctx.notoriety
                ),
                "fr" => format!(
                    "Alerte : Niveau de notoriété {}. Volez dans l'espace pour la dissiper avant de payer vos amendes.",
                    ctx.notoriety
                ),
                "de" => format!(
                    "Achtung: Notorietätsstufe {}. Bauen Sie sie im Raum ab, bevor Sie Strafen zahlen können.",
                    ctx.notoriety
                ),
                "pt" => format!(
                    "Alerta: Nível de notoriedade {}. Voe no espaço para dissipá-la antes de pagar multas.",
                    ctx.notoriety
                ),
                _ => format!(
                    "Alerta: Nivel de notoriedad {}. Debes volar en el espacio para disiparla antes de poder pagar multas.",
                    ctx.notoriety
                ),
            }
        } else if ctx.total_fines > 0 || ctx.total_bounties > 0 || is_wanted {
            match locale {
                "en" => format!(
                    "Active fines and bounties detected totaling {} credits. Visit an Interstellar Factor.",
                    ctx.total_fines + ctx.total_bounties
                ),
                "fr" => format!(
                    "Amendes et primes actives détectées pour un total de {} crédits. Consultez un Interstellar Factor.",
                    ctx.total_fines + ctx.total_bounties
                ),
                "de" => format!(
                    "Aktive Strafen in Höhe von {} Credits verzeichnet. Besuchen Sie einen Interstellar Factor.",
                    ctx.total_fines + ctx.total_bounties
                ),
                "pt" => format!(
                    "Multas e recompensas ativas totalizando {} créditos. Visite um Interstellar Factor.",
                    ctx.total_fines + ctx.total_bounties
                ),
                _ => format!(
                    "Multas y recompensas activas por un total de {} créditos. Puedes acudir a un factor interestelar.",
                    ctx.total_fines + ctx.total_bounties
                ),
            }
        } else {
            match locale {
                "en" => "Clean record. No outstanding fines or bounties on file.".into(),
                "fr" => "Casier vierge. Aucune amende ni prime en cours.".into(),
                "de" => "Saubere Akte. Keine offenen Bußgelder oder Kopfgelder verzeichnet.".into(),
                "pt" => "Ficha limpa. Nenhuma multa ou recompensa registrada.".into(),
                _ => "Historial limpio. Sin multas pendientes ni recompensas por captura.".into(),
            }
        };

        VoiceCommandResponse {
            intent: VoiceIntent::CrimeStatus,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({
                "notoriety": ctx.notoriety,
                "total_fines": ctx.total_fines,
                "total_bounties": ctx.total_bounties,
                "is_wanted": is_wanted
            }),
        }
    }

    fn resolve_bio_signals(ctx: &VoiceResolverContext, locale: &str) -> VoiceCommandResponse {
        let count = ctx.bio_signal_bodies_count;
        let spoken_text = if count > 0 {
            match locale {
                "en" => format!(
                    "Biological signals detected on {} bodies in current system.",
                    count
                ),
                "fr" => format!(
                    "Signaux biologiques détectés sur {} corps dans le système actuel.",
                    count
                ),
                "de" => format!(
                    "Biologische Signale auf {} Himmelskörpern im aktuellen System entdeckt.",
                    count
                ),
                "pt" => format!(
                    "Sinais biológicos detectados em {} corpos no sistema atual.",
                    count
                ),
                _ => format!(
                    "Señales biológicas detectadas en {} cuerpos del sistema actual.",
                    count
                ),
            }
        } else {
            match locale {
                "en" => "No biological signals reported in the current system.".into(),
                "fr" => "Aucun signal biologique signalé dans le système actuel.".into(),
                "de" => "Keine biologischen Signale im aktuellen System gemeldet.".into(),
                "pt" => "Nenhum sinal biológico relatado no sistema atual.".into(),
                _ => "No se han detectado señales biológicas en el sistema actual.".into(),
            }
        };

        VoiceCommandResponse {
            intent: VoiceIntent::BioSignals,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({ "bio_bodies_count": count }),
        }
    }

    fn resolve_fuel_status(state: &GameState, locale: &str) -> VoiceCommandResponse {
        let cur = state.ship.as_ref().and_then(|s| s.fuel_main).unwrap_or(0.0);
        let cap = state
            .ship
            .as_ref()
            .and_then(|s| s.fuel_capacity_main)
            .unwrap_or(0.0);
        let pct = if cap > 0.0 {
            ((cur / cap) * 100.0).round() as i64
        } else {
            100
        };

        let spoken_text = match locale {
            "en" => format!(
                "Fuel at {} percent, {:.1} tons available in main tank.",
                pct, cur
            ),
            "fr" => format!(
                "Carburant à {} pour cent, {:.1} tonnes disponibles dans le réservoir.",
                pct, cur
            ),
            "de" => format!(
                "Treibstoff bei {} Prozent, {:.1} Tonnen im Haupttank verfügbar.",
                pct, cur
            ),
            "pt" => format!(
                "Combustível em {} por cento, {:.1} toneladas disponíveis no tanque.",
                pct, cur
            ),
            _ => format!(
                "Combustible al {} por ciento, {:.1} toneladas disponibles en el tanque principal.",
                pct, cur
            ),
        };

        VoiceCommandResponse {
            intent: VoiceIntent::FuelStatus,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({
                "fuel_current": cur,
                "fuel_capacity": cap,
                "fuel_pct": pct
            }),
        }
    }

    fn resolve_materials(locale: &str) -> VoiceCommandResponse {
        let spoken_text = match locale {
            "en" => {
                "Engineering materials database and inventory synchronized in companion.".into()
            }
            "fr" => {
                "Base de données des matériaux d'ingénierie synchronisée dans le compagnon.".into()
            }
            "de" => "Materialdatenbank und Inventar im Begleiter synchronisiert.".into(),
            "pt" => "Banco de dados e inventário de materiais sincronizados no assistente.".into(),
            _ => "Base de datos e inventario de materiales de ingeniería sincronizados.".into(),
        };

        VoiceCommandResponse {
            intent: VoiceIntent::MaterialsSummary,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::Value::Null,
        }
    }

    fn resolve_unknown(raw: &str, locale: &str) -> VoiceCommandResponse {
        let spoken_text = match locale {
            "en" => "Query not recognized. Try asking for ship status, next jump, or nearest factor.".into(),
            "fr" => "Requête non reconnue. Essayez de demander l'état du vaisseau ou le prochain saut.".into(),
            "de" => "Befehl nicht erkannt. Fragen Sie nach dem Schiffstatus oder dem nächsten Sprung.".into(),
            "pt" => "Comando não reconhecido. Tente perguntar pelo estado da nave ou próximo salto.".into(),
            _ => "Consulta no reconocida. Prueba preguntando por el estado de la nave o el siguiente salto.".into(),
        };

        VoiceCommandResponse {
            intent: VoiceIntent::Unknown(raw.to_string()),
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::companion::RouteHop;
    use tmc_model::state::{RouteState, ShipState};

    #[test]
    fn parse_intent_recognizes_all_categories_in_spanish() {
        assert_eq!(parse_intent("¿Estado de la nave?"), VoiceIntent::ShipStatus);
        assert_eq!(parse_intent("informe de nave"), VoiceIntent::ShipStatus);
        assert_eq!(
            parse_intent("¿Cuál es el siguiente salto?"),
            VoiceIntent::NextJump
        );
        assert_eq!(
            parse_intent("copia el siguiente sistema"),
            VoiceIntent::CopyNextSystem
        );
        assert_eq!(
            parse_intent("¿Dónde está el factor interestelar más cercano?"),
            VoiceIntent::NearestInterstellarFactor
        );
        assert_eq!(
            parse_intent("¿Dónde hay un mercado negro?"),
            VoiceIntent::NearestBlackMarket
        );
        assert_eq!(parse_intent("¿Tengo multas?"), VoiceIntent::CrimeStatus);
        assert_eq!(
            parse_intent("¿Hay bio-señales en el sistema?"),
            VoiceIntent::BioSignals
        );
        assert_eq!(
            parse_intent("¿Cuánto combustible me queda?"),
            VoiceIntent::FuelStatus
        );
        assert_eq!(
            parse_intent("¿Qué materiales me faltan?"),
            VoiceIntent::MaterialsSummary
        );
    }

    #[test]
    fn parse_intent_recognizes_queries_in_english_and_other_languages() {
        assert_eq!(parse_intent("ship status report"), VoiceIntent::ShipStatus);
        assert_eq!(parse_intent("next jump please"), VoiceIntent::NextJump);
        assert_eq!(
            parse_intent("copy next system"),
            VoiceIntent::CopyNextSystem
        );
        assert_eq!(
            parse_intent("nearest interstellar factor"),
            VoiceIntent::NearestInterstellarFactor
        );
        assert_eq!(
            parse_intent("nearest black market"),
            VoiceIntent::NearestBlackMarket
        );
        assert_eq!(
            parse_intent("do I have any bounties"),
            VoiceIntent::CrimeStatus
        );
        assert_eq!(
            parse_intent("any biological signals here"),
            VoiceIntent::BioSignals
        );
        assert_eq!(parse_intent("fuel level"), VoiceIntent::FuelStatus);

        // Francés y Alemán
        assert_eq!(parse_intent("état du vaisseau"), VoiceIntent::ShipStatus);
        assert_eq!(parse_intent("prochain saut"), VoiceIntent::NextJump);
        assert_eq!(parse_intent("schiffstatus"), VoiceIntent::ShipStatus);
        assert_eq!(parse_intent("nachster sprung"), VoiceIntent::NextJump);
    }

    #[test]
    fn resolver_resolves_ship_status_and_fuel() {
        let state = GameState {
            ship: Some(ShipState {
                hull_health: Some(0.85),
                fuel_main: Some(24.0),
                fuel_capacity_main: Some(32.0),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ctx = VoiceResolverContext {
            locale: "es".into(),
            ..Default::default()
        };

        let res = VoiceIntentResolver::resolve("¿Estado de la nave?", &state, &ctx);
        assert_eq!(res.intent, VoiceIntent::ShipStatus);
        assert!(res.spoken_text.contains("85 por ciento"));
        assert!(res.spoken_text.contains("75 por ciento"));

        let res_fuel = VoiceIntentResolver::resolve("combustible", &state, &ctx);
        assert_eq!(res_fuel.intent, VoiceIntent::FuelStatus);
        assert!(res_fuel.spoken_text.contains("24.0 toneladas"));
    }

    #[test]
    fn resolver_resolves_next_jump_and_copy() {
        let state = GameState {
            route: RouteState {
                hops: vec![RouteHop {
                    system_address: 12345,
                    star_system: "Sol".into(),
                    star_pos: [0.0, 0.0, 0.0],
                    star_class: Some("G".into()),
                }],
                next_system: Some("Sol".into()),
                remaining_jumps: Some(1),
            },
            ..Default::default()
        };

        let ctx = VoiceResolverContext {
            locale: "es".into(),
            ..Default::default()
        };

        let res_jump = VoiceIntentResolver::resolve("siguiente salto", &state, &ctx);
        assert_eq!(res_jump.intent, VoiceIntent::NextJump);
        assert!(res_jump.spoken_text.contains("Sol"));

        let res_copy = VoiceIntentResolver::resolve("copiar siguiente sistema", &state, &ctx);
        assert_eq!(res_copy.intent, VoiceIntent::CopyNextSystem);
        assert_eq!(res_copy.clipboard_copied.as_deref(), Some("Sol"));
        assert_eq!(
            res_copy.action_executed.as_deref(),
            Some("clipboard.copy-next-system")
        );
    }

    #[test]
    fn resolver_resolves_nearest_interstellar_factor_with_copy() {
        let state = GameState::default();
        let ctx = VoiceResolverContext {
            locale: "es".into(),
            nearest_interstellar_factor: Some(NearbyFacility {
                station_name: "Baird Gateway".into(),
                system_name: "LTT 1873".into(),
                distance_ly: 8.42,
                pad_size: "L".into(),
                is_planetary: false,
            }),
            ..Default::default()
        };

        let res = VoiceIntentResolver::resolve("factor interestelar", &state, &ctx);
        assert_eq!(res.intent, VoiceIntent::NearestInterstellarFactor);
        assert!(res.spoken_text.contains("Baird Gateway"));
        assert!(res.spoken_text.contains("LTT 1873"));
        assert_eq!(res.clipboard_copied.as_deref(), Some("LTT 1873"));
        assert_eq!(res.action_executed.as_deref(), Some("crime.copy_system"));
    }
}
