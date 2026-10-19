//! `tmc-headless eval`: métricas agregadas de los criterios 3, 4 y 6 de F1
//! (spec §10.1) sobre una carpeta de diarios. No imprime nombres de
//! comandante ni de sistemas: solo números y símbolos de datos del juego.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use tmc_model::events::{JournalEvent, parse_line};
use tmc_modules::eval::{EvalReport, eval_events};
use tmc_store::{ImportOptions, Store, import_history};

/// Mínimo de casos para dar un veredicto en vez de `INSUFICIENTE`.
const MIN_CASES: u32 = 5;
const RANGE_TOLERANCE_LY: f64 = 0.05;
const VALUE_TOLERANCE: f64 = 0.05;
const PREDICTION_THRESHOLD: f64 = 0.70;
/// Corte superior para la consulta de visitas (todo el historial).
const FAR_FUTURE: &str = "9999-12-31T23:59:59.999999999Z";

/// Duración (ms) de las consultas de historial de la spec §7.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timings {
    pub visits_ms: f64,
    pub unsold_ms: f64,
    pub bio_ms: f64,
    pub events: u64,
}

pub struct Loaded {
    pub events: Vec<JournalEvent>,
    pub files: usize,
    pub skipped_lines: u64,
}

/// Lee todos los diarios de la carpeta en orden cronológico.
pub fn load_events(dir: &Path) -> std::io::Result<Loaded> {
    let paths = tmc_journal::list_journals(dir)?;
    let mut events = Vec::new();
    let mut skipped_lines = 0;
    for path in &paths {
        let bytes = std::fs::read(path)?;
        for line in String::from_utf8_lossy(&bytes).lines() {
            if line.trim().is_empty() {
                continue;
            }
            match parse_line(line) {
                Ok(event) => events.push(event),
                Err(_) => skipped_lines += 1,
            }
        }
    }
    Ok(Loaded {
        events,
        files: paths.len(),
        skipped_lines,
    })
}

#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Pass,
    Fail,
    Insufficient,
}

impl Verdict {
    fn label(&self, cases: u32) -> String {
        match self {
            Self::Pass => "PASS".to_owned(),
            Self::Fail => "FAIL".to_owned(),
            Self::Insufficient => format!("INSUFICIENTE ({cases} casos)"),
        }
    }
}

fn range_verdict(r: &tmc_modules::eval::RangeReport) -> Verdict {
    if r.cases < MIN_CASES {
        Verdict::Insufficient
    } else if r.within_tolerance < r.cases {
        Verdict::Fail
    } else if r.engineered_cases == 0 || r.guardian_cases == 0 {
        // Todos los casos disponibles coinciden, pero faltan ingeniería/Guardian.
        Verdict::Insufficient
    } else {
        Verdict::Pass
    }
}

fn value_verdict(v: &tmc_modules::eval::ValueReport) -> Verdict {
    if v.sales == 0 {
        Verdict::Insufficient
    } else if v.rel_error <= VALUE_TOLERANCE {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

fn prediction_verdict(p: &tmc_modules::eval::PredictionReport) -> Verdict {
    if p.bodies < MIN_CASES {
        Verdict::Insufficient
    } else if p.ratio >= PREDICTION_THRESHOLD {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

/// Texto del informe; solo agregados y símbolos de datos del juego.
pub fn render(
    report: &EvalReport,
    loaded: (usize, usize, u64),
    timings: Option<&Timings>,
) -> String {
    let (files, events, skipped) = loaded;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "TMC eval: {files} diarios, {events} eventos, {skipped} líneas descartadas"
    );
    let r = &report.range;
    let _ = writeln!(
        out,
        "\nCriterio 3, alcance de salto (±{RANGE_TOLERANCE_LY} al): {}",
        range_verdict(r).label(r.cases)
    );
    if range_verdict(r) == Verdict::Insufficient
        && r.cases >= MIN_CASES
        && r.within_tolerance == r.cases
    {
        let mut missing = Vec::new();
        if r.engineered_cases == 0 {
            missing.push("ingeniería");
        }
        if r.guardian_cases == 0 {
            missing.push("Guardian");
        }
        let _ = writeln!(
            out,
            "  motivo: los {} casos disponibles coinciden, pero el historial no incluye {}",
            r.cases,
            missing.join(" ni ")
        );
    }
    let _ = writeln!(
        out,
        "  casos={} dentro_de_tolerancia={} con_ingenieria={} con_guardian={}",
        r.cases, r.within_tolerance, r.engineered_cases, r.guardian_cases
    );
    let _ = writeln!(
        out,
        "  error_max={:.4} al error_medio={:.4} al (fórmula con tope por salto)",
        r.max_abs_error, r.mean_abs_error
    );
    let _ = writeln!(
        out,
        "  aciertos_modelo_de_produccion_con_masa_de_referencia={}",
        r.max_fuel_mass_hits
    );
    let fitted = r.fsd_stats.iter().filter(|f| f.fitted).count();
    let _ = writeln!(
        out,
        "  fsd_vistos={} ajustados_con_este_historial={} preexistentes={}",
        r.fsd_stats.len(),
        fitted,
        r.fsd_stats.len() - fitted
    );
    for f in &r.fsd_stats {
        let _ = writeln!(
            out,
            "    {}: loadouts={} masas_distintas={} ajustado={} verificado={}",
            f.item, f.loadouts, f.distinct_masses, f.fitted, f.verified
        );
    }
    if fitted > 0 {
        let _ = writeln!(
            out,
            "  nota: las entradas ajustadas con este historial hacen el criterio parcialmente circular (consistencia, no prueba independiente)"
        );
    }
    let _ = writeln!(out, "  fsd_fuera_de_tabla={}", join(&report.missing_fsd));

    let v = &report.value;
    let _ = writeln!(
        out,
        "\nCriterio 4, valores de exploración (±{:.0} %): {}",
        VALUE_TOLERANCE * 100.0,
        value_verdict(v).label(v.sales)
    );
    let _ = writeln!(
        out,
        "  ventas_cubiertas={} (en_portaaviones={}) calculado={} cobrado={} error_relativo={:.2} % ventas_sin_cobertura={}",
        v.sales,
        v.carrier_sales,
        v.computed,
        v.paid,
        v.rel_error * 100.0,
        v.uncovered_sales
    );
    let _ = writeln!(
        out,
        "  bruto(base+bonus)={} error_relativo_bruto={:.2} %",
        v.gross,
        v.gross_rel_error * 100.0
    );
    let probes = &report.probes;
    let _ = writeln!(
        out,
        "  sondas: casos={} error_medio={:.2}",
        probes.cases, probes.mean_abs_error
    );

    let p = &report.prediction;
    let _ = writeln!(
        out,
        "\nCriterio 6, predicción de especies (≥ {:.0} %): {}",
        PREDICTION_THRESHOLD * 100.0,
        prediction_verdict(p).label(p.bodies)
    );
    let _ = writeln!(
        out,
        "  casos_cuerpo_especie={} alta_o_media={} ratio={:.1} %",
        p.bodies,
        p.hit_high_or_medium,
        p.ratio * 100.0
    );
    let count = |level| p.likelihoods.iter().filter(|l| **l == level).count();
    use tmc_modules::exobiology::predict::Likelihood::{High, Low, Medium};
    let _ = writeln!(
        out,
        "  alta={} media={} baja={} no_candidata={}",
        count(Some(High)),
        count(Some(Medium)),
        count(Some(Low)),
        count(None) + count(Some(tmc_modules::exobiology::predict::Likelihood::Unknown))
    );
    let _ = writeln!(
        out,
        "  bonus_x5_primer_registro: casos={} acierto={:.1} %",
        p.first_logged_cases,
        p.first_logged_accuracy * 100.0
    );

    let s = &report.species_values;
    let _ = writeln!(
        out,
        "\nCalidad de datos de especies (valor de venta frente a species.json)"
    );
    let _ = writeln!(
        out,
        "  entradas={} coinciden={} difieren={} sin_tabla={}",
        s.entries, s.matches, s.mismatches, s.unknown
    );
    let _ = writeln!(out, "  especies_que_difieren={}", join(&s.mismatch_symbols));
    let _ = writeln!(out, "  especies_sin_tabla={}", join(&s.unknown_symbols));

    render_vouchers(&mut out, &report.vouchers);
    render_yield(&mut out, &report.mining);

    if let Some(t) = timings {
        let _ = writeln!(
            out,
            "\nTiempos de consultas de historial ({} eventos importados, SQLite en memoria)",
            t.events
        );
        let _ = writeln!(out, "  visitas={:.1} ms", t.visits_ms);
        let _ = writeln!(out, "  sin_vender={:.1} ms", t.unsold_ms);
        let _ = writeln!(out, "  estadistica_bio={:.1} ms", t.bio_ms);
    }
    out
}

/// Un exceso de canje (diferencia negativa) no es un fallo demostrable: el
/// historial no ve los vales ganados antes del primer diario, así que es
/// `INSUFICIENTE`. Solo una diferencia positiva (el libro perdió importe que
/// nadie canjeó ni se perdió al morir) es una inconsistencia real.
fn voucher_verdict(v: &tmc_modules::eval::VoucherReport) -> Verdict {
    let diffs = [v.bounty.difference, v.combat_bond.difference];
    if diffs.iter().any(|d| *d > 0) {
        Verdict::Fail
    } else if v.redeem_events == 0 || diffs.iter().any(|d| *d < 0) {
        Verdict::Insufficient
    } else {
        Verdict::Pass
    }
}

fn render_vouchers(out: &mut String, v: &tmc_modules::eval::VoucherReport) {
    let verdict = voucher_verdict(v);
    let _ = writeln!(
        out,
        "\nF2, conciliación de vales (ganado frente a canjeado): {}",
        verdict.label(v.redeem_events)
    );
    let _ = writeln!(out, "  canjes={}", v.redeem_events);
    for (label, k) in [("bounty", &v.bounty), ("combat_bond", &v.combat_bond)] {
        let _ = writeln!(
            out,
            "  {label}: ganado={} canjeado={} perdido_al_morir={} pendiente={} diferencia={}",
            k.earned, k.redeemed, k.lost, k.pending, k.difference
        );
    }
    let _ = writeln!(
        out,
        "  nota: diferencia = ganado - perdido - canjeado - pendiente; negativa si se canjearon vales anteriores al historial; la atribución de refinados solo se reinicia en LoadGame"
    );
}

fn render_yield(out: &mut String, m: &tmc_modules::eval::MiningYieldReport) {
    let _ = writeln!(
        out,
        "\nF2, toneladas refinadas por asteroide prospectado (heurística: cada refinado se atribuye al último asteroide prospectado de la sesión)"
    );
    let current = tmc_modules::data::mining_yield();
    use tmc_model::events::AsteroidContent::{High, Low, Medium};
    for (label, content, level) in [
        ("Low", Low, &m.low),
        ("Medium", Medium, &m.medium),
        ("High", High, &m.high),
    ] {
        let status = if level.asteroids < MIN_CASES {
            format!("INSUFICIENTE ({} casos)", level.asteroids)
        } else {
            format!(
                "propuesta={:.1} (actual={:.1}, verified sigue false)",
                level.mean,
                current.tons_for(content).unwrap_or_default()
            )
        };
        let _ = writeln!(
            out,
            "  {label}: n={} toneladas={} media={:.2} -> {status}",
            level.asteroids, level.tons, level.mean
        );
    }
    let _ = writeln!(
        out,
        "  contenido_desconocido: n={} toneladas={} sin_asteroide_previo_toneladas={}",
        m.unknown.asteroids, m.unknown.tons, m.unattributed_tons
    );
    let _ = writeln!(
        out,
        "  nota: mining_yield.json no se modifica automáticamente"
    );
}

fn join(items: &[String]) -> String {
    if items.is_empty() {
        "ninguno".to_owned()
    } else {
        items.join(", ")
    }
}

fn time_ms(
    f: impl FnOnce() -> Result<serde_json::Value, tmc_store::StoreError>,
) -> anyhow::Result<f64> {
    let start = Instant::now();
    f()?;
    Ok(start.elapsed().as_secs_f64() * 1000.0)
}

/// Importa los diarios a una base en memoria y mide las tres consultas con el
/// comandante con más eventos.
pub fn measure_timings(dir: &Path) -> anyhow::Result<Option<Timings>> {
    let store = Store::open_in_memory()?;
    let opts = ImportOptions {
        throttle: Arc::new(AtomicBool::new(false)),
    };
    import_history(&store, dir, &opts, &mut |_, _| {})?;
    let rows = store.query_json(
        "SELECT cmdr, COUNT(*) FROM event WHERE cmdr IS NOT NULL GROUP BY cmdr ORDER BY 2 DESC LIMIT 1",
        &[],
    )?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };
    let cmdr = row[0].as_str().unwrap_or_default().to_owned();
    let events = row[1].as_u64().unwrap_or_default();
    let visits_ms = time_ms(|| {
        store
            .query_json(
                tmc_modules::navigation::VISITS_SQL,
                &[cmdr.clone().into(), FAR_FUTURE.into()],
            )
            .map(serde_json::Value::from)
    })?;
    let unsold_ms =
        time_ms(|| tmc_modules::exploration::unsold::rebuild_from_history(cmdr.clone())(&store))?;
    let bio_ms = time_ms(|| tmc_modules::exobiology::predict::load_history(cmdr.clone())(&store))?;
    Ok(Some(Timings {
        visits_ms,
        unsold_ms,
        bio_ms,
        events,
    }))
}

/// Punto de entrada del subcomando; `args` son los argumentos tras `eval`.
pub fn run(args: impl Iterator<Item = String>) -> anyhow::Result<()> {
    let mut dir = None;
    let mut timings = false;
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--journal-dir" => dir = args.next().map(std::path::PathBuf::from),
            "--timings" => timings = true,
            other => anyhow::bail!("argumento desconocido: {other}"),
        }
    }
    let dir = dir
        .or_else(tmc_journal::default_journal_dir)
        .ok_or_else(|| anyhow::anyhow!("falta --journal-dir"))?;
    let loaded = load_events(&dir)?;
    let counts = (loaded.files, loaded.events.len(), loaded.skipped_lines);
    let report = eval_events(loaded.events.into_iter());
    let timings = if timings {
        measure_timings(&dir)?
    } else {
        None
    };
    print!("{}", render(&report, counts, timings.as_ref()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir() -> std::path::PathBuf {
        let dir = tempfile::tempdir().unwrap().keep();
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/journal/exploration_session.log");
        std::fs::copy(src, dir.join("Journal.2026-04-10T200000.01.log")).unwrap();
        dir
    }

    #[test]
    fn eval_report_contains_no_names() {
        let dir = fixture_dir();
        let loaded = load_events(&dir).unwrap();
        assert_eq!(loaded.files, 1);
        let counts = (loaded.files, loaded.events.len(), loaded.skipped_lines);
        let report = eval_events(loaded.events.into_iter());
        let timings = measure_timings(&dir).unwrap();
        let text = render(&report, counts, timings.as_ref());
        for forbidden in [
            "CMDR TESTER",
            "TESTER",
            "Kappa Testis",
            "Sol Testis",
            "Sys A",
            "Test Orbital",
        ] {
            assert!(!text.contains(forbidden), "{forbidden} in:\n{text}");
        }
        assert!(text.contains("Criterio 3"), "{text}");
        assert!(text.contains("INSUFICIENTE (0 casos)"), "{text}");
        assert!(text.contains("visitas="), "{text}");
        std::fs::remove_dir_all(dir).ok();
    }

    fn fixture_report(name: &str) -> tmc_modules::eval::EvalReport {
        let dir = tempfile::tempdir().unwrap().keep();
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/journal")
            .join(name);
        std::fs::copy(src, dir.join("Journal.2026-10-18T100000.01.log")).unwrap();
        let loaded = load_events(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        eval_events(loaded.events.into_iter())
    }

    #[test]
    fn eval_reports_voucher_reconciliation_for_fixture() {
        let report = fixture_report("combat_session.log");
        let v = &report.vouchers;
        assert_eq!(v.redeem_events, 3);
        assert_eq!(v.bounty.difference, 0);
        assert_eq!(v.combat_bond.difference, 0);
        assert_eq!(
            (
                v.bounty.earned,
                v.bounty.redeemed,
                v.bounty.lost,
                v.bounty.pending
            ),
            (720_000, 500_000, 70_000, 150_000)
        );
        assert_eq!(
            (
                v.combat_bond.earned,
                v.combat_bond.redeemed,
                v.combat_bond.pending
            ),
            (90_000, 50_000, 40_000)
        );
        let text = render(&report, (1, 0, 0), None);
        assert!(text.contains("conciliación de vales"), "{text}");
        for forbidden in ["TESTER", "Alpha Test", "Beta Test", "Pirates"] {
            assert!(!text.contains(forbidden), "{forbidden} in:\n{text}");
        }
    }

    #[test]
    fn eval_reports_tons_per_asteroid_by_content() {
        let report = fixture_report("mining_session.log");
        let m = &report.mining;
        assert_eq!(m.high.asteroids, 3);
        assert_eq!(m.high.tons, 1);
        assert!((m.high.mean - 1.0 / 3.0).abs() < 1e-12);
        assert_eq!((m.medium.asteroids, m.medium.tons), (3, 3));
        assert_eq!((m.low.asteroids, m.low.tons), (4, 3));
        assert_eq!(m.unattributed_tons, 0);
        let text = render(&report, (1, 0, 0), None);
        assert!(text.contains("INSUFICIENTE (3 casos)"), "{text}");
    }

    #[test]
    fn voucher_excess_redeem_is_insufficient_not_fail() {
        use tmc_modules::eval::{VoucherKindReport, VoucherReport};
        let mut v = VoucherReport {
            redeem_events: 1,
            bounty: VoucherKindReport {
                earned: 100,
                redeemed: 150,
                difference: -50,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(voucher_verdict(&v), Verdict::Insufficient);
        v.bounty.difference = 0;
        assert_eq!(voucher_verdict(&v), Verdict::Pass);
        v.bounty.difference = 10;
        assert_eq!(voucher_verdict(&v), Verdict::Fail);
    }

    #[test]
    fn verdicts_use_thresholds_and_report_insufficient_data() {
        use tmc_modules::eval::{PredictionReport, RangeReport, ValueReport};
        let mut range = RangeReport {
            cases: 5,
            within_tolerance: 5,
            engineered_cases: 1,
            guardian_cases: 1,
            ..Default::default()
        };
        assert_eq!(range_verdict(&range), Verdict::Pass);
        range.within_tolerance = 4;
        assert_eq!(range_verdict(&range), Verdict::Fail);
        range.cases = 4;
        assert_eq!(range_verdict(&range), Verdict::Insufficient);
        let value = ValueReport {
            sales: 2,
            rel_error: 0.051,
            ..Default::default()
        };
        assert_eq!(value_verdict(&value), Verdict::Fail);
        let prediction = PredictionReport {
            bodies: 10,
            ratio: 0.7,
            ..Default::default()
        };
        assert_eq!(prediction_verdict(&prediction), Verdict::Pass);
    }
}
