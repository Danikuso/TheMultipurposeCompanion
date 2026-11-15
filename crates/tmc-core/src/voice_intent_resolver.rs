//! Intérprete semántico de intenciones de voz contra el estado de juego (Fase 10).

use tmc_model::crime::NearbyFacility;
use tmc_model::state::GameState;
use tmc_model::status::Flags;
use tmc_model::voice_intent::{VoiceCommandResponse, VoiceIntent};

fn speech_decimal(value: f64, locale: &str) -> String {
    let number = format!("{value:.1}");
    if locale == "en" {
        number
    } else {
        number.replace('.', ",")
    }
}

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
    pub exploration_bodies_scanned: u32,
    pub exploration_bodies_total: u32,
    pub exploration_valuable_count: u32,
    pub exploration_value_now: u64,
    pub combat_kills: u32,
    pub combat_unredeemed_total: u64,
    pub carrier_name: Option<String>,
    pub carrier_system: Option<String>,
    pub carrier_fuel_depot: Option<u32>,
    pub carrier_max_jumps: Option<u32>,
    pub colonisation_station: Option<String>,
    pub colonisation_progress_pct: Option<u32>,
    pub colonisation_remaining_tons: Option<u64>,
    pub colonisation_top_missing: Option<String>,
    pub pinned_goals_total: u32,
    pub pinned_goals_ready: u32,
    pub inventory_materials_total: u32,
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
            '-' => normalized.push(' '),
            '?' | '¿' | '!' | '¡' | '.' | ',' | ';' | ':' | '"' | '\'' => {}
            _ => normalized.push(ch),
        }
    }

    normalized
}

fn wants_clipboard_copy(q: &str) -> bool {
    q.contains("copi") || q.contains("copy") || q.contains("kopier")
}

/// Analiza la cadena normalizada y determina la intención principal.
pub fn parse_intent(query: &str) -> VoiceIntent {
    let q = normalize_query(query);

    // 1. Ubicación actual / copiar sistema actual (prioritario frente a siguiente sistema)
    if q.contains("donde estoy")
        || q.contains("sistema actual")
        || q.contains("ubicacion actual")
        || q.contains("posicion actual")
        || q.contains("where am i")
        || q.contains("current system")
        || q.contains("current location")
        || q.contains("ou suis je")
        || q.contains("systeme actuel")
        || q.contains("position actuelle")
        || q.contains("wo bin ich")
        || q.contains("aktuelles system")
        || q.contains("aktueller standort")
        || q.contains("onde estou")
        || q.contains("localizacao atual")
    {
        return VoiceIntent::CurrentLocation;
    }

    // 2. Acciones de copia del siguiente salto priorizadas
    if wants_clipboard_copy(&q)
        && (q.contains("siguiente")
            || q.contains("proximo")
            || q.contains("salto")
            || q.contains("sistema")
            || q.contains("next")
            || q.contains("jump")
            || q.contains("sprung")
            || q.contains("saut")
            || q.contains("prochain")
            || q.contains("systeme"))
    {
        return VoiceIntent::CopyNextSystem;
    }

    // 3. Estado del Portanaves (antes de ShipStatus para evitar colisión con "estado")
    if q.contains("portanaves")
        || q.contains("porta naves")
        || q.contains("fleet carrier")
        || q.contains("carrier")
        || q.contains("porte vaisseaux")
        || q.contains("porte-vaisseaux")
        || q.contains("flottentrager")
        || q.contains("tritio")
        || q.contains("tritium")
    {
        return VoiceIntent::CarrierStatus;
    }

    // 4. Colonización y construcción activa
    if q.contains("colonizacion")
        || q.contains("colonisation")
        || q.contains("colonization")
        || q.contains("kolonisierung")
        || q.contains("colonizacao")
        || q.contains("deposito colonial")
        || q.contains("proyecto colonial")
    {
        return VoiceIntent::ColonisationStatus;
    }

    // 5. Interstellar Factor
    if q.contains("facteur")
        || q.contains("interstellaire")
        || q.contains("interestelar")
        || q.contains("interstellarer vertreter")
        || q.contains("factor")
        || q.contains("interstellar")
        || q.contains("fator")
        || (q.contains("limpiar") && (q.contains("multa") || q.contains("recompensa")))
        || (q.contains("pagar") && (q.contains("multa") || q.contains("recompensa")))
        || (q.contains("clear") && (q.contains("fine") || q.contains("bount")))
        || (q.contains("pay") && (q.contains("fine") || q.contains("bount")))
    {
        return VoiceIntent::NearestInterstellarFactor;
    }

    // 6. Mercado Negro
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

    // 7. Resumen de combate y vales sin cobrar (antes de CrimeStatus para separar "combat/vouchers")
    if q.contains("combate")
        || q.contains("combat")
        || q.contains("bajas")
        || q.contains("kills")
        || q.contains("abates")
        || q.contains("vales de recompensa")
        || q.contains("bonos de combate")
        || q.contains("bounty voucher")
        || q.contains("combat bond")
        || q.contains("pendientes de cobro")
        || q.contains("unredeemed")
        || q.contains("primes de combat")
        || q.contains("kampf")
    {
        return VoiceIntent::CombatSummary;
    }

    // 8. Crimen y multas
    if q.contains("multa")
        || q.contains("recompensa")
        || q.contains("notoriedad")
        || q.contains("notoriety")
        || q.contains("buscado")
        || q.contains("wanted")
        || q.contains("notoriete")
        || q.contains("fine")
        || q.contains("bount")
        || q.contains("gesucht")
        || q.contains("recherche")
        || q.contains("procurado")
        || q.contains("estado legal")
        || q.contains("legal status")
    {
        return VoiceIntent::CrimeStatus;
    }

    // 9. Siguiente salto / ruta
    if q.contains("siguiente salto")
        || q.contains("proximo salto")
        || q.contains("siguiente sistema")
        || q.contains("proximo sistema")
        || q.contains("next jump")
        || q.contains("next system")
        || q.contains("prochain saut")
        || q.contains("nachster sprung")
        || q.contains("a donde vamos")
        || q.contains("donde vamos")
        || q.contains("destino")
        || q.contains("destination")
    {
        return VoiceIntent::NextJump;
    }

    // 10. Resumen de exploración y cuerpos valiosos
    if q.contains("exploracion")
        || q.contains("exploration")
        || q.contains("erkundung")
        || q.contains("exploracao")
        || q.contains("cuerpos valiosos")
        || q.contains("valor del sistema")
        || q.contains("system value")
        || q.contains("valuable bodies")
        || q.contains("escaner fss")
        || q.contains("system scan")
        || q.contains("corps precieux")
        || q.contains("wertvolle")
        || q.contains("corpos valiosos")
    {
        return VoiceIntent::ExplorationSummary;
    }

    // 11. Bio-señales y exobiología
    if q.contains("bio")
        || q.contains("biolog")
        || q.contains("exobiolog")
        || q.contains("senales de vida")
    {
        return VoiceIntent::BioSignals;
    }

    // 12. Combustible
    if q.contains("combustivel")
        || q.contains("combustible")
        || q.contains("fuel")
        || q.contains("carburant")
        || q.contains("treibstoff")
        || q.contains("gasolina")
        || q.contains("tanque")
    {
        return VoiceIntent::FuelStatus;
    }

    // 13. Materiales de ingeniería
    if q.contains("material")
        || q.contains("materiaux")
        || q.contains("materialien")
        || q.contains("recetas fijadas")
        || q.contains("pinned")
        || q.contains("ingenier")
    {
        return VoiceIntent::MaterialsSummary;
    }

    // 14. Estado de nave
    if q.contains("estado")
        || q.contains("nave")
        || q.contains("ship")
        || q.contains("vaisseau")
        || q.contains("schiff")
        || q.contains("casco")
        || q.contains("hull")
        || q.contains("escudo")
        || q.contains("bouclier")
        || q.contains("schilde")
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
        let norm = normalize_query(query);

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
            VoiceIntent::MaterialsSummary => Self::resolve_materials(ctx, locale),
            VoiceIntent::CurrentLocation => Self::resolve_current_location(&norm, state, locale),
            VoiceIntent::ExplorationSummary => {
                Self::resolve_exploration_summary(state, ctx, locale)
            }
            VoiceIntent::CombatSummary => Self::resolve_combat_summary(ctx, locale),
            VoiceIntent::CarrierStatus => Self::resolve_carrier_status(ctx, locale),
            VoiceIntent::ColonisationStatus => Self::resolve_colonisation_status(ctx, locale),
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
                    "Nearest Interstellar Factor: {} in system {} at {} light years. System copied to clipboard.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
                ),
                "fr" => format!(
                    "Interstellar Factor le plus proche : {} dans le système {} à {} années-lumière. Système copié.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
                ),
                "de" => format!(
                    "Nächster interstellarer Vertreter: {} im System {} in {} Lichtjahren. System kopiert.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
                ),
                "pt" => format!(
                    "Agente interestelar mais próximo: {} no sistema {} a {} anos-luz. Sistema copiado.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
                ),
                _ => format!(
                    "Factor interestelar más cercano: {} en sistema {} a {} años luz. Sistema copiado al portapapeles.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
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
                "de" => "Kein interstellarer Vertreter in nahegelegenen Systemen gefunden.".into(),
                "pt" => "Nenhum agente interestelar encontrado em sistemas próximos.".into(),
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
                    "Nearest Black Market: {} in system {} at {} light years. System copied to clipboard.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
                ),
                "fr" => format!(
                    "Marché Noir le plus proche : {} dans le système {} à {} années-lumière. Système copié.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
                ),
                "de" => format!(
                    "Nächster Schwarzmarkt: {} im System {} in {} Lichtjahren. System kopiert.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
                ),
                "pt" => format!(
                    "Mercado Negro mais próximo: {} no sistema {} a {} anos-luz. Sistema copiado.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
                ),
                _ => format!(
                    "Mercado negro más cercano: {} en sistema {} a {} años luz. Sistema copiado al portapapeles.",
                    facility.station_name,
                    facility.system_name,
                    speech_decimal(facility.distance_ly, locale)
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
                    "Aktive Strafen in Höhe von {} Credits verzeichnet. Besuchen Sie einen interstellaren Vertreter.",
                    ctx.total_fines + ctx.total_bounties
                ),
                "pt" => format!(
                    "Multas e recompensas ativas totalizando {} créditos. Visite um agente interestelar.",
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
                "Fuel at {} percent, {} tonnes available in main tank.",
                pct,
                speech_decimal(cur, locale)
            ),
            "fr" => format!(
                "Carburant à {} pour cent, {} tonnes disponibles dans le réservoir.",
                pct,
                speech_decimal(cur, locale)
            ),
            "de" => format!(
                "Treibstoff bei {} Prozent, {} Tonnen im Haupttank verfügbar.",
                pct,
                speech_decimal(cur, locale)
            ),
            "pt" => format!(
                "Combustível em {} por cento, {} toneladas disponíveis no tanque.",
                pct,
                speech_decimal(cur, locale)
            ),
            _ => format!(
                "Combustible al {} por ciento, {} toneladas disponibles en el tanque principal.",
                pct,
                speech_decimal(cur, locale)
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

    fn resolve_materials(ctx: &VoiceResolverContext, locale: &str) -> VoiceCommandResponse {
        let spoken_text = if ctx.pinned_goals_total > 0 {
            let pending = ctx
                .pinned_goals_total
                .saturating_sub(ctx.pinned_goals_ready);
            match locale {
                "en" => format!(
                    "You have {} pinned engineering goals ({} ready, {} missing materials) and {} units in inventory.",
                    ctx.pinned_goals_total,
                    ctx.pinned_goals_ready,
                    pending,
                    ctx.inventory_materials_total
                ),
                "fr" => format!(
                    "{} objectifs d'ingénierie épinglés ({} prêts, {} incomplets) et {} matériaux en stock.",
                    ctx.pinned_goals_total,
                    ctx.pinned_goals_ready,
                    pending,
                    ctx.inventory_materials_total
                ),
                "de" => format!(
                    "{} angeheftete Ingenieursziele ({} bereit, {} fehlend) und {} Materialien im Inventar.",
                    ctx.pinned_goals_total,
                    ctx.pinned_goals_ready,
                    pending,
                    ctx.inventory_materials_total
                ),
                "pt" => format!(
                    "{} metas de engenharia fixadas ({} prontas, {} pendentes) e {} materiais no inventário.",
                    ctx.pinned_goals_total,
                    ctx.pinned_goals_ready,
                    pending,
                    ctx.inventory_materials_total
                ),
                _ => format!(
                    "Tienes {} recetas fijadas de ingeniería ({} listas y {} con materiales pendientes) y {} unidades en inventario.",
                    ctx.pinned_goals_total,
                    ctx.pinned_goals_ready,
                    pending,
                    ctx.inventory_materials_total
                ),
            }
        } else if ctx.inventory_materials_total > 0 {
            match locale {
                "en" => format!(
                    "No pinned engineering goals. You have {} material units recorded in inventory.",
                    ctx.inventory_materials_total
                ),
                "fr" => format!(
                    "Aucun objectif épinglé. {} matériaux enregistrés dans votre inventaire.",
                    ctx.inventory_materials_total
                ),
                "de" => format!(
                    "Keine angehefteten Ziele. {} Materialien im Inventar erfasst.",
                    ctx.inventory_materials_total
                ),
                "pt" => format!(
                    "Nenhuma meta fixada. {} unidades de materiais registradas no inventário.",
                    ctx.inventory_materials_total
                ),
                _ => format!(
                    "Sin recetas fijadas de ingeniería. Tienes {} unidades de materiales registradas en inventario.",
                    ctx.inventory_materials_total
                ),
            }
        } else {
            match locale {
                "en" => {
                    "Consult the engineering materials database and recorded inventory in the companion."
                        .into()
                }
                "fr" => {
                    "Consultez la base de données des matériaux et l’inventaire enregistré dans le compagnon."
                        .into()
                }
                "de" => {
                    "Prüfen Sie die Materialdatenbank und das erfasste Inventar im Begleiter.".into()
                }
                "pt" => {
                    "Consulte o banco de dados e o inventário registrado de materiais no assistente."
                        .into()
                }
                _ => {
                    "Consulta la base de datos y el inventario registrado de materiales de ingeniería."
                        .into()
                }
            }
        };

        VoiceCommandResponse {
            intent: VoiceIntent::MaterialsSummary,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({
                "pinned_goals_total": ctx.pinned_goals_total,
                "pinned_goals_ready": ctx.pinned_goals_ready,
                "inventory_materials_total": ctx.inventory_materials_total
            }),
        }
    }

    fn resolve_current_location(
        query_norm: &str,
        state: &GameState,
        locale: &str,
    ) -> VoiceCommandResponse {
        let system = state.location.system.trim();
        let copy = wants_clipboard_copy(query_norm);
        if system.is_empty() {
            let spoken_text = match locale {
                "en" => "Current star system not detected yet.".into(),
                "fr" => "Système stellaire actuel pas encore détecté.".into(),
                "de" => "Aktuelles Sternensystem noch nicht erkannt.".into(),
                "pt" => "Sistema estelar atual ainda não detectado.".into(),
                _ => "Sistema estelar actual aún no detectado.".into(),
            };
            return VoiceCommandResponse {
                intent: VoiceIntent::CurrentLocation,
                action_executed: None,
                spoken_text,
                clipboard_copied: None,
                data: serde_json::Value::Null,
            };
        }

        let place = state
            .location
            .station
            .as_deref()
            .or(state.location.body.as_deref());
        let spoken_text = match (locale, place, copy) {
            ("en", Some(p), true) => {
                format!("Current location: {p} in system {system}. System copied to clipboard.")
            }
            ("en", None, true) => {
                format!("Current system: {system}. System copied to clipboard.")
            }
            ("en", Some(p), false) => format!("Current location: {p} in system {system}."),
            ("en", None, false) => format!("Current system: {system}."),
            ("fr", Some(p), true) => {
                format!("Position actuelle : {p} dans le système {system}. Système copié.")
            }
            ("fr", None, true) => format!("Système actuel : {system}. Système copié."),
            ("fr", Some(p), false) => format!("Position actuelle : {p} dans le système {system}."),
            ("fr", None, false) => format!("Système actuel : {system}."),
            ("de", Some(p), true) => {
                format!("Aktueller Standort: {p} im System {system}. System kopiert.")
            }
            ("de", None, true) => format!("Aktuelles System: {system}. System kopiert."),
            ("de", Some(p), false) => format!("Aktueller Standort: {p} im System {system}."),
            ("de", None, false) => format!("Aktuelles System: {system}."),
            ("pt", Some(p), true) => {
                format!("Localização atual: {p} no sistema {system}. Sistema copiado.")
            }
            ("pt", None, true) => format!("Sistema atual: {system}. Sistema copiado."),
            ("pt", Some(p), false) => format!("Localização atual: {p} no sistema {system}."),
            ("pt", None, false) => format!("Sistema atual: {system}."),
            (_, Some(p), true) => format!(
                "Ubicación actual: {p} en el sistema {system}. Sistema copiado al portapapeles."
            ),
            (_, None, true) => {
                format!("Sistema actual: {system}. Sistema copiado al portapapeles.")
            }
            (_, Some(p), false) => format!("Ubicación actual: {p} en el sistema {system}."),
            (_, None, false) => format!("Sistema actual: {system}."),
        };

        VoiceCommandResponse {
            intent: VoiceIntent::CurrentLocation,
            action_executed: copy.then(|| "clipboard.copy-current-system".to_owned()),
            spoken_text,
            clipboard_copied: copy.then(|| system.to_owned()),
            data: serde_json::json!({
                "system": system,
                "body": state.location.body,
                "station": state.location.station,
            }),
        }
    }

    fn resolve_exploration_summary(
        state: &GameState,
        ctx: &VoiceResolverContext,
        locale: &str,
    ) -> VoiceCommandResponse {
        let sys = state.location.system.trim();
        let spoken_text = if ctx.exploration_bodies_total > 0 || ctx.exploration_bodies_scanned > 0
        {
            let total_disp = ctx
                .exploration_bodies_total
                .max(ctx.exploration_bodies_scanned);
            match locale {
                "en" => format!(
                    "Exploration in {}: {} of {} bodies scanned, {} valuable bodies, estimated value {} credits.",
                    if sys.is_empty() {
                        "current system"
                    } else {
                        sys
                    },
                    ctx.exploration_bodies_scanned,
                    total_disp,
                    ctx.exploration_valuable_count,
                    ctx.exploration_value_now
                ),
                "fr" => format!(
                    "Exploration dans {} : {} sur {} corps scannés, {} précieux, valeur estimée {} crédits.",
                    if sys.is_empty() {
                        "le système actuel"
                    } else {
                        sys
                    },
                    ctx.exploration_bodies_scanned,
                    total_disp,
                    ctx.exploration_valuable_count,
                    ctx.exploration_value_now
                ),
                "de" => format!(
                    "Erkundung in {}: {} von {} Körpern gescannt, {} wertvoll, geschätzter Wert {} Credits.",
                    if sys.is_empty() {
                        "aktuelles System"
                    } else {
                        sys
                    },
                    ctx.exploration_bodies_scanned,
                    total_disp,
                    ctx.exploration_valuable_count,
                    ctx.exploration_value_now
                ),
                "pt" => format!(
                    "Exploração em {}: {} de {} corpos escaneados, {} valiosos, valor estimado {} créditos.",
                    if sys.is_empty() { "sistema atual" } else { sys },
                    ctx.exploration_bodies_scanned,
                    total_disp,
                    ctx.exploration_valuable_count,
                    ctx.exploration_value_now
                ),
                _ => format!(
                    "Exploración en {}: {} de {} cuerpos escaneados, {} objetivos valiosos, valor estimado {} créditos.",
                    if sys.is_empty() {
                        "el sistema actual"
                    } else {
                        sys
                    },
                    ctx.exploration_bodies_scanned,
                    total_disp,
                    ctx.exploration_valuable_count,
                    ctx.exploration_value_now
                ),
            }
        } else {
            match locale {
                "en" => "No scanned exploration bodies recorded in the current system.".into(),
                "fr" => "Aucun corps scanné enregistré dans le système actuel.".into(),
                "de" => "Keine gescannten Himmelskörper im aktuellen System verzeichnet.".into(),
                "pt" => "Nenhum corpo escaneado registrado no sistema atual.".into(),
                _ => "Sin cuerpos escaneados registrados en el sistema actual.".into(),
            }
        };

        VoiceCommandResponse {
            intent: VoiceIntent::ExplorationSummary,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({
                "bodies_scanned": ctx.exploration_bodies_scanned,
                "bodies_total": ctx.exploration_bodies_total,
                "valuable_count": ctx.exploration_valuable_count,
                "value_now": ctx.exploration_value_now,
            }),
        }
    }

    fn resolve_combat_summary(ctx: &VoiceResolverContext, locale: &str) -> VoiceCommandResponse {
        let spoken_text = match locale {
            "en" => format!(
                "Combat session: {} kills confirmed, {} credits in unredeemed vouchers.",
                ctx.combat_kills, ctx.combat_unredeemed_total
            ),
            "fr" => format!(
                "Session de combat : {} cibles détruites, {} crédits en bons non encaissés.",
                ctx.combat_kills, ctx.combat_unredeemed_total
            ),
            "de" => format!(
                "Kampfsitzung: {} Abschüsse bestätigt, {} Credits an offenen Belohnungen.",
                ctx.combat_kills, ctx.combat_unredeemed_total
            ),
            "pt" => format!(
                "Sessão de combate: {} abates confirmados, {} créditos em vales não resgatados.",
                ctx.combat_kills, ctx.combat_unredeemed_total
            ),
            _ => format!(
                "Sesión de combate: {} bajas confirmadas y {} créditos en vales pendientes de cobro.",
                ctx.combat_kills, ctx.combat_unredeemed_total
            ),
        };

        VoiceCommandResponse {
            intent: VoiceIntent::CombatSummary,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({
                "kills": ctx.combat_kills,
                "unredeemed_total": ctx.combat_unredeemed_total,
            }),
        }
    }

    fn resolve_carrier_status(ctx: &VoiceResolverContext, locale: &str) -> VoiceCommandResponse {
        let spoken_text = if let Some(ref sys) = ctx.carrier_system {
            let name = ctx.carrier_name.as_deref().unwrap_or("Fleet Carrier");
            let fuel = ctx.carrier_fuel_depot.unwrap_or(0);
            let jumps = ctx.carrier_max_jumps.unwrap_or(0);
            match locale {
                "en" => format!(
                    "Fleet Carrier {name} is in {sys} with {fuel} tonnes of tritium ({jumps} jumps available)."
                ),
                "fr" => format!(
                    "Porte-vaisseaux {name} situé à {sys} avec {fuel} tonnes de tritium ({jumps} sauts disponibles)."
                ),
                "de" => format!(
                    "Flottenträger {name} befindet sich in {sys} mit {fuel} Tonnen Tritium ({jumps} Sprünge möglich)."
                ),
                "pt" => format!(
                    "Porta-naves {name} está em {sys} com {fuel} toneladas de trítio ({jumps} saltos disponíveis)."
                ),
                _ => format!(
                    "Portanaves {name} en el sistema {sys}, con {fuel} toneladas de tritio y autonomía para {jumps} saltos."
                ),
            }
        } else {
            match locale {
                "en" => "No Fleet Carrier telemetry available yet.".into(),
                "fr" => "Aucune télémétrie de porte-vaisseaux disponible pour le moment.".into(),
                "de" => "Derzeit keine Flottenträger-Telemetrie verfügbar.".into(),
                "pt" => "Nenhuma telemetria de porta-naves disponível no momento.".into(),
                _ => "Sin telemetría de portanaves disponible en este momento.".into(),
            }
        };

        VoiceCommandResponse {
            intent: VoiceIntent::CarrierStatus,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({
                "carrier_name": ctx.carrier_name,
                "carrier_system": ctx.carrier_system,
                "fuel_depot": ctx.carrier_fuel_depot,
                "max_jumps_possible": ctx.carrier_max_jumps,
            }),
        }
    }

    fn resolve_colonisation_status(
        ctx: &VoiceResolverContext,
        locale: &str,
    ) -> VoiceCommandResponse {
        let spoken_text = if let Some(pct) = ctx.colonisation_progress_pct {
            let station = ctx.colonisation_station.as_deref().unwrap_or("depot");
            let remaining = ctx.colonisation_remaining_tons.unwrap_or(0);
            let top = ctx.colonisation_top_missing.as_deref().unwrap_or("—");
            match locale {
                "en" => format!(
                    "Colonisation project at {station} is {pct} percent complete, {remaining} tonnes remaining. Top priority: {top}."
                ),
                "fr" => format!(
                    "Projet de colonisation à {station} complété à {pct} pour cent, {remaining} tonnes restantes. Priorité : {top}."
                ),
                "de" => format!(
                    "Kolonisierungsprojekt bei {station} zu {pct} Prozent abgeschlossen, {remaining} Tonnen offen. Priorität: {top}."
                ),
                "pt" => format!(
                    "Projeto de colonização em {station} em {pct} por cento, {remaining} toneladas restantes. Prioridade: {top}."
                ),
                _ => format!(
                    "Proyecto de colonización en {station} al {pct} por ciento, faltan {remaining} toneladas. Prioritario: {top}."
                ),
            }
        } else {
            match locale {
                "en" => "No active colonisation construction project detected.".into(),
                "fr" => "Aucun projet de colonisation actif détecté.".into(),
                "de" => "Kein aktives Kolonisierungsprojekt erkannt.".into(),
                "pt" => "Nenhum projeto de colonização ativo detectado.".into(),
                _ => "No hay ningún proyecto activo de colonización detectado.".into(),
            }
        };

        VoiceCommandResponse {
            intent: VoiceIntent::ColonisationStatus,
            action_executed: None,
            spoken_text,
            clipboard_copied: None,
            data: serde_json::json!({
                "station": ctx.colonisation_station,
                "progress_pct": ctx.colonisation_progress_pct,
                "remaining_tons": ctx.colonisation_remaining_tons,
                "top_missing": ctx.colonisation_top_missing,
            }),
        }
    }

    fn resolve_unknown(raw: &str, locale: &str) -> VoiceCommandResponse {
        let spoken_text = match locale {
            "en" => "Query not recognized. Try asking for ship status, current system, exploration, combat, carrier, or nearest factor.".into(),
            "fr" => "Requête non reconnue. Essayez de demander l'état du vaisseau, votre position, l'exploration ou le prochain saut.".into(),
            "de" => "Befehl nicht erkannt. Fragen Sie nach Schiffstatus, aktuellem System, Erkundung, Kampf oder dem nächsten Sprung.".into(),
            "pt" => "Comando não reconhecido. Tente perguntar pelo estado da nave, sistema atual, exploração ou próximo salto.".into(),
            _ => "Consulta no reconocida. Prueba preguntando por estado de la nave, dónde estoy, exploración, combate, portanaves o siguiente salto.".into(),
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
    fn localized_queries_match_the_same_intent() {
        for query in [
            "facteur interstellaire",
            "agente interestelar",
            "interstellarer Vertreter",
        ] {
            assert_eq!(
                parse_intent(query),
                VoiceIntent::NearestInterstellarFactor,
                "{query}"
            );
        }
        assert_eq!(parse_intent("combustível"), VoiceIntent::FuelStatus);
        assert_eq!(
            parse_intent("copier le prochain système"),
            VoiceIntent::CopyNextSystem
        );
        for query in ["boucliers", "Schilde"] {
            assert_eq!(parse_intent(query), VoiceIntent::ShipStatus, "{query}");
        }
        assert_eq!(parse_intent("notoriété"), VoiceIntent::CrimeStatus);
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
        assert!(res_fuel.spoken_text.contains("24,0 toneladas"));
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

    #[test]
    fn parse_intent_and_resolve_new_f22_categories() {
        assert_eq!(parse_intent("¿Dónde estoy?"), VoiceIntent::CurrentLocation);
        assert_eq!(
            parse_intent("copia el sistema actual"),
            VoiceIntent::CurrentLocation
        );
        assert_eq!(parse_intent("where am I"), VoiceIntent::CurrentLocation);
        assert_eq!(parse_intent("où suis-je"), VoiceIntent::CurrentLocation);
        assert_eq!(parse_intent("wo bin ich"), VoiceIntent::CurrentLocation);
        assert_eq!(parse_intent("onde estou"), VoiceIntent::CurrentLocation);

        assert_eq!(
            parse_intent("resumen de exploración"),
            VoiceIntent::ExplorationSummary
        );
        assert_eq!(
            parse_intent("valuable bodies system scan"),
            VoiceIntent::ExplorationSummary
        );
        assert_eq!(
            parse_intent("resumen de combate"),
            VoiceIntent::CombatSummary
        );
        assert_eq!(
            parse_intent("unredeemed bounty vouchers"),
            VoiceIntent::CombatSummary
        );
        assert_eq!(
            parse_intent("estado del portanaves"),
            VoiceIntent::CarrierStatus
        );
        assert_eq!(
            parse_intent("fleet carrier tritium"),
            VoiceIntent::CarrierStatus
        );
        assert_eq!(
            parse_intent("progreso de colonización"),
            VoiceIntent::ColonisationStatus
        );
        assert_eq!(
            parse_intent("colonisation progress"),
            VoiceIntent::ColonisationStatus
        );

        let mut state = GameState::default();
        state.location.system = "Shinrarta Dezhra".into();
        state.location.station = Some("Jameson Memorial".into());

        let ctx = VoiceResolverContext {
            locale: "es".into(),
            exploration_bodies_scanned: 12,
            exploration_bodies_total: 15,
            exploration_valuable_count: 3,
            exploration_value_now: 2_450_000,
            combat_kills: 7,
            combat_unredeemed_total: 420_000,
            carrier_name: Some("Nostromo".into()),
            carrier_system: Some("Colonia".into()),
            carrier_fuel_depot: Some(750),
            carrier_max_jumps: Some(6),
            colonisation_station: Some("Orbital Alpha".into()),
            colonisation_progress_pct: Some(65),
            colonisation_remaining_tons: Some(1200),
            colonisation_top_missing: Some("Acero".into()),
            pinned_goals_total: 2,
            pinned_goals_ready: 1,
            inventory_materials_total: 480,
            ..Default::default()
        };

        let loc_copy = VoiceIntentResolver::resolve("copia el sistema actual", &state, &ctx);
        assert_eq!(loc_copy.intent, VoiceIntent::CurrentLocation);
        assert_eq!(
            loc_copy.clipboard_copied.as_deref(),
            Some("Shinrarta Dezhra")
        );
        assert!(loc_copy.spoken_text.contains("Jameson Memorial"));

        let exp = VoiceIntentResolver::resolve("resumen de exploración", &state, &ctx);
        assert_eq!(exp.intent, VoiceIntent::ExplorationSummary);
        assert!(exp.spoken_text.contains("12 de 15"));
        assert!(exp.spoken_text.contains("2450000"));

        let comb = VoiceIntentResolver::resolve("resumen de combate", &state, &ctx);
        assert_eq!(comb.intent, VoiceIntent::CombatSummary);
        assert!(comb.spoken_text.contains("7 bajas"));
        assert!(comb.spoken_text.contains("420000"));

        let car = VoiceIntentResolver::resolve("estado del portanaves", &state, &ctx);
        assert_eq!(car.intent, VoiceIntent::CarrierStatus);
        assert!(car.spoken_text.contains("Nostromo"));
        assert!(car.spoken_text.contains("750"));

        let col = VoiceIntentResolver::resolve("progreso de colonización", &state, &ctx);
        assert_eq!(col.intent, VoiceIntent::ColonisationStatus);
        assert!(col.spoken_text.contains("Orbital Alpha"));
        assert!(col.spoken_text.contains("65 por ciento"));

        let mat = VoiceIntentResolver::resolve("materiales de ingeniería", &state, &ctx);
        assert_eq!(mat.intent, VoiceIntent::MaterialsSummary);
        assert!(mat.spoken_text.contains("2 recetas"));
        assert!(mat.spoken_text.contains("480 unidades"));
    }
}
