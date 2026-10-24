//! Modelos para el Asistente e Integración de Voz de cabina (Fase 10).

use serde::{Deserialize, Serialize};

/// Intenciones reconocidas por el asistente de voz de cabina.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceIntent {
    /// Consulta del estado general de la nave (casco, escudos, combustible).
    ShipStatus,
    /// Consulta del siguiente salto estelar planificado.
    NextJump,
    /// Copia el siguiente sistema en ruta al portapapeles.
    CopyNextSystem,
    /// Localiza el factor interestelar más cercano y lo copia al portapapeles.
    NearestInterstellarFactor,
    /// Localiza el mercado negro más cercano.
    NearestBlackMarket,
    /// Consulta sobre multas, recompensas y nivel de notoriedad.
    CrimeStatus,
    /// Consulta sobre bio-señales en el sistema estelar actual.
    BioSignals,
    /// Consulta sobre materiales prioritarios / recetas fijadas.
    MaterialsSummary,
    /// Consulta sobre el nivel y depósito de combustible.
    FuelStatus,
    /// Consulta o comando no reconocido.
    #[serde(untagged)]
    Unknown(String),
}

impl VoiceIntent {
    /// Devuelve el identificador textual de la intención para diagnóstico y UI.
    pub fn as_str(&self) -> &str {
        match self {
            Self::ShipStatus => "ship_status",
            Self::NextJump => "next_jump",
            Self::CopyNextSystem => "copy_next_system",
            Self::NearestInterstellarFactor => "nearest_interstellar_factor",
            Self::NearestBlackMarket => "nearest_black_market",
            Self::CrimeStatus => "crime_status",
            Self::BioSignals => "bio_signals",
            Self::MaterialsSummary => "materials_summary",
            Self::FuelStatus => "fuel_status",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

/// Petición enviada al endpoint de voz de TMC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceCommandRequest {
    /// Texto transcrito o comando de voz emitido por el piloto.
    pub query: String,
    /// Idioma preferido (ej: "es", "en", "fr", "de", "pt"). Opcional; por defecto el configurado en TMC.
    pub locale: Option<String>,
}

/// Respuesta devuelta por el asistente de voz.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceCommandResponse {
    /// Intención identificada por el analizador semántico.
    pub intent: VoiceIntent,
    /// Nombre de la acción ejecutada si corresponde (ej: "clipboard.copy_system").
    pub action_executed: Option<String>,
    /// Texto exacto vocalizado por el Copiloto TTS.
    pub spoken_text: String,
    /// Contenido copiado al portapapeles del piloto (si aplica).
    pub clipboard_copied: Option<String>,
    /// Datos adicionales contextuales de la respuesta.
    pub data: serde_json::Value,
}

/// Estado de la integración automática con VoiceAttack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VoiceAttackStatus {
    /// Si VoiceAttack está instalado o detectado en el sistema.
    pub installed: bool,
    /// Si el proceso de VoiceAttack se encuentra actualmente en ejecución.
    pub running: bool,
    /// Si el perfil oficial `TMC-Copiloto.vap` está generado y sincronizado.
    pub profile_available: bool,
    /// Ruta del ejecutable o carpeta de instalación si fue localizado.
    pub install_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_intent_serde_roundtrip() {
        let intents = [
            (VoiceIntent::ShipStatus, "\"ship_status\""),
            (VoiceIntent::NextJump, "\"next_jump\""),
            (VoiceIntent::CopyNextSystem, "\"copy_next_system\""),
            (
                VoiceIntent::NearestInterstellarFactor,
                "\"nearest_interstellar_factor\"",
            ),
            (VoiceIntent::NearestBlackMarket, "\"nearest_black_market\""),
            (VoiceIntent::CrimeStatus, "\"crime_status\""),
            (VoiceIntent::BioSignals, "\"bio_signals\""),
            (VoiceIntent::MaterialsSummary, "\"materials_summary\""),
            (VoiceIntent::FuelStatus, "\"fuel_status\""),
            (
                VoiceIntent::Unknown("custom_action".into()),
                "\"custom_action\"",
            ),
        ];

        for (intent, json_str) in intents {
            let serialized = serde_json::to_string(&intent).unwrap();
            assert_eq!(serialized, json_str);
            let deserialized: VoiceIntent = serde_json::from_str(json_str).unwrap();
            assert_eq!(deserialized, intent);
        }
    }

    #[test]
    fn voice_command_request_and_response_roundtrip() {
        let req = VoiceCommandRequest {
            query: "¿Dónde está el factor interestelar más cercano?".into(),
            locale: Some("es".into()),
        };
        let req_json = serde_json::to_string(&req).unwrap();
        let req_back: VoiceCommandRequest = serde_json::from_str(&req_json).unwrap();
        assert_eq!(req, req_back);

        let res = VoiceCommandResponse {
            intent: VoiceIntent::NearestInterstellarFactor,
            action_executed: Some("clipboard.copy_system".into()),
            spoken_text: "Baird Gateway en LTT 1873 a 8,4 años luz.".into(),
            clipboard_copied: Some("LTT 1873".into()),
            data: serde_json::json!({
                "system": "LTT 1873",
                "station": "Baird Gateway"
            }),
        };
        let res_json = serde_json::to_string(&res).unwrap();
        let res_back: VoiceCommandResponse = serde_json::from_str(&res_json).unwrap();
        assert_eq!(res, res_back);
    }

    #[test]
    fn voiceattack_status_defaults() {
        let status = VoiceAttackStatus::default();
        assert!(!status.installed);
        assert!(!status.running);
        assert!(!status.profile_available);
        assert_eq!(status.install_path, None);
    }
}
