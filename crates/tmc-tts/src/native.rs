//! Motor de síntesis de voz nativo de Windows (SAPI5 / OneCore).
//!
//! Emplea la interfaz nativa `ISpVoice` del sistema operativo.
//! Huella de memoria mínima (< 2 MB), latencia inmediata y sin dependencias de descarga externa.

use crate::{TtsEngine, TtsError};
use tmc_model::tts::VoiceInfo;

#[cfg(windows)]
mod imp {
    use super::*;
    use std::collections::HashMap;
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::thread::{Builder, JoinHandle};
    use tracing::{error, warn};
    use windows::Win32::Media::Speech::*;
    use windows::Win32::System::Com::*;
    use windows::core::{HSTRING, Interface, w};

    enum WorkerCmd {
        Speak {
            text: String,
            voice: Option<String>,
            volume: u8,
            rate: f32,
        },
        Stop,
        Terminate,
    }

    pub struct WindowsNativeEngine {
        tx: Sender<WorkerCmd>,
        voices: Vec<VoiceInfo>,
        worker: Option<JoinHandle<()>>,
    }

    impl WindowsNativeEngine {
        pub fn new() -> Result<Self, TtsError> {
            let voices = Self::enumerate_all_voices();

            let (tx, rx) = channel::<WorkerCmd>();
            let worker = Builder::new()
                .name("tmc-native-tts-worker".into())
                .spawn(move || {
                    Self::run_worker(rx);
                })
                .map_err(|e| {
                    TtsError::Engine(format!("no se pudo iniciar hilo nativo de TTS: {e}"))
                })?;

            Ok(Self {
                tx,
                voices,
                worker: Some(worker),
            })
        }

        fn run_worker(rx: Receiver<WorkerCmd>) {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }

            let voice_res: windows::core::Result<ISpVoice> =
                unsafe { CoCreateInstance(&SpVoice, None, CLSCTX_ALL) };

            let voice: ISpVoice = match voice_res {
                Ok(v) => v,
                Err(e) => {
                    error!("Error al inicializar ISpVoice en el hilo de trabajo: {e:?}");
                    return;
                }
            };

            let token_map = Self::load_token_map();

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    WorkerCmd::Speak {
                        text,
                        voice: voice_id_or_name,
                        volume,
                        rate,
                    } => unsafe {
                        // 1. Asignar voz si se especificó
                        if let Some(target) = voice_id_or_name {
                            if let Some(token) = token_map.get(&target) {
                                let _ = voice.SetVoice(token);
                            } else {
                                for (id, token) in &token_map {
                                    if id.contains(&target) || target.contains(id) {
                                        let _ = voice.SetVoice(token);
                                        break;
                                    }
                                }
                            }
                        }

                        // 2. Volumen (0..100)
                        let vol = volume.min(100) as u16;
                        let _ = voice.SetVolume(vol);

                        // 3. Velocidad (-10..10, mapeado de 0.8..1.5)
                        let sapi_rate =
                            (((rate - 1.0) / 0.5) * 5.0).round().clamp(-10.0, 10.0) as i32;
                        let _ = voice.SetRate(sapi_rate);

                        // 4. Locución no bloqueante interrumpiendo avisos anteriores
                        let wide = HSTRING::from(text.as_str());
                        let flags = (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0) as u32;
                        if let Err(e) = voice.Speak(&wide, flags, None) {
                            warn!("Error al ejecutar Speak en ISpVoice: {e:?}");
                        }
                    },
                    WorkerCmd::Stop => unsafe {
                        let flags = SPF_PURGEBEFORESPEAK.0 as u32;
                        let _ = voice.Speak(&HSTRING::new(), flags, None);
                    },
                    WorkerCmd::Terminate => break,
                }
            }

            unsafe {
                CoUninitialize();
            }
        }

        fn load_token_map() -> HashMap<String, ISpObjectToken> {
            let mut map = HashMap::new();
            unsafe {
                // SAPI
                if let Ok(cat) = CoCreateInstance::<_, ISpObjectTokenCategory>(
                    &SpObjectTokenCategory,
                    None,
                    CLSCTX_ALL,
                ) && cat.SetId(SPCAT_VOICES, false).is_ok()
                {
                    Self::collect_tokens(&cat, &mut map);
                }
                // OneCore
                if let Ok(cat) = CoCreateInstance::<_, ISpObjectTokenCategory>(
                    &SpObjectTokenCategory,
                    None,
                    CLSCTX_ALL,
                ) && cat
                    .SetId(
                        w!("HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Speech_OneCore\\Voices"),
                        false,
                    )
                    .is_ok()
                {
                    Self::collect_tokens(&cat, &mut map);
                }
            }
            map
        }

        unsafe fn collect_tokens(
            cat: &ISpObjectTokenCategory,
            map: &mut HashMap<String, ISpObjectToken>,
        ) {
            unsafe {
                if let Ok(tokens) = cat.EnumTokens(None, None) {
                    let mut count = 0u32;
                    let _ = tokens.GetCount(&mut count);
                    for i in 0..count {
                        if let Ok(token) = tokens.Item(i) {
                            let id = token
                                .GetId()
                                .map(|p| p.to_string().unwrap_or_default())
                                .unwrap_or_default();
                            let data_key: windows::core::Result<ISpDataKey> = token.cast();
                            let name = data_key
                                .ok()
                                .and_then(|k| k.GetStringValue(None).ok())
                                .map(|p| p.to_string().unwrap_or_default())
                                .unwrap_or_default();

                            if !id.is_empty() {
                                map.insert(id.clone(), token.clone());
                            }
                            if !name.is_empty() {
                                map.insert(name, token);
                            }
                        }
                    }
                }
            }
        }

        fn enumerate_all_voices() -> Vec<VoiceInfo> {
            let mut result = Vec::new();
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

                // 1. SAPI Voices
                if let Ok(cat) = CoCreateInstance::<_, ISpObjectTokenCategory>(
                    &SpObjectTokenCategory,
                    None,
                    CLSCTX_ALL,
                ) && cat.SetId(SPCAT_VOICES, false).is_ok()
                {
                    Self::extract_voices_info(&cat, &mut result);
                }

                // 2. OneCore Voices
                if let Ok(cat) = CoCreateInstance::<_, ISpObjectTokenCategory>(
                    &SpObjectTokenCategory,
                    None,
                    CLSCTX_ALL,
                ) && cat
                    .SetId(
                        w!("HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Speech_OneCore\\Voices"),
                        false,
                    )
                    .is_ok()
                {
                    Self::extract_voices_info(&cat, &mut result);
                }
            }
            result
        }

        unsafe fn extract_voices_info(cat: &ISpObjectTokenCategory, result: &mut Vec<VoiceInfo>) {
            unsafe {
                if let Ok(tokens) = cat.EnumTokens(None, None) {
                    let mut count = 0u32;
                    let _ = tokens.GetCount(&mut count);
                    for i in 0..count {
                        if let Ok(token) = tokens.Item(i) {
                            let id = token
                                .GetId()
                                .map(|p| p.to_string().unwrap_or_default())
                                .unwrap_or_default();
                            let data_key: windows::core::Result<ISpDataKey> = token.cast();
                            let name = data_key
                                .ok()
                                .and_then(|k| k.GetStringValue(None).ok())
                                .map(|p| p.to_string().unwrap_or_default())
                                .unwrap_or_else(|| "Unknown".to_string());

                            let mut gender = None;
                            let mut lang = "es-ES".to_string();

                            if let Ok(attrs) = token.OpenKey(w!("Attributes")) {
                                if let Ok(g) = attrs.GetStringValue(w!("Gender")) {
                                    gender = g.to_string().ok().map(|s| s.to_lowercase());
                                }
                                if let Ok(l) = attrs.GetStringValue(w!("Language"))
                                    && let Ok(hex_code) = l.to_string()
                                {
                                    lang = Self::lcid_to_lang_code(&hex_code);
                                }
                            }

                            // Deduplicar si ya existe
                            if !result.iter().any(|v| v.id == id) {
                                result.push(VoiceInfo {
                                    id,
                                    name,
                                    lang,
                                    gender,
                                });
                            }
                        }
                    }
                }
            }
        }

        fn lcid_to_lang_code(lcid_hex: &str) -> String {
            match lcid_hex.trim().to_uppercase().as_str() {
                "C0A" | "40A" | "80A" => "es-ES".to_string(),
                "409" | "809" | "C09" => "en-US".to_string(),
                "40C" | "80C" => "fr-FR".to_string(),
                "407" | "807" => "de-DE".to_string(),
                "416" | "816" => "pt-BR".to_string(),
                _ => "es-ES".to_string(),
            }
        }
    }

    impl Drop for WindowsNativeEngine {
        fn drop(&mut self) {
            let _ = self.tx.send(WorkerCmd::Terminate);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    impl TtsEngine for WindowsNativeEngine {
        fn speak(
            &mut self,
            text: &str,
            voice: Option<&str>,
            volume: u8,
            rate: f32,
        ) -> Result<(), TtsError> {
            self.tx
                .send(WorkerCmd::Speak {
                    text: text.to_string(),
                    voice: voice.map(String::from),
                    volume,
                    rate,
                })
                .map_err(|e| {
                    TtsError::Engine(format!("canal del sintetizador nativo cerrado: {e}"))
                })
        }

        fn stop(&mut self) -> Result<(), TtsError> {
            self.tx.send(WorkerCmd::Stop).map_err(|e| {
                TtsError::Engine(format!("canal del sintetizador nativo cerrado: {e}"))
            })
        }

        fn available_voices(&self) -> Vec<VoiceInfo> {
            self.voices.clone()
        }
    }
}

/// Motor de síntesis nativo del sistema operativo.
pub struct NativeEngine {
    #[cfg(windows)]
    inner: imp::WindowsNativeEngine,
    #[cfg(not(windows))]
    voices: Vec<VoiceInfo>,
}

impl NativeEngine {
    /// Inicializa el motor nativo del sistema operativo.
    pub fn new() -> Result<Self, TtsError> {
        #[cfg(windows)]
        {
            let inner = imp::WindowsNativeEngine::new()?;
            Ok(Self { inner })
        }
        #[cfg(not(windows))]
        {
            Ok(Self {
                voices: vec![VoiceInfo {
                    id: "native_default".into(),
                    name: "Native Default".into(),
                    lang: "es-ES".into(),
                    gender: None,
                }],
            })
        }
    }
}

impl TtsEngine for NativeEngine {
    fn speak(
        &mut self,
        text: &str,
        voice: Option<&str>,
        volume: u8,
        rate: f32,
    ) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            self.inner.speak(text, voice, volume, rate)
        }
        #[cfg(not(windows))]
        {
            let _ = (text, voice, volume, rate);
            Ok(())
        }
    }

    fn stop(&mut self) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            self.inner.stop()
        }
        #[cfg(not(windows))]
        {
            Ok(())
        }
    }

    fn available_voices(&self) -> Vec<VoiceInfo> {
        #[cfg(windows)]
        {
            self.inner.available_voices()
        }
        #[cfg(not(windows))]
        {
            self.voices.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_engine_initializes_and_lists_voices() {
        let mut engine =
            NativeEngine::new().expect("NativeEngine debe inicializarse correctamente");
        let voices = engine.available_voices();
        #[cfg(windows)]
        {
            assert!(
                !voices.is_empty(),
                "Windows debe tener al menos una voz SAPI/OneCore instalada"
            );
            let first_voice_id = voices[0].id.clone();
            engine
                .speak("Prueba de copiloto", Some(&first_voice_id), 80, 1.0)
                .expect("Speak debe despacharse sin error");
            engine.stop().expect("Stop debe ejecutarse sin error");
        }
        #[cfg(not(windows))]
        {
            assert!(!voices.is_empty());
        }
    }
}
