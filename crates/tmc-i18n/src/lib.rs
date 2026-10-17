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

/// Recursos válidos del idioma; si la carga falla (no debería: lo cubre un test) queda vacío.
fn resources(lang: &str) -> Vec<FluentResource> {
    load_embedded(lang).unwrap_or_default()
}

/// Carga todos los `*.ftl` embebidos de un idioma, en orden alfabético.
fn load_embedded(lang: &str) -> Result<Vec<FluentResource>, String> {
    let prefix = format!("{lang}/");
    let mut sources: Vec<(String, String)> = Resources::iter()
        .filter(|path| path.starts_with(&prefix) && path.ends_with(".ftl"))
        .filter_map(|path| {
            let file = Resources::get(&path)?;
            let text = std::str::from_utf8(file.data.as_ref()).ok()?.to_owned();
            Some((path.to_string(), text))
        })
        .collect();
    sources.sort();
    parse_sources(sources)
}

/// Carga todos los `*.ftl` de una carpeta, en orden alfabético.
#[cfg(test)]
fn load_dir(dir: &std::path::Path) -> Result<Vec<FluentResource>, String> {
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "ftl") {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            sources.push((path.display().to_string(), text));
        }
    }
    sources.sort();
    parse_sources(sources)
}

fn parse_sources(sources: Vec<(String, String)>) -> Result<Vec<FluentResource>, String> {
    let mut seen: std::collections::BTreeMap<String, String> = Default::default();
    let mut resources = Vec::new();
    for (name, text) in sources {
        let resource = FluentResource::try_new(text)
            .map_err(|(_, errors)| format!("{name}: invalid Fluent syntax: {errors:?}"))?;
        for id in message_ids(&resource) {
            if let Some(first) = seen.insert(id.clone(), name.clone()) {
                return Err(format!("duplicate key `{id}` in {first} and {name}"));
            }
        }
        resources.push(resource);
    }
    Ok(resources)
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
                ("total", FluentValue::from(9)),
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

    #[test]
    fn localizer_is_send_and_sync() {
        fn check<T: Send + Sync>() {}
        check::<I18n>();
    }
}
