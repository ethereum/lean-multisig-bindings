use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lean_multisig_comparison::{BlsFixtureSet, FixtureSet, MAX_DISTINCT_CLAIMS};

const SAME_CLAIM_SIZES: [usize; 8] = [1, 8, 16, 32, 64, 128, 256, 512];
const DISTINCT_CLAIM_SIZES: [usize; 3] = [1, 8, 16];

fn aggregate_bls(signatures: &[lighthouse_bls::Signature]) -> lighthouse_bls::AggregateSignature {
    let mut aggregate = lighthouse_bls::AggregateSignature::infinity();
    for signature in signatures {
        aggregate.add_assign(signature);
    }
    aggregate
}

fn single_operations(criterion: &mut Criterion) {
    let fixtures = FixtureSet::same_claim(1).expect("single-signer fixture should be valid");
    let lean_claim = &fixtures.lean_claims()[0];
    let lean_key = &fixtures.lean_keys()[0];
    let lean_signature = &fixtures.lean_signatures()[0];
    let lean_public_key = &fixtures.lean_public_keys()[0];
    let bls_message = fixtures.bls_messages()[0];
    let bls_key = &fixtures.bls_keys()[0];
    let bls_signature = &fixtures.bls_signatures()[0];
    let bls_public_key = &fixtures.bls_public_keys()[0];

    assert!(lean_multisig::verify(
        lean_signature,
        std::slice::from_ref(lean_public_key),
        lean_claim,
    )
    .is_ok());
    assert!(bls_signature.verify(bls_public_key, bls_message));

    let artifact_sizes = [
        ("secret_key_16_slots/lean", lean_key.to_bytes().len()),
        (
            "secret_key_16_slots/lighthouse",
            bls_key.serialize().as_ref().len(),
        ),
        ("public_key/lean", lean_public_key.len()),
        ("public_key/lighthouse", bls_public_key.serialize().len()),
        ("raw_signature/lean", lean_signature.to_bytes().len()),
        ("raw_signature/lighthouse", bls_signature.serialize().len()),
    ];
    for (artifact, size) in artifact_sizes {
        println!("artifact-size {artifact} {size}");
    }

    // RangeInclusive counts both endpoints, so 16 active slots end at slot 15.
    let key_creation_last_slot = MAX_DISTINCT_CLAIMS
        .checked_sub(1)
        .and_then(|slot| u32::try_from(slot).ok())
        .expect("the fixture slot count should fit in a nonempty u32 range");
    let mut group = criterion.benchmark_group("key_creation");
    group.bench_function("lean", |bencher| {
        bencher.iter(|| {
            black_box(
                lean_multisig::SecretKey::generate(black_box(0..=key_creation_last_slot))
                    .expect("random XMSS key generation should succeed"),
            )
        });
    });
    group.bench_function("lighthouse", |bencher| {
        bencher.iter(|| black_box(lighthouse_bls::SecretKey::random()));
    });
    group.finish();

    let mut group = criterion.benchmark_group("public_key");
    group.bench_function("lean", |bencher| {
        bencher.iter(|| black_box(black_box(lean_key).public_key()));
    });
    group.bench_function("lighthouse", |bencher| {
        bencher.iter(|| black_box(black_box(bls_key).public_key()));
    });
    group.finish();

    let mut group = criterion.benchmark_group("sign");
    group.bench_function("lean", |bencher| {
        bencher.iter(|| {
            black_box(
                black_box(lean_key)
                    .sign(black_box(lean_claim))
                    .expect("fixture XMSS signing should succeed"),
            )
        });
    });
    group.bench_function("lighthouse", |bencher| {
        bencher.iter(|| black_box(black_box(bls_key).sign(black_box(bls_message))));
    });
    group.finish();

    let lean_signature_bytes = lean_signature.to_bytes();
    let bls_signature_bytes = bls_signature.serialize();
    let decoded_lean_signature = lean_multisig::Signature::from_bytes(
        &lean_signature_bytes,
        lean_claim,
        std::slice::from_ref(lean_public_key),
    )
    .expect("fixture XMSS signature deserialization should succeed");
    let decoded_bls_signature = lighthouse_bls::Signature::deserialize(&bls_signature_bytes)
        .expect("fixture BLS signature deserialization should succeed");
    assert!(lean_multisig::verify(
        &decoded_lean_signature,
        std::slice::from_ref(lean_public_key),
        lean_claim,
    )
    .is_ok());
    assert!(decoded_bls_signature.verify(bls_public_key, bls_message));

    let mut group = criterion.benchmark_group("raw_signature_serialize");
    group.bench_function("lean", |bencher| {
        bencher.iter(|| black_box(black_box(lean_signature).to_bytes()));
    });
    group.bench_function("lighthouse", |bencher| {
        bencher.iter(|| black_box(black_box(bls_signature).serialize()));
    });
    group.finish();

    let mut group = criterion.benchmark_group("raw_signature_deserialize");
    group.bench_function("lean", |bencher| {
        bencher.iter(|| {
            black_box(
                lean_multisig::Signature::from_bytes(
                    black_box(&lean_signature_bytes),
                    black_box(lean_claim),
                    black_box(std::slice::from_ref(lean_public_key)),
                )
                .expect("fixture XMSS signature deserialization should succeed"),
            )
        });
    });
    group.bench_function("lighthouse", |bencher| {
        bencher.iter(|| {
            black_box(
                lighthouse_bls::Signature::deserialize(black_box(&bls_signature_bytes))
                    .expect("fixture BLS signature deserialization should succeed"),
            )
        });
    });
    group.finish();

    let mut group = criterion.benchmark_group("single_verify");
    group.bench_function("lean", |bencher| {
        bencher.iter(|| {
            black_box(lean_multisig::verify(
                black_box(lean_signature),
                black_box(std::slice::from_ref(lean_public_key)),
                black_box(lean_claim),
            ))
        });
    });
    group.bench_function("lighthouse", |bencher| {
        bencher.iter(|| {
            black_box(
                black_box(bls_signature).verify(black_box(bls_public_key), black_box(bls_message)),
            )
        });
    });
    group.finish();
}

fn lighthouse_same_claim_aggregate(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lighthouse_same_claim_aggregate");
    for size in SAME_CLAIM_SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &size| {
                let fixtures = BlsFixtureSet::same_claim(size)
                    .expect("same-claim BLS fixture should be valid");
                let signatures = fixtures.signatures();
                let aggregate = aggregate_bls(signatures);
                assert!(fixtures.verify_same_claim_aggregate(&aggregate));

                bencher.iter(|| {
                    let mut aggregate = lighthouse_bls::AggregateSignature::infinity();
                    for signature in black_box(signatures) {
                        aggregate.add_assign(black_box(signature));
                    }
                    black_box(aggregate)
                });
            },
        );
    }
    group.finish();
}

fn lighthouse_same_claim_verify(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lighthouse_same_claim_verify");
    for size in SAME_CLAIM_SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &size| {
                let fixtures = BlsFixtureSet::same_claim(size)
                    .expect("same-claim BLS fixture should be valid");
                let aggregate = fixtures.aggregate();
                let message = fixtures.messages()[0];
                let public_keys = fixtures.public_keys().iter().collect::<Vec<_>>();
                assert!(aggregate.fast_aggregate_verify(message, &public_keys));

                bencher.iter(|| {
                    black_box(
                        black_box(&aggregate)
                            .fast_aggregate_verify(black_box(message), black_box(&public_keys)),
                    )
                });
            },
        );
    }
    group.finish();
}

fn lighthouse_distinct_claim_aggregate(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lighthouse_distinct_claim_aggregate");
    for size in DISTINCT_CLAIM_SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &size| {
                let fixtures = BlsFixtureSet::distinct_claims(size)
                    .expect("distinct-claim BLS fixture should be valid");
                let signatures = fixtures.signatures();
                let aggregate = aggregate_bls(signatures);
                assert!(fixtures.verify_distinct_claim_aggregate(&aggregate));

                bencher.iter(|| {
                    let mut aggregate = lighthouse_bls::AggregateSignature::infinity();
                    for signature in black_box(signatures) {
                        aggregate.add_assign(black_box(signature));
                    }
                    black_box(aggregate)
                });
            },
        );
    }
    group.finish();
}

fn lighthouse_distinct_claim_verify(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lighthouse_distinct_claim_verify");
    for size in DISTINCT_CLAIM_SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &size| {
                let fixtures = BlsFixtureSet::distinct_claims(size)
                    .expect("distinct-claim BLS fixture should be valid");
                let aggregate = fixtures.aggregate();
                let messages = fixtures.messages();
                let public_keys = fixtures.public_keys().iter().collect::<Vec<_>>();
                assert!(aggregate.aggregate_verify(messages, &public_keys));

                bencher.iter(|| {
                    black_box(
                        black_box(&aggregate)
                            .aggregate_verify(black_box(messages), black_box(&public_keys)),
                    )
                });
            },
        );
    }
    group.finish();
}

fn lighthouse_signature_sets_verify(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lighthouse_signature_sets_verify");
    for size in DISTINCT_CLAIM_SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &size| {
                let fixtures = BlsFixtureSet::distinct_claims(size)
                    .expect("distinct-claim BLS fixture should be valid");
                let signature_sets = fixtures.signature_sets();
                assert!(lighthouse_bls::verify_signature_sets(signature_sets.iter()));

                bencher.iter(|| {
                    black_box(lighthouse_bls::verify_signature_sets(black_box(
                        signature_sets.iter(),
                    )))
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    single_operations,
    lighthouse_same_claim_aggregate,
    lighthouse_same_claim_verify,
    lighthouse_distinct_claim_aggregate,
    lighthouse_distinct_claim_verify,
    lighthouse_signature_sets_verify,
);
criterion_main!(benches);
