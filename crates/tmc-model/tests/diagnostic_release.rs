#[cfg(not(debug_assertions))]
#[test]
fn release_diagnostic_does_not_evaluate_or_typecheck_arguments() {
    tmc_model::diagnostic!(does_not_exist(), "release.noop", panic!("must not execute"));
}
