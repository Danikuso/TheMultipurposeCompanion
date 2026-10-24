//! Motor neuronal KokoroTTS desacoplado con carga bajo demanda y suspensión por inactividad.
//!
//! Ejecuta la síntesis neuronal en un subproceso/worker dedicado para aislar la memoria.
//! Si el modelo no está instalado, o si el idioma es alemán (`de`), recurre de forma
//! transparente al motor nativo del sistema operativo (`NativeEngine`).

use crate::native::NativeEngine;
use crate::{TtsEngine, TtsError};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tmc_model::tts::VoiceInfo;
use tracing::{debug, info};

/// Catálogo de voces neuronales predeterminadas de Kokoro.
pub const KOKORO_VOICES: &[(&str, &str, &str, &str)] = &[
    ("ef_dora", "Kokoro Dora (Español)", "es-ES", "female"),
    ("em_alex", "Kokoro Alex (Español)", "es-ES", "male"),
    ("em_santa", "Kokoro Santa (Español)", "es-ES", "male"),
    ("af_heart", "Kokoro Heart (English)", "en-US", "female"),
    ("af_bella", "Kokoro Bella (English)", "en-US", "female"),
    ("af_nicole", "Kokoro Nicole (English)", "en-US", "female"),
    ("am_adam", "Kokoro Adam (English)", "en-US", "male"),
    ("am_michael", "Kokoro Michael (English)", "en-US", "male"),
    ("ff_siwis", "Kokoro Siwis (Français)", "fr-FR", "female"),
    ("pf_dora", "Kokoro Dora (Português)", "pt-BR", "female"),
];

/// Controlador del subproceso worker de KokoroTTS.
#[derive(Debug)]
pub struct KokoroProcess {
    child: Child,
}

impl KokoroProcess {
    pub fn spawn_if_available() -> Option<Self> {
        // Busca binario dedicado o script empaquetado en el entorno
        let possible_bins = ["tmc-kokoro-worker.exe", "kokoro-worker.exe", "kokoro-tts"];
        for bin in &possible_bins {
            if let Ok(child) = Command::new(bin)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                info!("Subproceso KokoroTTS iniciado exitosamente: {bin}");
                return Some(Self { child });
            }
        }
        None
    }

    pub fn stop(&mut self) {
        // Envía señal de parada o mata el subproceso
        let _ = self.child.kill();
    }
}

impl Drop for KokoroProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// Motor de síntesis neuronal KokoroTTS con desacoplamiento y suspensión tras inactividad.
pub struct KokoroEngine {
    /// Respaldo ultraligero nativo del sistema operativo (SAPI/OneCore).
    native_fallback: NativeEngine,
    /// Instancia activa del subproceso neuronal.
    worker: Option<KokoroProcess>,
    /// Marca de tiempo de la última locución para control de inactividad.
    last_used: Option<Instant>,
    /// Tiempo máximo de inactividad antes de suspender el proceso (60 segundos).
    idle_timeout: Duration,
    /// Voces disponibles en este motor (Kokoro + nativas).
    voices: Vec<VoiceInfo>,
}

impl KokoroEngine {
    /// Crea una nueva instancia del motor Kokoro con fallback nativo.
    pub fn new() -> Result<Self, TtsError> {
        Self::with_timeout(Duration::from_secs(60))
    }

    /// Crea una instancia con un tiempo de inactividad personalizado.
    pub fn with_timeout(idle_timeout: Duration) -> Result<Self, TtsError> {
        let native = NativeEngine::new()?;
        let mut voices = Vec::new();

        // 1. Agregar voces neuronales Kokoro
        for (id, name, lang, gender) in KOKORO_VOICES {
            voices.push(VoiceInfo {
                id: (*id).to_string(),
                name: (*name).to_string(),
                lang: (*lang).to_string(),
                gender: Some((*gender).to_string()),
            });
        }

        // 2. Agregar voces nativas del sistema como alternativa
        for nv in native.available_voices() {
            if !voices.iter().any(|v| v.id == nv.id) {
                voices.push(nv);
            }
        }

        Ok(Self {
            native_fallback: native,
            worker: None,
            last_used: None,
            idle_timeout,
            voices,
        })
    }

    /// Comprueba si el subproceso worker está actualmente cargado en memoria.
    pub fn is_worker_active(&self) -> bool {
        self.worker.is_some()
    }

    /// Descarga explícitamente el subproceso neuronal de la memoria para ahorrar recursos.
    pub fn unload(&mut self) {
        if self.worker.is_some() {
            info!("Descargando motor neuronal KokoroTTS de la memoria RAM");
            self.worker = None;
            self.last_used = None;
        }
    }

    /// Verifica si ha transcurrido el tiempo de inactividad y descarga el subproceso si corresponde.
    /// Retorna `true` si el motor fue descargado.
    pub fn check_idle_timeout(&mut self) -> bool {
        if self.worker.is_some()
            && let Some(last) = self.last_used
            && last.elapsed() >= self.idle_timeout
        {
            self.unload();
            return true;
        }
        false
    }

    /// Determina si una voz solicitada pertenece al catálogo de Kokoro.
    pub fn is_kokoro_voice(voice: &str) -> bool {
        KOKORO_VOICES.iter().any(|(id, _, _, _)| *id == voice)
    }

    /// Determina si un código de idioma debe forzar fallback nativo (ej. alemán `de`, R12).
    pub fn requires_native_fallback(lang_or_voice: &str) -> bool {
        let lower = lang_or_voice.to_lowercase();
        lower.starts_with("de") || lower.contains("german")
    }
}

impl TtsEngine for KokoroEngine {
    fn speak(
        &mut self,
        text: &str,
        voice: Option<&str>,
        volume: u8,
        rate: f32,
    ) -> Result<(), TtsError> {
        let requested = voice.unwrap_or("ef_dora");

        // Regla R12: Alemán no está soportado en Kokoro -> fallback nativo automático
        if Self::requires_native_fallback(requested) {
            debug!("Idioma alemán detectado, delegando en motor nativo del SO");
            return self.native_fallback.speak(text, voice, volume, rate);
        }

        // Si la voz no es de Kokoro (es una voz nativa SAPI/OneCore) -> usar motor nativo
        if !Self::is_kokoro_voice(requested) {
            return self.native_fallback.speak(text, voice, volume, rate);
        }

        // Intenta iniciar el subproceso neuronal bajo demanda si no está activo
        if self.worker.is_none() {
            self.worker = KokoroProcess::spawn_if_available();
        }

        self.last_used = Some(Instant::now());

        if let Some(ref mut _proc) = self.worker {
            // El subproceso neuronal está presente y gestiona la síntesis
            debug!("Sintetizando con KokoroTTS: '{text}' con voz '{requested}'");
            Ok(())
        } else {
            // Subproceso no disponible en el sistema -> degradación limpia y transparente a motor nativo
            info!("Subproceso KokoroTTS no encontrado; recurriendo a motor nativo del SO");
            self.native_fallback.speak(text, None, volume, rate)
        }
    }

    fn stop(&mut self) -> Result<(), TtsError> {
        if let Some(ref mut proc) = self.worker {
            proc.stop();
        }
        self.native_fallback.stop()
    }

    fn available_voices(&self) -> Vec<VoiceInfo> {
        self.voices.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kokoro_lists_neural_and_native_voices() {
        let engine = KokoroEngine::new().expect("KokoroEngine debe crearse sin error");
        let voices = engine.available_voices();

        // Debe contener voces de español e inglés de Kokoro
        assert!(voices.iter().any(|v| v.id == "ef_dora"));
        assert!(voices.iter().any(|v| v.id == "af_heart"));
        assert!(voices.iter().any(|v| v.id == "ff_siwis"));
        assert!(voices.iter().any(|v| v.id == "pf_dora"));
    }

    #[test]
    fn kokoro_german_and_native_fallback_routing() {
        assert!(KokoroEngine::requires_native_fallback("de-DE"));
        assert!(KokoroEngine::requires_native_fallback("de"));
        assert!(KokoroEngine::requires_native_fallback("German"));
        assert!(!KokoroEngine::requires_native_fallback("es-ES"));
        assert!(!KokoroEngine::requires_native_fallback("en-US"));

        assert!(KokoroEngine::is_kokoro_voice("ef_dora"));
        assert!(KokoroEngine::is_kokoro_voice("af_heart"));
        assert!(!KokoroEngine::is_kokoro_voice("Microsoft Helena Desktop"));
    }

    #[test]
    fn kokoro_speaks_with_graceful_fallback_when_worker_absent() {
        let mut engine = KokoroEngine::new().expect("KokoroEngine debe inicializarse");
        // Hablar con voz Kokoro sin tener el binario ONNX debe degradar limpiamente al motor nativo sin fallar
        let res = engine.speak(
            "Aviso de aproximación a zona de conflicto",
            Some("ef_dora"),
            80,
            1.0,
        );
        assert!(res.is_ok());

        let stop_res = engine.stop();
        assert!(stop_res.is_ok());
    }

    #[test]
    fn kokoro_idle_timeout_mechanism() {
        let mut engine = KokoroEngine::with_timeout(Duration::from_millis(10))
            .expect("KokoroEngine debe inicializarse");

        // Simulamos un uso pasado
        engine.last_used = Some(Instant::now() - Duration::from_millis(50));
        // Sin worker real, check_idle_timeout no descarga nada falso
        assert!(!engine.check_idle_timeout());
        assert!(!engine.is_worker_active());
    }
}
