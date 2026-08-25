import importlib.util
import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path


SCRIPT_PATH = Path(__file__).with_name("build_benchmark_dashboard.py")
DASHBOARD_PATH = Path(__file__).parents[1] / "benchmarks/comparison/dashboard/index.html"


def load_dashboard_module():
    spec = importlib.util.spec_from_file_location("build_benchmark_dashboard", SCRIPT_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("failed to load dashboard builder")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class DashboardBuilderTests(unittest.TestCase):
    def test_static_dashboard_is_latest_only_and_loads_both_suites(self):
        html = DASHBOARD_PATH.read_text(encoding="utf-8")
        lowered = html.lower()
        self.assertIn("leanvm multisig vs lighthouse bls", lowered)
        self.assertIn('id="fast-results"', html)
        self.assertIn('id="slow-results"', html)
        self.assertIn('loadSuite("fast", renderFast)', html)
        self.assertIn('loadSuite("slow", renderSlow)', html)
        self.assertIn('fetch(`data/${suite}.json`', html)
        self.assertIn("textContent", html)
        self.assertIn("overflow-x: auto", html)
        self.assertIn("LeanVM ops/s", html)
        self.assertIn("BLS ops/s", html)
        self.assertNotIn("innerHTML", html)
        self.assertNotIn("<canvas", lowered)
        self.assertNotIn("<svg", lowered)
        self.assertNotIn("chart", lowered)
        self.assertNotIn("dev/bench", lowered)

    def test_dashboard_folds_input_size_into_operation_names(self):
        html = DASHBOARD_PATH.read_text(encoding="utf-8")
        self.assertIn("function operationLabel(workload, inputSize)", html)
        self.assertIn("(size = ${inputSize})", html)
        self.assertNotIn('["Operation", "Size"', html)

    def test_dashboard_explains_benchmark_terms_after_results(self):
        html = DASHBOARD_PATH.read_text(encoding="utf-8")
        lowered = html.lower()
        self.assertIn('id="explainer"', html)
        self.assertGreater(html.index('id="explainer"'), html.index('id="slow-results"'))
        self.assertIn("same-claim aggregation", lowered)
        self.assertIn("same message", lowered)
        self.assertIn("distinct-claim aggregation", lowered)
        self.assertIn("proof size", lowered)
        self.assertIn("peak rss", lowered)

    def test_dashboard_names_the_slow_section_by_what_it_measures(self):
        html = DASHBOARD_PATH.read_text(encoding="utf-8")
        self.assertIn("Aggregate proof generation and verification", html)
        self.assertNotIn("Proof-backed operations", html)

    def test_dashboard_metadata_wraps_and_omits_secondary_details(self):
        html = DASHBOARD_PATH.read_text(encoding="utf-8")
        self.assertIn("overflow-wrap: anywhere", html)
        self.assertIn("white-space: normal", html)
        self.assertNotIn("text-overflow: ellipsis", html)
        self.assertNotIn('<div class="legend"', html)
        self.assertNotIn('["Runner"', html)
        self.assertNotIn('["Same-claim sizes"', html)
        self.assertNotIn('["Distinct-claim sizes"', html)
        self.assertNotIn('["Samples", data.environment?.samples', html)
        self.assertIn('["Samples", String(data.samples)]', html)

    def test_dashboard_splits_results_into_operation_family_tables(self):
        html = DASHBOARD_PATH.read_text(encoding="utf-8")
        self.assertIn("function addGroupedTables", html)
        self.assertIn('"Basic operations"', html)
        self.assertIn('"Same-claim aggregation"', html)
        self.assertIn('"Same-claim verification"', html)
        self.assertIn('"Distinct-claim aggregation"', html)
        self.assertIn('"Distinct-claim verification"', html)
        self.assertIn('|| workload === "signature_sets_verify"', html)

    def test_dashboard_labels_proof_and_signature_sizes_explicitly(self):
        html = DASHBOARD_PATH.read_text(encoding="utf-8")
        self.assertIn("LeanVM proof size", html)
        self.assertIn("BLS signature size", html)
        self.assertNotIn("Lean artifact", html)
        self.assertNotIn("BLS artifact", html)

    def test_dashboard_uses_leanvm_in_user_facing_copy(self):
        html = DASHBOARD_PATH.read_text(encoding="utf-8")
        for old_label in (
            "Lean median",
            "Lean / BLS",
            "Lean ops/s",
            "Lean proof size",
            "Full Lean proof",
            "Lean checks",
        ):
            self.assertNotIn(old_label, html)
        self.assertIn("LeanVM median", html)
        self.assertIn("LeanVM / BLS", html)
        self.assertIn("LeanVM ops/s", html)
        self.assertIn("LeanVM proof size", html)

    def test_fast_command_normalizes_real_bencher_rows(self):
        dashboard = load_dashboard_module()
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            criterion = directory / "criterion.txt"
            environment = directory / "environment.txt"
            output = directory / "fast.json"
            criterion.write_text(
                "\n".join(
                    [
                        "Compiling dependencies",
                        "test key_creation/lean ... bench:   5,937,262 ns/iter (+/- 658,840)",
                        "test key_creation/lighthouse ... bench:         843 ns/iter (+/- 8)",
                        "test lighthouse_same_claim_aggregate/512 ... bench:     760,779 ns/iter (+/- 5,231)",
                    ]
                ),
                encoding="utf-8",
            )
            environment.write_text(
                "\n".join(
                    [
                        "commit=abc123",
                        "measured_at=2026-08-25T18:00:00Z",
                        "runner_os=Linux",
                        "runner_arch=X64",
                        "runner_image_os=ubuntu24",
                        "runner_image_version=20260823.1",
                        "lighthouse_revision=deadbeef",
                        "suite=fast",
                        "samples=criterion-default",
                        "same_sizes=1,8,16,32,64,128,256,512",
                        "distinct_sizes=1,8,16",
                        "proof_warmup=not-applicable",
                        "",
                        "Model name: AMD EPYC 7763 64-Core Processor",
                    ]
                ),
                encoding="utf-8",
            )

            self.assertEqual(
                dashboard.main(
                    [
                        "fast",
                        "--input",
                        str(criterion),
                        "--environment",
                        str(environment),
                        "--output",
                        str(output),
                    ]
                ),
                0,
            )

            data = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(data["schema_version"], 1)
            self.assertEqual(data["suite"], "fast")
            self.assertEqual(data["measured_at"], "2026-08-25T18:00:00Z")
            self.assertEqual(data["environment"]["commit"], "abc123")
            self.assertEqual(
                data["environment"]["cpu"], "AMD EPYC 7763 64-Core Processor"
            )
            self.assertEqual(
                data["benchmarks"],
                [
                    {
                        "name": "key_creation/lean",
                        "workload": "key_creation",
                        "implementation": "lean",
                        "input_size": None,
                        "median_ns": 5_937_262,
                        "deviation_ns": 658_840,
                    },
                    {
                        "name": "key_creation/lighthouse",
                        "workload": "key_creation",
                        "implementation": "lighthouse",
                        "input_size": None,
                        "median_ns": 843,
                        "deviation_ns": 8,
                    },
                    {
                        "name": "lighthouse_same_claim_aggregate/512",
                        "workload": "same_claim_aggregate",
                        "implementation": "lighthouse",
                        "input_size": 512,
                        "median_ns": 760_779,
                        "deviation_ns": 5_231,
                    },
                ],
            )

    def test_environment_requires_cpu_metadata(self):
        dashboard = load_dashboard_module()
        with tempfile.TemporaryDirectory() as directory:
            environment = Path(directory) / "environment.txt"
            environment.write_text(
                "commit=abc123\nmeasured_at=2026-08-25T18:00:00Z\nsuite=fast\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "cpu"):
                dashboard.parse_environment(environment)

    def test_peak_rss_requires_one_positive_exact_integer(self):
        dashboard = load_dashboard_module()
        cases = {
            "missing": "Elapsed time: 1.0\n",
            "duplicate": (
                "Maximum resident set size (kbytes): 10\n"
                "Maximum resident set size (kbytes): 11\n"
            ),
            "malformed": "Maximum resident set size (kbytes): nope\n",
            "zero": "Maximum resident set size (kbytes): 0\n",
            "overflow": "Maximum resident set size (kbytes): 8796093022208\n",
        }
        with tempfile.TemporaryDirectory() as directory:
            resource = Path(directory) / "resource.txt"
            for name, contents in cases.items():
                with self.subTest(name=name):
                    resource.write_text(contents, encoding="utf-8")
                    with self.assertRaises(ValueError):
                        dashboard.parse_peak_rss(resource)

    def test_fast_sized_benchmark_rejects_zero_input(self):
        dashboard = load_dashboard_module()
        with self.assertRaisesRegex(ValueError, "input size"):
            dashboard.parse_benchmark_name("lighthouse_same_claim_aggregate/0")

    def test_slow_command_combines_report_environment_and_peak_rss(self):
        dashboard = load_dashboard_module()
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            report_path = directory / "full.json"
            resource = directory / "resource-usage.txt"
            environment = directory / "environment.txt"
            output = directory / "slow.json"
            comparison = {
                "workload": "same_claim_aggregate",
                "input_size": 512,
                "lean": {
                    "samples_ns": [2_803_529_591, 2_810_875_903, 2_801_488_105],
                    "median_ns": 2_803_529_591,
                    "operations_per_second": 0.35669322100620554,
                },
                "lighthouse": {
                    "samples_ns": [471_101, 469_438, 465_042],
                    "median_ns": 469_438,
                    "operations_per_second": 2130.2067578679184,
                },
                "lean_over_lighthouse": 5972.097680630882,
                "lean_artifact_bytes": 178_574,
                "lighthouse_artifact_bytes": 96,
            }
            supplemental = {
                "workload": "lighthouse_signature_sets_verify",
                "input_size": 512,
                "lighthouse": {
                    "samples_ns": [800_000, 801_000, 799_000],
                    "median_ns": 800_000,
                    "operations_per_second": 1250.0,
                },
            }
            report_path.write_text(
                json.dumps(
                    {
                        "lighthouse_revision": "deadbeef",
                        "samples": 3,
                        "proof_warmup": True,
                        "comparisons": [comparison],
                        "supplemental": [supplemental],
                    }
                ),
                encoding="utf-8",
            )
            resource.write_text(
                "Maximum resident set size (kbytes): 4600824\n", encoding="utf-8"
            )
            environment.write_text(
                "\n".join(
                    [
                        "commit=abc123",
                        "measured_at=2026-08-25T19:00:00Z",
                        "runner_os=Linux",
                        "runner_arch=X64",
                        "runner_image_os=ubuntu24",
                        "runner_image_version=20260823.1",
                        "suite=slow",
                        "samples=3",
                        "same_sizes=1,8,16,32,64,128,256,512",
                        "distinct_sizes=1,8,16",
                        "proof_warmup=enabled",
                        "lighthouse_revision=deadbeef",
                        "",
                        "Model name: AMD EPYC 7763 64-Core Processor",
                    ]
                ),
                encoding="utf-8",
            )

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                self.assertEqual(
                    dashboard.main(
                        [
                            "slow",
                            "--input",
                            str(report_path),
                            "--resource",
                            str(resource),
                            "--environment",
                            str(environment),
                            "--output",
                            str(output),
                        ]
                    ),
                    0,
                )

            data = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(data["schema_version"], 1)
            self.assertEqual(data["suite"], "slow")
            self.assertEqual(data["measured_at"], "2026-08-25T19:00:00Z")
            self.assertEqual(data["environment"]["commit"], "abc123")
            self.assertEqual(data["lighthouse_revision"], "deadbeef")
            self.assertEqual(data["samples"], 3)
            self.assertTrue(data["proof_warmup"])
            self.assertEqual(data["peak_rss_bytes"], 4_600_824 * 1024)
            self.assertEqual(data["comparisons"], [comparison])
            self.assertEqual(data["supplemental"], [supplemental])
            self.assertIn("4.39 GiB", stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
