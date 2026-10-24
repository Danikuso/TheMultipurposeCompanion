//! Crate de síntesis de voz (TTS) y copiloto auditivo para TMC.
#![forbid(unsafe_code)]

pub mod queue;

pub use queue::{QueuedSpeech, TtsQueue};
use tmc_model::tts::VoiceInfo;

/// Errores derivados de la síntesis de voz.
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("error del motor de voz: {0}")]
    Engine(String),
    #[error("voz no encontrada: {0}")]
    VoiceNotFound(String),
    #[error("motor no inicializado o no soportado en esta plataforma")]
    Unsupported,
    #[error("error de comunicación con el subproceso: {0}")]
    Ipc(String),
}

/// Contrato para cualquier motor de síntesis de voz (Nativo del SO, Kokoro, etc.).
pub trait TtsEngine: Send + Sync {
    /// Sintetiza y reproduce el texto indicado de forma no bloqueante o devuelve control tras iniciar.
    fn speak(
        &mut self,
        text: &str,
        voice: Option<&str>,
        volume: u8,
        rate: f32,
    ) -> Result<(), TtsError>;

    /// Detiene de inmediato cualquier locución en reproducción.
    fn stop(&mut self) -> Result<(), TtsError>;

    /// Lista todas las voces disponibles para este motor en el sistema actual.
    fn available_voices(&self) -> Vec<VoiceInfo>;
}

/// Motor simulado para pruebas unitarias e integración sin hardware de audio.
#[derive(Debug, Default)]
pub struct MockEngine {
    pub spoken: Vec<String>,
    pub stopped: bool,
    pub voices: Vec<VoiceInfo>,
}

impl MockEngine {
    pub fn new() -> Self {
        Self {
            spoken: Vec::new(),
            stopped: false,
            voices: vec![
                VoiceInfo {
                    id: "mock_es".into(),
                    name: "Mock Spanish Voice".into(),
                    lang: "es-ES".into(),
                    gender: Some("female".into()),
                },
                VoiceInfo {
                    id: "mock_en".into(),
                    name: "Mock English Voice".into(),
                    lang: "en-US".into(),
                    gender: Some("male".into()),
                },
            ],
        }
    }
}

impl TtsEngine for MockEngine {
    fn speak(
        &mut self,
        text: &str,
        _voice: Option<&str>,
        _volume: u8,
        _rate: f32,
    ) -> Result<(), TtsError> {
        self.spoken.push(text.to_string());
        self.stopped = false;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), TtsError> {
        self.stopped = true;
        Ok(())
    }

    fn available_voices(&self) -> Vec<VoiceInfo> {
        self.voices.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_engine_operations() {
        let mut engine = MockEngine::new();
        assert_eq!(engine.available_voices().len(), 2);

        engine.speak("Hola comandante", None, 80, 1.0).unwrap();
        assert_eq!(engine.spoken, vec!["Hola comandante".to_string()]);

        engine.stop().unwrap();
        assert!(engine.stopped);
    }
}
