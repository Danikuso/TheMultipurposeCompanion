//! Detección e integración automática con VoiceAttack y generación de perfiles `.vap`.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tmc_model::voice_intent::VoiceAttackStatus;

/// Rutas canónicas donde VoiceAttack suele estar instalado en Windows.
const STANDARD_INSTALL_PATHS: &[&str] = &[
    r"C:\Program Files\VoiceAttack\VoiceAttack.exe",
    r"C:\Program Files (x86)\VoiceAttack\VoiceAttack.exe",
    r"C:\Program Files (x86)\Steam\steamapps\common\VoiceAttack\VoiceAttack.exe",
    r"C:\Program Files\Steam\steamapps\common\VoiceAttack\VoiceAttack.exe",
    r"D:\Steam\steamapps\common\VoiceAttack\VoiceAttack.exe",
    r"D:\SteamLibrary\steamapps\common\VoiceAttack\VoiceAttack.exe",
    r"E:\SteamLibrary\steamapps\common\VoiceAttack\VoiceAttack.exe",
];

/// Detecta el estado de VoiceAttack en el sistema operativo local.
pub fn detect_voiceattack() -> VoiceAttackStatus {
    let install_path = find_voiceattack_executable();
    let installed = install_path.is_some();
    let running = is_voiceattack_running();

    VoiceAttackStatus {
        installed,
        running,
        profile_available: true,
        install_path: install_path.map(|p| p.to_string_lossy().into_owned()),
    }
}

/// Comprueba si el ejecutable de VoiceAttack existe en rutas conocidas o por variable de entorno.
pub fn find_voiceattack_executable() -> Option<PathBuf> {
    // 1. Variable de entorno explícita (e.g. `VOICEATTACK_PATH` o `VOICEATTACK_DIR`)
    if let Ok(env_path) = std::env::var("VOICEATTACK_PATH") {
        let p = PathBuf::from(env_path);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(env_dir) = std::env::var("VOICEATTACK_DIR") {
        let p = PathBuf::from(env_dir).join("VoiceAttack.exe");
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. Comprobación en rutas estándar de Windows
    for candidate in STANDARD_INSTALL_PATHS {
        let path = Path::new(candidate);
        if path.is_file() {
            return Some(path.to_path_buf());
        }
    }

    None
}

/// Comprueba si el proceso `VoiceAttack.exe` está actualmente activo en el sistema.
pub fn is_voiceattack_running() -> bool {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());

    system.processes().values().any(|process| {
        let name = process.name().to_string_lossy();
        name.eq_ignore_ascii_case("VoiceAttack.exe") || name.eq_ignore_ascii_case("VoiceAttack")
    })
}

/// Genera el contenido XML del perfil oficial `TMC-Copiloto.vap` adaptado al puerto local de TMC.
pub fn generate_vap_profile(port: u16) -> String {
    let commands = [
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b11",
            "¿Estado de la nave?;Estado de la nave;Informe de nave;Ship status",
            "ShipStatus",
            "Consulta el estado general del casco, escudos y depósitos",
        ),
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b12",
            "¿Cuál es el siguiente salto?;Siguiente salto;Próximo salto;Next jump",
            "NextJump",
            "Informa del siguiente sistema estelar en la ruta de navegación",
        ),
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b13",
            "Copia el siguiente sistema;Copiar salto;Copiar siguiente salto;Copy next jump",
            "CopyNextSystem",
            "Copia el siguiente salto estelar al portapapeles del piloto",
        ),
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b14",
            "¿Dónde está el factor interestelar más cercano?;Factor interestelar más cercano;Factor interestelar;Nearest interstellar factor",
            "NearestInterstellarFactor",
            "Localiza el factor interestelar más próximo y copia su sistema al portapapeles",
        ),
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b15",
            "¿Dónde hay un mercado negro?;Mercado negro más cercano;Mercado negro;Nearest black market",
            "NearestBlackMarket",
            "Localiza el mercado negro más próximo y copia su sistema al portapapeles",
        ),
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b16",
            "¿Tengo multas?;¿Cuál es mi notoriedad?;Estado legal;Multas y notoriedad;Crime status",
            "CrimeStatus",
            "Consulta el nivel de notoriedad, multas activas y recompensas",
        ),
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b17",
            "¿Hay bio señales?;Señales biológicas;Bio señales;Bio signals",
            "BioSignals",
            "Informa sobre la presencia de bio-señales en cuerpos del sistema actual",
        ),
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b18",
            "¿Qué materiales me faltan?;Resumen de materiales;Materiales fijados;Materials summary",
            "MaterialsSummary",
            "Consulta los materiales pendientes para las recetas fijadas de ingeniería",
        ),
        (
            "b4382594-5a21-4f17-8a4b-2d7c04192b19",
            "¿Cuánto combustible me queda?;Nivel de combustible;Estado del combustible;Fuel status",
            "FuelStatus",
            "Consulta el nivel porcentual y toneladas del tanque de combustible",
        ),
    ];

    let mut xml = String::with_capacity(8192);
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n");
    xml.push_str("<Profile xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\r\n");
    xml.push_str("  <Id>b4382594-5a21-4f17-8a4b-2d7c04192b01</Id>\r\n");
    xml.push_str("  <Name>TMC - The Multipurpose Companion</Name>\r\n");
    xml.push_str("  <Commands>\r\n");

    for (cmd_id, phrases, query_val, desc) in commands {
        xml.push_str("    <Command>\r\n");
        xml.push_str(&format!("      <Id>{cmd_id}</Id>\r\n"));
        xml.push_str(&format!(
            "      <CommandString>{phrases}</CommandString>\r\n"
        ));
        xml.push_str(&format!("      <Description>{desc}</Description>\r\n"));
        xml.push_str("      <Category>TMC Cabina</Category>\r\n");
        xml.push_str("      <ActionSequence>\r\n");
        xml.push_str("        <CommandAction>\r\n");
        xml.push_str("          <ActionType>Launch</ActionType>\r\n");
        xml.push_str("          <Path>powershell.exe</Path>\r\n");
        xml.push_str(&format!(
            "          <Args>-WindowStyle Hidden -NoProfile -Command \"Invoke-RestMethod -Uri 'http://127.0.0.1:{port}/api/voice/intent' -Method Post -ContentType 'application/json' -Body '{{\\\"query\\\":\\\"{query_val}\\\"}}'\"</Args>\r\n"
        ));
        xml.push_str("          <Async>true</Async>\r\n");
        xml.push_str("        </CommandAction>\r\n");
        xml.push_str("      </ActionSequence>\r\n");
        xml.push_str("      <Async>true</Async>\r\n");
        xml.push_str("      <Enabled>true</Enabled>\r\n");
        xml.push_str("    </Command>\r\n");
    }

    xml.push_str("  </Commands>\r\n");
    xml.push_str("</Profile>\r\n");

    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vap_profile_contains_expected_commands_and_port() {
        let profile = generate_vap_profile(47320);
        assert!(profile.contains("<Name>TMC - The Multipurpose Companion</Name>"));
        assert!(profile.contains("http://127.0.0.1:47320/api/voice/intent"));
        assert!(profile.contains("ShipStatus"));
        assert!(profile.contains("NearestInterstellarFactor"));
        assert!(profile.contains("NearestBlackMarket"));
        assert!(profile.contains("CrimeStatus"));
        assert!(profile.contains("BioSignals"));
        assert!(profile.contains("FuelStatus"));
        assert!(profile.contains("CopyNextSystem"));
    }

    #[test]
    fn detect_voiceattack_returns_valid_struct() {
        let status = detect_voiceattack();
        assert!(status.profile_available);
        // `installed` and `running` will reflect the test environment without failing.
        let _ = status.installed;
        let _ = status.running;
    }
}
