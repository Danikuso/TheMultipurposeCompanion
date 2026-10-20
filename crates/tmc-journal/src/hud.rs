use std::path::Path;
use tmc_model::theme::{HudTheme, Rgb};

pub fn read_hud_theme(path: Option<&Path>) -> HudTheme {
    let base = HudTheme::default();
    let Some(path) = path else {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "source.hud.not_configured",
            tmc_model::diagnostics::json!({"is_default":true})
        );
        return base;
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_error) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "source.hud.read_failed",
                tmc_model::diagnostics::json!({"is_default":true,"failure_class":tmc_model::diagnostics::io_class(&_error)})
            );
            return base;
        }
    };
    let Ok(document) = roxmltree::Document::parse(&text) else {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "source.hud.invalid_xml",
            tmc_model::diagnostics::json!({"is_default":true,"bytes":text.len()})
        );
        return base;
    };
    let Some(red) = read_matrix(&document, "MatrixRed") else {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "source.hud.invalid_red",
            tmc_model::diagnostics::json!({"is_default":true})
        );
        return base;
    };
    let Some(green) = read_matrix(&document, "MatrixGreen") else {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "source.hud.invalid_green",
            tmc_model::diagnostics::json!({"is_default":true})
        );
        return base;
    };
    let Some(blue) = read_matrix(&document, "MatrixBlue") else {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "source.hud.invalid_blue",
            tmc_model::diagnostics::json!({"is_default":true})
        );
        return base;
    };
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::State,
        "source.hud.accepted",
        tmc_model::diagnostics::json!({"red":red,"green":green,"blue":blue})
    );
    let transform = |Rgb(r, g, b): Rgb| {
        let channel = |index: usize| {
            (f64::from(r) * red[index] + f64::from(g) * green[index] + f64::from(b) * blue[index])
                .clamp(0.0, 255.0) as u8
        };
        Rgb(channel(0), channel(1), channel(2))
    };
    HudTheme {
        primary: transform(base.primary),
        secondary: transform(base.secondary),
        background: transform(base.background),
        is_default: false,
        ..base
    }
}

fn read_matrix(document: &roxmltree::Document<'_>, tag: &str) -> Option<[f64; 3]> {
    let node = document.descendants().find(|node| node.has_tag_name(tag))?;
    let values = node.text()?;
    let mut parts = values.split(',');
    let mut result = [0.0; 3];
    for value in &mut result {
        let parsed = parts.next()?.trim().parse::<f32>().ok()?;
        if !parsed.is_finite() {
            return None;
        }
        *value = f64::from(parsed);
    }
    parts.next().is_none().then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn hud_theme_default_when_empty_override() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("GraphicsConfigurationOverride.xml");
        fs::write(&path, "<GraphicsConfig />").unwrap();
        assert_eq!(read_hud_theme(Some(&path)), HudTheme::default());
        assert_eq!(read_hud_theme(None), HudTheme::default());
        assert_eq!(
            read_hud_theme(Some(&dir.path().join("missing"))),
            HudTheme::default()
        );
    }

    #[test]
    fn hud_theme_applies_matrix() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("GraphicsConfigurationOverride.xml");
        fs::write(&path, "<GraphicsConfig><MatrixRed>0,0,1</MatrixRed><MatrixGreen>0,1,0</MatrixGreen><MatrixBlue>1,0,0</MatrixBlue></GraphicsConfig>").unwrap();
        let theme = read_hud_theme(Some(&path));
        assert_eq!(theme.primary.to_css(), "#0071ff");
        assert_eq!(theme.secondary.to_css(), "#3cb4ff");
        assert_eq!(theme.background.to_css(), "#0e0b0b");
        assert_eq!(theme.danger, HudTheme::default().danger);
        assert!(!theme.is_default);
    }

    #[test]
    fn hud_rejects_non_finite_or_incomplete_matrix() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("override.xml");
        for red in ["NaN,0,1", "inf,0,1", "1,0", "1,0,0,1"] {
            fs::write(&path, format!("<GraphicsConfig><MatrixRed>{red}</MatrixRed><MatrixGreen>0,1,0</MatrixGreen><MatrixBlue>0,0,1</MatrixBlue></GraphicsConfig>")).unwrap();
            assert_eq!(read_hud_theme(Some(&path)), HudTheme::default());
        }
    }

    #[test]
    fn hud_ignores_commented_matrices() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("override.xml");
        fs::write(&path, "<GraphicsConfig><!-- <MatrixRed>0,0,1</MatrixRed><MatrixGreen>0,1,0</MatrixGreen><MatrixBlue>1,0,0</MatrixBlue> --></GraphicsConfig>").unwrap();
        assert_eq!(read_hud_theme(Some(&path)), HudTheme::default());
        fs::write(&path, "<GraphicsConfig><!-- <MatrixRed>0,0,1</MatrixRed> --><MatrixRed>1,0,0</MatrixRed><MatrixGreen>0,1,0</MatrixGreen><MatrixBlue>0,0,1</MatrixBlue></GraphicsConfig>").unwrap();
        let theme = read_hud_theme(Some(&path));
        assert_eq!(theme.primary, HudTheme::default().primary);
        assert!(!theme.is_default);
    }

    #[test]
    fn hud_rejects_malformed_and_truncated_xml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("override.xml");
        let matrices = "<MatrixRed>0,0,1</MatrixRed><MatrixGreen>0,1,0</MatrixGreen><MatrixBlue>1,0,0</MatrixBlue>";
        for xml in [
            format!("<GraphicsConfig>{matrices}"),
            format!("<GraphicsConfig>{matrices}</Other>"),
            format!("<GraphicsConfig>{matrices}</GraphicsConfig>broken"),
        ] {
            fs::write(&path, xml).unwrap();
            assert_eq!(read_hud_theme(Some(&path)), HudTheme::default());
        }
    }
}
