use crate::carrier::Carrier;
use crate::colonisation::Colonisation;
use crate::combat::Combat;
use crate::dashboard::Dashboard;
use crate::engineering::Engineering;
use crate::exobiology::Exobiology;
use crate::exploration::Exploration;
use crate::mining::Mining;
use crate::missions::Missions;
use crate::navigation::Navigation;
use crate::trade::Trade;
use tmc_core::Module;

pub fn all_modules() -> Vec<Box<dyn Module>> {
    vec![
        Box::new(Dashboard::new()),
        Box::new(Navigation::new()),
        Box::new(Exploration::new()),
        Box::new(Exobiology::new()),
        Box::new(Combat::new()),
        Box::new(Mining::new()),
        Box::new(Trade::new()),
        Box::new(Missions::new()),
        Box::new(Engineering::new()),
        Box::new(Carrier::new()),
        Box::new(Colonisation::new()),
    ]
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;
    use std::path::Path;
    use tmc_model::diagnostics::{is_registered_action, is_registered_topic};

    fn rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                rs_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    #[test]
    fn every_module_topic_and_action_is_in_the_diagnostics_registry() {
        for m in all_modules() {
            for topic in m.topics() {
                assert!(
                    is_registered_topic(&topic),
                    "tema `{topic}` sin registrar en tmc-model::diagnostics::TOPICS"
                );
            }
        }
        // Las acciones no son introspectables: se leen de `const ACTION_*` y de los
        // brazos `"modulo.accion" =>` del planificador de navegación.
        let mut files = Vec::new();
        rs_files(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let mut found = 0;
        for f in files {
            let text = std::fs::read_to_string(&f).unwrap();
            for line in text.lines().map(str::trim) {
                let is_action = line.starts_with("const ACTION_")
                    || line.starts_with("pub const ACTION_")
                    || (line.starts_with("\"navigation.") && line.contains("=>"));
                let name = is_action.then(|| line.split('"').nth(1)).flatten();
                if let Some(name) = name {
                    found += 1;
                    assert!(
                        is_registered_action(name),
                        "acción `{name}` sin registrar en tmc-model::diagnostics::ACTIONS"
                    );
                }
            }
        }
        assert!(
            found >= 12,
            "el escaneo de acciones no encontró las esperadas ({found})"
        );
    }
}
