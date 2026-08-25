#!/usr/bin/env python3
"""Normalize benchmark artifacts for the latest-results dashboard."""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path


BENCHMARK_RE = re.compile(
    r"^test (?P<name>\S+) \.\.\. bench:\s+"
    r"(?P<median>[\d,]+) ns/iter \(\+/- (?P<deviation>[\d,]+)\)$"
)
PEAK_RSS_RE = re.compile(
    r"^\s*Maximum resident set size \(kbytes\):\s*(?P<value>\S+)\s*$"
)
MAX_EXACT_KIB = (2**53 - 1) // 1024


def parse_environment(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    cpu = None
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("Model name:"):
            cpu = line.partition(":")[2].strip()
        elif "=" in line:
            key, value = line.split("=", 1)
            values[key.strip()] = value.strip()

    required = ("commit", "measured_at", "suite")
    missing = [key for key in required if not values.get(key)]
    if missing:
        raise ValueError(f"environment is missing: {', '.join(missing)}")
    if not cpu:
        raise ValueError("environment is missing: cpu")
    values["cpu"] = cpu
    return values


def parse_benchmark_name(name: str) -> tuple[str, str, int | None]:
    workload, separator, suffix = name.rpartition("/")
    if not separator:
        raise ValueError(f"unrecognized benchmark name: {name}")

    if suffix in {"lean", "lighthouse"}:
        if not workload:
            raise ValueError(f"unrecognized benchmark name: {name}")
        return workload, suffix, None

    if workload.startswith("lighthouse_") and suffix.isdigit():
        normalized_workload = workload.removeprefix("lighthouse_")
        if not normalized_workload:
            raise ValueError(f"unrecognized benchmark name: {name}")
        input_size = int(suffix)
        if input_size <= 0:
            raise ValueError(f"benchmark input size must be positive: {name}")
        return normalized_workload, "lighthouse", input_size

    raise ValueError(f"unrecognized benchmark name: {name}")


def build_fast(input_path: Path, environment_path: Path, output_path: Path) -> None:
    environment = parse_environment(environment_path)
    if environment["suite"] != "fast":
        raise ValueError("environment suite must be fast")

    benchmarks = []
    seen = set()
    for line in input_path.read_text(encoding="utf-8").splitlines():
        match = BENCHMARK_RE.fullmatch(line.strip())
        if not match:
            continue
        name = match.group("name")
        if name in seen:
            raise ValueError(f"duplicate benchmark: {name}")
        seen.add(name)
        workload, implementation, input_size = parse_benchmark_name(name)
        median_ns = int(match.group("median").replace(",", ""))
        deviation_ns = int(match.group("deviation").replace(",", ""))
        if median_ns <= 0 or deviation_ns < 0:
            raise ValueError(f"invalid timing for benchmark: {name}")
        benchmarks.append(
            {
                "name": name,
                "workload": workload,
                "implementation": implementation,
                "input_size": input_size,
                "median_ns": median_ns,
                "deviation_ns": deviation_ns,
            }
        )

    if not benchmarks:
        raise ValueError("benchmark output did not contain any Bencher rows")

    document = {
        "schema_version": 1,
        "suite": "fast",
        "measured_at": environment["measured_at"],
        "environment": {
            key: value for key, value in environment.items() if key != "suite"
        },
        "benchmarks": benchmarks,
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


def positive_integer(value: object, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise ValueError(f"{field} must be a positive integer")
    return value


def positive_number(value: object, field: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{field} must be a positive number")
    converted = float(value)
    if not math.isfinite(converted) or converted <= 0:
        raise ValueError(f"{field} must be a positive finite number")
    return converted


def validate_summary(summary: object, field: str, samples: int) -> None:
    if not isinstance(summary, dict):
        raise ValueError(f"{field} must be an object")
    samples_ns = summary.get("samples_ns")
    if not isinstance(samples_ns, list) or len(samples_ns) != samples:
        raise ValueError(f"{field}.samples_ns must contain {samples} samples")
    for index, value in enumerate(samples_ns):
        positive_integer(value, f"{field}.samples_ns[{index}]")
    positive_integer(summary.get("median_ns"), f"{field}.median_ns")
    positive_number(
        summary.get("operations_per_second"), f"{field}.operations_per_second"
    )


def validate_comparison(row: object, samples: int, index: int) -> None:
    field = f"comparisons[{index}]"
    if not isinstance(row, dict):
        raise ValueError(f"{field} must be an object")
    if not isinstance(row.get("workload"), str) or not row["workload"]:
        raise ValueError(f"{field}.workload must be a non-empty string")
    positive_integer(row.get("input_size"), f"{field}.input_size")
    validate_summary(row.get("lean"), f"{field}.lean", samples)
    validate_summary(row.get("lighthouse"), f"{field}.lighthouse", samples)
    positive_number(
        row.get("lean_over_lighthouse"), f"{field}.lean_over_lighthouse"
    )
    positive_integer(row.get("lean_artifact_bytes"), f"{field}.lean_artifact_bytes")
    positive_integer(
        row.get("lighthouse_artifact_bytes"),
        f"{field}.lighthouse_artifact_bytes",
    )


def validate_supplemental(row: object, samples: int, index: int) -> None:
    field = f"supplemental[{index}]"
    if not isinstance(row, dict):
        raise ValueError(f"{field} must be an object")
    if not isinstance(row.get("workload"), str) or not row["workload"]:
        raise ValueError(f"{field}.workload must be a non-empty string")
    positive_integer(row.get("input_size"), f"{field}.input_size")
    validate_summary(row.get("lighthouse"), f"{field}.lighthouse", samples)


def parse_peak_rss(path: Path) -> int:
    values = []
    for line in path.read_text(encoding="utf-8").splitlines():
        match = PEAK_RSS_RE.fullmatch(line)
        if match:
            values.append(match.group("value"))
    if len(values) != 1:
        raise ValueError("resource report must contain exactly one peak-RSS measurement")
    if not values[0].isdigit():
        raise ValueError("peak RSS must be a positive integer")
    peak_kib = int(values[0])
    if peak_kib <= 0:
        raise ValueError("peak RSS must be greater than zero")
    if peak_kib > MAX_EXACT_KIB:
        raise ValueError("peak RSS is too large to represent exactly in bytes")
    return peak_kib * 1024


def build_slow(
    input_path: Path,
    resource_path: Path,
    environment_path: Path,
    output_path: Path,
) -> None:
    environment = parse_environment(environment_path)
    if environment["suite"] != "slow":
        raise ValueError("environment suite must be slow")

    report = json.loads(input_path.read_text(encoding="utf-8"))
    if not isinstance(report, dict):
        raise ValueError("slow report must be an object")
    revision = report.get("lighthouse_revision")
    if not isinstance(revision, str) or not revision:
        raise ValueError("lighthouse_revision must be a non-empty string")
    if environment.get("lighthouse_revision") != revision:
        raise ValueError("environment and report Lighthouse revisions differ")
    samples = positive_integer(report.get("samples"), "samples")
    if environment.get("samples") != str(samples):
        raise ValueError("environment and report sample counts differ")
    proof_warmup = report.get("proof_warmup")
    if not isinstance(proof_warmup, bool):
        raise ValueError("proof_warmup must be a boolean")
    expected_warmup = "enabled" if proof_warmup else "disabled"
    if environment.get("proof_warmup") != expected_warmup:
        raise ValueError("environment and report proof-warmup modes differ")

    comparisons = report.get("comparisons")
    supplemental = report.get("supplemental")
    if not isinstance(comparisons, list) or not comparisons:
        raise ValueError("comparisons must be a non-empty list")
    if not isinstance(supplemental, list):
        raise ValueError("supplemental must be a list")
    for index, row in enumerate(comparisons):
        validate_comparison(row, samples, index)
    for index, row in enumerate(supplemental):
        validate_supplemental(row, samples, index)

    peak_rss_bytes = parse_peak_rss(resource_path)
    document = {
        "schema_version": 1,
        "suite": "slow",
        "measured_at": environment["measured_at"],
        "environment": {
            key: value for key, value in environment.items() if key != "suite"
        },
        "lighthouse_revision": revision,
        "samples": samples,
        "proof_warmup": proof_warmup,
        "peak_rss_bytes": peak_rss_bytes,
        "comparisons": comparisons,
        "supplemental": supplemental,
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    print(
        f"Suite peak RSS: {peak_rss_bytes:,} bytes "
        f"({peak_rss_bytes / 1024**3:.2f} GiB)"
    )


def make_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    fast = subparsers.add_parser("fast")
    fast.add_argument("--input", type=Path, required=True)
    fast.add_argument("--environment", type=Path, required=True)
    fast.add_argument("--output", type=Path, required=True)
    slow = subparsers.add_parser("slow")
    slow.add_argument("--input", type=Path, required=True)
    slow.add_argument("--resource", type=Path, required=True)
    slow.add_argument("--environment", type=Path, required=True)
    slow.add_argument("--output", type=Path, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = make_parser().parse_args(argv)
    if args.command == "fast":
        build_fast(args.input, args.environment, args.output)
    elif args.command == "slow":
        build_slow(args.input, args.resource, args.environment, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
