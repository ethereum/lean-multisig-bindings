use std::time::Duration;

use lean_multisig_comparison::{
    BenchmarkReport, ComparisonReport, RunConfig, SampleSummary, SupplementalReport,
    LIGHTHOUSE_REVISION, MAX_DISTINCT_CLAIMS,
};

#[test]
fn summary_reports_samples_median_and_throughput() {
    let summary = SampleSummary::from_durations([
        Duration::from_millis(30),
        Duration::from_millis(10),
        Duration::from_millis(20),
    ])
    .unwrap();

    assert_eq!(summary.samples_ns, vec![30_000_000, 10_000_000, 20_000_000]);
    assert_eq!(summary.median_ns, 20_000_000);
    assert_eq!(summary.operations_per_second, 50.0);
}

#[test]
fn summary_rejects_empty_zero_and_oversized_durations() {
    assert!(SampleSummary::from_durations([]).is_err());
    assert!(SampleSummary::from_durations([Duration::ZERO]).is_err());
    assert!(SampleSummary::from_durations([Duration::MAX]).is_err());
}

#[test]
fn summary_ratio_uses_integer_medians() {
    let lean = SampleSummary::from_durations([Duration::from_millis(30)]).unwrap();
    let lighthouse = SampleSummary::from_durations([Duration::from_millis(10)]).unwrap();

    assert_eq!(lean.ratio_to(&lighthouse).unwrap(), 3.0);
}

#[test]
fn comparison_report_calculates_ratio_from_medians() {
    let lean = SampleSummary::from_durations([Duration::from_millis(30)]).unwrap();
    let lighthouse = SampleSummary::from_durations([Duration::from_millis(10)]).unwrap();

    let report =
        ComparisonReport::new("same_claim_aggregate", 8, lean, lighthouse, 1_024, 96).unwrap();

    assert_eq!(report.lean_over_lighthouse, 3.0);
}

#[test]
fn comparison_report_rejects_zero_lighthouse_median() {
    let lean = SampleSummary::from_durations([Duration::from_millis(30)]).unwrap();
    let lighthouse = SampleSummary {
        samples_ns: vec![0],
        median_ns: 0,
        operations_per_second: f64::INFINITY,
    };

    assert!(
        ComparisonReport::new("same_claim_aggregate", 8, lean, lighthouse, 1_024, 96,).is_err()
    );
}

#[test]
fn comparison_report_rejects_zero_sizes() {
    let summary = SampleSummary::from_durations([Duration::from_millis(1)]).unwrap();

    assert!(ComparisonReport::new(
        "same_claim_aggregate",
        0,
        summary.clone(),
        summary.clone(),
        1,
        1,
    )
    .is_err());
    assert!(ComparisonReport::new(
        "same_claim_aggregate",
        MAX_DISTINCT_CLAIMS + 1,
        summary.clone(),
        summary.clone(),
        1,
        1,
    )
    .is_err());
    assert!(ComparisonReport::new(
        "same_claim_aggregate",
        1,
        summary.clone(),
        summary.clone(),
        0,
        1,
    )
    .is_err());
    assert!(
        ComparisonReport::new("same_claim_aggregate", 1, summary.clone(), summary, 1, 0,).is_err()
    );
}

#[test]
fn config_defaults_to_practical_sizes_and_three_samples() {
    let config = RunConfig::parse_from(["slow-comparison"]).unwrap();

    assert_eq!(config.samples, 3);
    assert_eq!(config.sizes, vec![1, 8, 16]);
    assert!(!config.warmup_proofs);
}

#[test]
fn config_accepts_samples_sizes_and_json_path() {
    let config = RunConfig::parse_from([
        "slow-comparison",
        "--samples",
        "2",
        "--sizes",
        "1,8",
        "--json",
        "/tmp/report.json",
        "--warmup-proofs",
    ])
    .unwrap();

    assert_eq!(config.samples, 2);
    assert_eq!(config.sizes, vec![1, 8]);
    assert_eq!(config.json_path.unwrap().to_str(), Some("/tmp/report.json"));
    assert!(config.warmup_proofs);
}

#[test]
fn config_rejects_zero_samples_and_out_of_range_sizes() {
    assert!(RunConfig::parse_from(["slow-comparison", "--samples", "0"]).is_err());
    assert!(RunConfig::parse_from(["slow-comparison", "--sizes", "0"]).is_err());
    assert!(RunConfig::parse_from(["slow-comparison", "--sizes", "17"]).is_err());
}

#[test]
fn config_rejects_unknown_repeated_and_missing_options() {
    for arguments in [
        vec!["slow-comparison", "--unknown"],
        vec!["slow-comparison", "--samples", "2", "--samples", "3"],
        vec!["slow-comparison", "--sizes", "1", "--sizes", "8"],
        vec![
            "slow-comparison",
            "--json",
            "/tmp/one.json",
            "--json",
            "/tmp/two.json",
        ],
        vec!["slow-comparison", "--samples"],
        vec!["slow-comparison", "--sizes"],
        vec!["slow-comparison", "--json"],
        vec!["slow-comparison", "--warmup-proofs", "--warmup-proofs"],
    ] {
        assert!(RunConfig::parse_from(arguments).is_err());
    }
}

#[test]
fn unknown_option_error_lists_the_proof_warmup_flag() {
    let error = RunConfig::parse_from(["slow-comparison", "--unknown"]).unwrap_err();

    assert!(error.to_string().contains("--warmup-proofs"));
}

#[test]
fn config_rejects_malformed_and_duplicate_sizes() {
    for sizes in ["", "1,", ",1", "1,,8", "one", "1,1"] {
        assert!(RunConfig::parse_from(["slow-comparison", "--sizes", sizes]).is_err());
    }
}

#[test]
fn benchmark_report_round_trips_through_json() {
    let lean = SampleSummary::from_durations([Duration::from_millis(30)]).unwrap();
    let lighthouse = SampleSummary::from_durations([Duration::from_millis(10)]).unwrap();
    let report = BenchmarkReport {
        lighthouse_revision: "e423a66763bb1bd780492d635123f208d80c3538".to_owned(),
        samples: 1,
        proof_warmup: true,
        comparisons: vec![ComparisonReport::new(
            "same_claim_aggregate",
            8,
            lean,
            lighthouse.clone(),
            1_024,
            96,
        )
        .unwrap()],
        supplemental: vec![SupplementalReport {
            workload: "verify_signature_sets".to_owned(),
            input_size: 8,
            lighthouse,
        }],
    };

    let json = serde_json::to_string_pretty(&report).unwrap();
    let restored: BenchmarkReport = serde_json::from_str(&json).unwrap();

    assert_eq!(restored, report);
    assert!(restored.proof_warmup);
}

#[test]
fn reported_lighthouse_revision_matches_manifest_pin() {
    let manifest = include_str!("../Cargo.toml");
    let expected = format!("rev = \"{LIGHTHOUSE_REVISION}\"");
    let dependency = manifest
        .lines()
        .find(|line| line.starts_with("lighthouse-bls ="))
        .expect("manifest must contain the Lighthouse BLS dependency");

    assert!(dependency.contains(&expected));
}

#[test]
fn table_contains_paired_measurements_and_artifact_sizes() {
    let lean = SampleSummary::from_durations([Duration::from_millis(2)]).unwrap();
    let lighthouse = SampleSummary::from_durations([Duration::from_micros(500)]).unwrap();
    let report = BenchmarkReport {
        lighthouse_revision: "revision".to_owned(),
        samples: 1,
        proof_warmup: false,
        comparisons: vec![ComparisonReport::new(
            "same_claim_aggregate",
            8,
            lean,
            lighthouse,
            1_024,
            96,
        )
        .unwrap()],
        supplemental: vec![],
    };

    let table = report.to_table();

    assert!(table.contains("same_claim_aggregate"));
    assert!(table.contains('8'));
    assert!(table.contains("2.000 ms"));
    assert!(table.contains("500.000 us"));
    assert!(table.contains("4.00x"));
    assert!(table.contains("500.00 ops/s"));
    assert!(table.contains("2000.00 ops/s"));
    assert!(table.contains("1024"));
    assert!(table.contains("96"));
}

#[test]
fn table_visibly_labels_supplemental_rows_as_lighthouse_only() {
    let lighthouse = SampleSummary::from_durations([Duration::from_micros(500)]).unwrap();
    let report = BenchmarkReport {
        lighthouse_revision: "revision".to_owned(),
        samples: 1,
        proof_warmup: false,
        comparisons: vec![],
        supplemental: vec![SupplementalReport {
            workload: "verify_signature_sets".to_owned(),
            input_size: 8,
            lighthouse,
        }],
    };

    let table = report.to_table();

    assert!(table.contains("verify_signature_sets"));
    assert!(table.contains("Lighthouse-only"));
    assert!(table.contains("500.000 us"));
}
