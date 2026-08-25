use std::time::Duration;

use lean_multisig_comparison::{
    BenchmarkReport, ComparisonReport, RunConfig, SampleSummary, SupplementalReport,
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
fn config_defaults_to_practical_sizes_and_three_samples() {
    let config = RunConfig::parse_from(["slow-comparison"]).unwrap();

    assert_eq!(config.samples, 3);
    assert_eq!(config.sizes, vec![1, 8, 16]);
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
    ])
    .unwrap();

    assert_eq!(config.samples, 2);
    assert_eq!(config.sizes, vec![1, 8]);
    assert_eq!(config.json_path.unwrap().to_str(), Some("/tmp/report.json"));
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
    ] {
        assert!(RunConfig::parse_from(arguments).is_err());
    }
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
        comparisons: vec![ComparisonReport {
            workload: "same_claim_aggregate".to_owned(),
            input_size: 8,
            lean_over_lighthouse: lean.ratio_to(&lighthouse).unwrap(),
            lean,
            lighthouse: lighthouse.clone(),
            lean_artifact_bytes: 1_024,
            lighthouse_artifact_bytes: 96,
        }],
        supplemental: vec![SupplementalReport {
            workload: "verify_signature_sets".to_owned(),
            input_size: 8,
            lighthouse,
        }],
    };

    let json = serde_json::to_string_pretty(&report).unwrap();
    let restored: BenchmarkReport = serde_json::from_str(&json).unwrap();

    assert_eq!(restored, report);
}
