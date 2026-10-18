//! Recursos Fluent compartidos entre el núcleo y la interfaz.

use fluent_bundle::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use fluent_syntax::ast::Entry;
use rust_embed::RustEmbed;
use std::collections::BTreeSet;
use unic_langid::LanguageIdentifier;

pub const LANGUAGES: [&str; 5] = ["es", "en", "fr", "de", "pt"];

#[derive(RustEmbed)]
#[folder = "../../locales"]
struct Resources;

pub struct I18n {
    primary: FluentBundle<FluentResource>,
    fallback: FluentBundle<FluentResource>,
}
impl I18n {
    pub fn new(lang: &str) -> Self {
        let lang = supported_language(lang);
        Self {
            primary: bundle(lang, resources(lang)),
            fallback: bundle("es", resources("es")),
        }
    }
    pub fn t(&self, key: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let mut arguments = FluentArgs::new();
        for (name, value) in args {
            arguments.set(*name, value.clone());
        }
        for bundle in [&self.primary, &self.fallback] {
            if let Some(message) = bundle.get_message(key)
                && let Some(pattern) = message.value()
            {
                let mut errors = Vec::new();
                let text = bundle.format_pattern(pattern, Some(&arguments), &mut errors);
                if errors.is_empty() {
                    return text.into_owned();
                }
            }
        }
        key.to_owned()
    }
}

pub fn keys(lang: &str) -> BTreeSet<String> {
    resources(supported_language(lang))
        .iter()
        .flat_map(message_ids)
        .collect()
}

fn supported_language(lang: &str) -> &str {
    if LANGUAGES.contains(&lang) {
        lang
    } else {
        "es"
    }
}

fn message_ids(resource: &FluentResource) -> Vec<String> {
    resource
        .entries()
        .filter_map(|entry| match entry {
            Entry::Message(message) => Some(message.id.name.to_owned()),
            _ => None,
        })
        .collect()
}

type Sources = Vec<(String, Result<String, String>)>;

/// Recursos del idioma en ejecución: degrada por archivo (omite y avisa del defectuoso).
fn resources(lang: &str) -> Vec<FluentResource> {
    let (resources, problems) = parse_sources(embedded_sources(lang));
    for problem in problems {
        tracing::warn!("{problem}");
    }
    resources
}

/// Carga estricta de todos los `*.ftl` embebidos de un idioma, en orden alfabético.
#[cfg(test)]
fn load_embedded(lang: &str) -> Result<Vec<FluentResource>, String> {
    strict(parse_sources(embedded_sources(lang)))
}

fn embedded_sources(lang: &str) -> Sources {
    let prefix = format!("{lang}/");
    let mut sources: Sources = Resources::iter()
        .filter(|path| path.starts_with(&prefix) && path.ends_with(".ftl"))
        .map(|path| {
            let text = match Resources::get(&path) {
                Some(file) => std::str::from_utf8(file.data.as_ref())
                    .map(str::to_owned)
                    .map_err(|e| format!("{path}: invalid UTF-8: {e}")),
                None => Err(format!("{path}: not readable")),
            };
            (path.to_string(), text)
        })
        .collect();
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    sources
}

#[cfg(test)]
fn strict(
    (resources, problems): (Vec<FluentResource>, Vec<String>),
) -> Result<Vec<FluentResource>, String> {
    match problems.into_iter().next() {
        Some(problem) => Err(problem),
        None => Ok(resources),
    }
}

#[cfg(test)]
fn dir_sources(dir: &std::path::Path) -> Sources {
    let mut sources: Sources = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ftl"))
        .map(|path| {
            let name = path.display().to_string();
            let text = std::fs::read(&path)
                .map_err(|e| format!("{name}: {e}"))
                .and_then(|bytes| {
                    String::from_utf8(bytes).map_err(|e| format!("{name}: invalid UTF-8: {e}"))
                });
            (name, text)
        })
        .collect();
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    sources
}

/// Carga estricta de todos los `*.ftl` de una carpeta.
#[cfg(test)]
fn load_dir(dir: &std::path::Path) -> Result<Vec<FluentResource>, String> {
    strict(parse_sources(dir_sources(dir)))
}

/// Devuelve los recursos válidos y la lista de problemas; los archivos defectuosos
/// o con claves ya definidas en otro archivo se omiten completos.
fn parse_sources(sources: Sources) -> (Vec<FluentResource>, Vec<String>) {
    let mut seen: std::collections::BTreeMap<String, String> = Default::default();
    let mut resources = Vec::new();
    let mut problems = Vec::new();
    'files: for (name, text) in sources {
        let text = match text {
            Ok(text) => text,
            Err(problem) => {
                problems.push(problem);
                continue;
            }
        };
        let resource = match FluentResource::try_new(text) {
            Ok(resource) => resource,
            Err((_, errors)) => {
                problems.push(format!("{name}: invalid Fluent syntax: {errors:?}"));
                continue;
            }
        };
        let ids = message_ids(&resource);
        for id in &ids {
            if let Some(first) = seen.get(id) {
                problems.push(format!("duplicate key `{id}` in {first} and {name}"));
                continue 'files;
            }
        }
        for id in ids {
            seen.insert(id, name.clone());
        }
        resources.push(resource);
    }
    (resources, problems)
}

fn bundle(lang: &str, resources: Vec<FluentResource>) -> FluentBundle<FluentResource> {
    let locale: LanguageIdentifier = lang.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![locale]);
    for resource in resources {
        let _ = bundle.add_resource(resource);
    }
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_languages_have_same_keys_as_es() {
        let reference = keys("es");
        assert!(reference.contains("app-name"));
        assert!(reference.contains("alert-module-crashed"));
        for language in LANGUAGES {
            assert_eq!(keys(language), reference, "{language}");
        }
    }

    #[test]
    fn formats_args() {
        let i18n = I18n::new("es");
        assert!(
            i18n.t("alert-hull-damage", &[("pct", FluentValue::from(25))])
                .contains("25")
        );
    }

    #[test]
    fn unknown_language_falls_back_to_es() {
        let expected = I18n::new("es").t("settings-language", &[]);
        assert_eq!(I18n::new("xx").t("settings-language", &[]), expected);
        assert_eq!(keys("xx"), keys("es"));
    }

    #[test]
    fn missing_key_returns_key() {
        assert_eq!(
            I18n::new("es").t("synthetic-missing-key", &[]),
            "synthetic-missing-key"
        );
    }

    #[test]
    fn missing_translation_falls_back_to_spanish() {
        let i18n = I18n {
            primary: bundle(
                "en",
                vec![FluentResource::try_new("other = Other".into()).unwrap()],
            ),
            fallback: bundle(
                "es",
                vec![FluentResource::try_new("missing = Traducción española".into()).unwrap()],
            ),
        };
        assert_eq!(i18n.t("missing", &[]), "Traducción española");
    }

    #[test]
    fn embedded_resources_parse_and_format_all_messages_without_errors() {
        for lang in LANGUAGES {
            let mut bundle = FluentBundle::new_concurrent(vec![lang.parse().unwrap()]);
            for resource in load_embedded(lang).unwrap() {
                bundle.add_resource(resource).unwrap();
            }
            let mut args = FluentArgs::new();
            for (name, value) in [
                ("pct", FluentValue::from(25)),
                ("module", FluentValue::from("dashboard")),
                ("system", FluentValue::from("Sol")),
                ("done", FluentValue::from(3)),
                ("scanned", FluentValue::from(3)),
                ("total", FluentValue::from(9)),
                ("age", FluentValue::from("2 min")),
                ("value", FluentValue::from(60)),
                ("count", FluentValue::from(5)),
                ("destination", FluentValue::from("Colonia")),
                ("current", FluentValue::from(2)),
                ("completed", FluentValue::from(1)),
                ("signals", FluentValue::from(3)),
                ("genus", FluentValue::from("Bacterium")),
                ("genera", FluentValue::from("Bacterium, Stratum")),
                ("n", FluentValue::from(2)),
                ("meters", FluentValue::from("160 m")),
                ("dir", FluentValue::from("a tu espalda")),
                ("compass", FluentValue::from("SO")),
                ("name", FluentValue::from("Bacterium Aurasus")),
                ("after", FluentValue::from("5 CR")),
                ("species", FluentValue::from("Bacterium Aurasus")),
                ("reward", FluentValue::from("1,2 M CR")),
                ("credits", FluentValue::from("3,4 M CR")),
                ("price", FluentValue::from("5200 CR")),
                ("rate", FluentValue::from("120 t/h")),
                ("tons", FluentValue::from(12)),
                ("fee", FluentValue::from("25 %")),
                ("faction", FluentValue::from("Sol Workers")),
                ("ship", FluentValue::from("Vulture")),
                ("material", FluentValue::from("Painita")),
                ("remaining", FluentValue::from("2 min")),
                ("distance", FluentValue::from("12 ly")),
                ("days", FluentValue::from(3)),
                ("time", FluentValue::from("00:45")),
                ("station", FluentValue::from("Jameson Memorial")),
            ] {
                args.set(name, value);
            }
            for key in keys(lang) {
                let message = bundle.get_message(&key).unwrap();
                let mut errors = Vec::new();
                bundle.format_pattern(message.value().unwrap(), Some(&args), &mut errors);
                assert!(errors.is_empty(), "{lang}: {key}: {errors:?}");
            }
        }
    }

    #[test]
    fn resources_contain_all_required_f0_keys_and_translate_each_language() {
        let required = "app-name nav-dashboard nav-settings panel-cmdr panel-ship panel-location panel-status panel-route panel-session panel-alerts field-credits field-hull field-fuel field-cargo field-rebuy field-jump-range field-system field-body field-station field-next-system field-remaining-jumps field-session-time field-jumps field-distance field-credits-earned mode-docked mode-landed mode-normal-space mode-supercruise mode-hyperspace mode-on-foot mode-in-srv mode-in-fighter mode-in-taxi mode-multicrew mode-unknown flag-shields flag-hardpoints flag-landing-gear flag-mass-locked flag-silent-running flag-cargo-scoop flag-sco alert-low-fuel alert-overheating alert-interdiction alert-under-attack alert-hull-damage alert-low-oxygen alert-low-health alert-module-crashed action-copy-next-system toast-copied import-progress game-not-running galaxy-legacy-warning settings-language settings-lan settings-lan-help settings-pairing-qr settings-auto-copy settings-theme theme-hud theme-classic tray-open tray-quit empty-no-data";
        let reference = keys("es");
        for key in required.split_whitespace() {
            assert!(reference.contains(key), "{key}");
        }
        let spanish = ["settings-language", "field-fuel"].map(|key| I18n::new("es").t(key, &[]));
        for language in ["en", "fr", "de", "pt"] {
            let translated =
                ["settings-language", "field-fuel"].map(|key| I18n::new(language).t(key, &[]));
            assert_ne!(translated, spanish, "{language}");
        }
    }

    #[test]
    fn combat_and_mining_resources_load_in_all_languages() {
        let reference = keys("es");
        for key in [
            "panel-combat",
            "panel-mining",
            "layout-combat",
            "layout-mining",
            "combat-badge-clean",
            "combat-badge-wanted",
            "combat-subsystem-powerplant",
            "combat-subsystem-other",
            "mining-verdict-worth",
            "mining-verdict-not-worth",
            "mining-no-data",
        ] {
            assert!(reference.contains(key), "{key}");
        }
        assert_eq!(
            I18n::new("es").t("combat-badge-clean", &[]),
            "LIMPIO · NO DISPARAR"
        );
        assert_eq!(
            I18n::new("en").t("combat-badge-clean", &[]),
            "CLEAN · DO NOT FIRE"
        );
        let wanted = I18n::new("en").t(
            "combat-badge-wanted",
            &[("reward", FluentValue::from("50 CR"))],
        );
        assert!(
            wanted.contains("WANTED") && wanted.contains("50 CR"),
            "{wanted}"
        );
        for language in LANGUAGES {
            assert_eq!(keys(language), reference, "{language}");
        }
    }

    #[test]
    fn french_and_portuguese_plurals_use_the_number_for_zero() {
        for (lang, key, var) in [
            ("fr", "combat-kills", "count"),
            ("pt", "combat-kills", "count"),
            ("fr", "mining-limpets-left", "count"),
            ("pt", "mining-limpets-left", "count"),
            ("fr", "mining-price-age", "days"),
            ("pt", "mining-price-age", "days"),
        ] {
            let text = I18n::new(lang).t(key, &[(var, FluentValue::from(0))]);
            assert!(text.contains('0'), "{lang} {key}: {text}");
        }
        let fr = I18n::new("fr").t("combat-kills", &[("count", FluentValue::from(0))]);
        assert!(fr.ends_with("destruction") && !fr.ends_with("destructions"), "{fr}");
    }

    #[test]
    fn keys_include_all_ftl_files() {
        assert!(keys("es").contains("panel-route-strip"));
        assert!(keys("de").contains("panel-system-bodies"));
        assert!(keys("pt").contains("panel-exobiology"));
        assert_eq!(
            I18n::new("fr").t("panel-system-bodies", &[]),
            "Corps du système"
        );
    }

    #[test]
    fn no_key_is_defined_in_two_files() {
        let dir = std::env::temp_dir().join(format!("tmc-i18n-dup-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.ftl"),
            "foo = 1
",
        )
        .unwrap();
        std::fs::write(
            dir.join("b.ftl"),
            "foo = 2
",
        )
        .unwrap();
        let error = load_dir(&dir).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(error.contains("foo"), "{error}");
        for lang in LANGUAGES {
            assert!(load_embedded(lang).is_ok(), "{lang}");
        }
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tmc-i18n-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn invalid_utf8_file_is_an_error_naming_the_file() {
        let dir = temp_dir("utf8");
        std::fs::write(
            dir.join("good.ftl"),
            "ok = 1
",
        )
        .unwrap();
        std::fs::write(dir.join("broken.ftl"), [0x66u8, 0xff, 0xfe, 0x3d]).unwrap();
        let error = load_dir(&dir).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(error.contains("broken.ftl"), "{error}");
    }

    #[test]
    fn runtime_loading_skips_broken_file_and_keeps_good_ones() {
        let dir = temp_dir("lenient");
        std::fs::write(
            dir.join("a.ftl"),
            "good = 1
",
        )
        .unwrap();
        std::fs::write(dir.join("b.ftl"), [0xffu8, 0xfe]).unwrap();
        std::fs::write(
            dir.join("c.ftl"),
            "good = 2
other = 3
",
        )
        .unwrap();
        std::fs::write(
            dir.join("d.ftl"),
            "last = 4
",
        )
        .unwrap();
        let (resources, problems) = parse_sources(dir_sources(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
        let ids: Vec<String> = resources.iter().flat_map(message_ids).collect();
        assert_eq!(ids, ["good", "last"]);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].contains("b.ftl"));
        assert!(problems[1].contains("good"));
    }

    #[test]
    fn localizer_is_send_and_sync() {
        fn check<T: Send + Sync>() {}
        check::<I18n>();
    }
}
