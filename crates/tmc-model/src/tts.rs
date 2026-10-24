//! Modelos de datos para el copiloto auditivo y avisos de voz (Fase 7).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Motor de síntesis de voz configurado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TtsEngineKind {
    /// Síntesis nativa del sistema operativo (Windows OneCore / SAPI5).
    /// Ultraligera (< 2 MB RAM, latencia cero, sin descargas).
    #[default]
    Native,
    /// Motor neuronal KokoroTTS ONNX en subproceso desacoplado.
    /// Voces IA cinematográficas con carga bajo demanda.
    Kokoro,
}

/// Canales temáticos de avisos de cabina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsChannel {
    /// Alertas de combate y supervivencia (escudos caídos, temperatura crítica, casco).
    Combat,
    /// Avisos de navegación y saltos (siguiente sistema, estrella repostable).
    Navigation,
    /// Alertas de exploración y exobiología (bio-señales en sistema, muestras biológicas).
    Exploration,
    /// Alertas de minería y comercio (prospectores valiosos, bodega llena).
    Mining,
    /// Avisos del Fleet Carrier (cuenta atrás de salto, salto completado).
    Carrier,
}

/// Prioridad de locución de un aviso de cabina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TtsPriority {
    /// Informativo: se encola y reproduce cuando no haya nada más urgente.
    #[default]
    Informational,
    /// Táctico: se sitúa al frente de la cola ordinaria (ej. bio-señales, salto).
    Tactical,
    /// Emergencia: interrumpe cualquier locución en curso (ej. escudos caídos, casco crítico).
    Emergency,
}

/// Configuración del copiloto de voz persistida en ajustes de la aplicación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TtsSettings {
    /// Si el copiloto auditivo está activado o silenciado globalmente.
    pub enabled: bool,
    /// Motor de síntesis de voz preferido.
    pub engine: TtsEngineKind,
    /// Identificador o nombre de la voz seleccionada (si es None, se usa la predeterminada del sistema/motor).
    pub voice: Option<String>,
    /// Nivel de volumen de 0 a 100.
    pub volume: u8,
    /// Velocidad de locución en porcentaje (80..=150, 100 = 1.0x por defecto).
    pub rate: u8,
    /// Canales habilitados individualmente.
    pub channels: BTreeMap<TtsChannel, bool>,
}

impl Default for TtsSettings {
    fn default() -> Self {
        let mut channels = BTreeMap::new();
        channels.insert(TtsChannel::Combat, true);
        channels.insert(TtsChannel::Navigation, true);
        channels.insert(TtsChannel::Exploration, true);
        channels.insert(TtsChannel::Mining, true);
        channels.insert(TtsChannel::Carrier, true);

        Self {
            enabled: false,
            engine: TtsEngineKind::Native,
            voice: None,
            volume: 80,
            rate: 100,
            channels,
        }
    }
}

impl TtsSettings {
    /// Comprueba si un canal específico está habilitado en los ajustes.
    pub fn is_channel_enabled(&self, channel: TtsChannel) -> bool {
        self.enabled && *self.channels.get(&channel).unwrap_or(&true)
    }

    /// Retorna el multiplicador de velocidad en punto flotante (ej. 1.0 para 100%).
    pub fn rate_multiplier(&self) -> f32 {
        self.rate as f32 / 100.0
    }
}

/// Solicitud de aviso táctico emitido por el `VoiceAdvisor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceCue {
    /// Prioridad del aviso.
    pub priority: TtsPriority,
    /// Canal al que pertenece para filtrado por categoría.
    pub channel: TtsChannel,
    /// Clave del mensaje en los archivos de localización `locales/*/tts.ftl`.
    pub text_key: String,
    /// Argumentos para el formateador Fluent de i18n.
    #[serde(default)]
    pub args: BTreeMap<String, String>,
    /// Clave única para deduplicación y control de cooldown (ej. "combat_heat_warning").
    #[serde(default)]
    pub cooldown_key: Option<String>,
}

impl VoiceCue {
    pub fn new(priority: TtsPriority, channel: TtsChannel, text_key: impl Into<String>) -> Self {
        Self {
            priority,
            channel,
            text_key: text_key.into(),
            args: BTreeMap::new(),
            cooldown_key: None,
        }
    }

    pub fn with_arg(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.args.insert(key.into(), val.into());
        self
    }

    pub fn with_cooldown(mut self, key: impl Into<String>) -> Self {
        self.cooldown_key = Some(key.into());
        self
    }
}

/// Información descriptiva de una voz disponible en el sistema o motor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceInfo {
    /// Identificador único interno para invocarla.
    pub id: String,
    /// Nombre visible amigable para la interfaz de usuario.
    pub name: String,
    /// Código de idioma BCP-47 o IETF (ej. "es-ES", "en-US", "fr-FR").
    pub lang: String,
    /// Género si se conoce (opcional).
    #[serde(default)]
    pub gender: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tts_settings_defaults() {
        let s = TtsSettings::default();
        assert!(!s.enabled);
        assert_eq!(s.engine, TtsEngineKind::Native);
        assert_eq!(s.volume, 80);
        assert_eq!(s.rate, 100);
        assert!((s.rate_multiplier() - 1.0).abs() < f32::EPSILON);
        assert_eq!(s.channels.len(), 5);
        assert!(!s.is_channel_enabled(TtsChannel::Combat)); // Deshabilitado globalmente
    }

    #[test]
    fn tts_channel_toggling() {
        let mut s = TtsSettings {
            enabled: true,
            ..Default::default()
        };
        assert!(s.is_channel_enabled(TtsChannel::Combat));
        assert!(s.is_channel_enabled(TtsChannel::Navigation));

        s.channels.insert(TtsChannel::Combat, false);
        assert!(!s.is_channel_enabled(TtsChannel::Combat));
        assert!(s.is_channel_enabled(TtsChannel::Navigation));
    }

    #[test]
    fn voice_cue_builder() {
        let cue = VoiceCue::new(
            TtsPriority::Tactical,
            TtsChannel::Navigation,
            "tts-nav-jump-scoopable",
        )
        .with_arg("system", "Sol")
        .with_cooldown("nav_jump");

        assert_eq!(cue.priority, TtsPriority::Tactical);
        assert_eq!(cue.channel, TtsChannel::Navigation);
        assert_eq!(cue.text_key, "tts-nav-jump-scoopable");
        assert_eq!(cue.args.get("system").unwrap(), "Sol");
        assert_eq!(cue.cooldown_key.as_deref(), Some("nav_jump"));
    }

    #[test]
    fn tts_settings_serde_roundtrip() {
        let s = TtsSettings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: TtsSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);

        // Deserialización vacía usa defaults
        let empty: TtsSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, TtsSettings::default());
    }

    #[test]
    fn voice_info_serde() {
        let info = VoiceInfo {
            id: "es_helena".into(),
            name: "Microsoft Helena - Spanish (Spain)".into(),
            lang: "es-ES".into(),
            gender: Some("female".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: VoiceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, back);
    }
}
