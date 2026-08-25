use lean_multisig_comparison::{deterministic_key_material, FixtureSet, MAX_DISTINCT_CLAIMS};

#[test]
fn fixtures_have_the_requested_unique_signers() {
    let fixtures = FixtureSet::same_claim(3).unwrap();

    assert_eq!(fixtures.len(), 3);
    assert_eq!(fixtures.lean_public_keys().len(), 3);
    assert_eq!(fixtures.bls_public_keys().len(), 3);
    assert_ne!(
        fixtures.lean_public_keys()[0],
        fixtures.lean_public_keys()[1]
    );
    assert_ne!(
        fixtures.bls_public_keys()[0].serialize(),
        fixtures.bls_public_keys()[1].serialize()
    );
}

#[test]
fn distinct_fixtures_use_distinct_messages_and_slots() {
    let fixtures = FixtureSet::distinct_claims(3).unwrap();

    assert_eq!(fixtures.lean_claims()[0].slot(), 0);
    assert_eq!(fixtures.lean_claims()[1].slot(), 1);
    assert_ne!(
        fixtures.lean_claims()[0].message(),
        fixtures.lean_claims()[1].message()
    );
    assert_ne!(fixtures.bls_messages()[0], fixtures.bls_messages()[1]);
}

#[test]
fn fixtures_reject_empty_and_excessive_counts() {
    assert!(FixtureSet::same_claim(0).is_err());
    assert!(FixtureSet::distinct_claims(0).is_err());
    assert!(FixtureSet::distinct_claims(MAX_DISTINCT_CLAIMS + 1).is_err());
}

#[test]
fn every_raw_fixture_is_valid_before_benchmarking() {
    FixtureSet::same_claim(3).unwrap().validate_raw().unwrap();
    FixtureSet::distinct_claims(3)
        .unwrap()
        .validate_raw()
        .unwrap();
}

#[test]
fn deterministic_key_material_is_stable_unique_and_nonzero() {
    let first = deterministic_key_material(0).unwrap();
    let second = deterministic_key_material(1).unwrap();

    assert_eq!(first, deterministic_key_material(0).unwrap());
    assert_ne!(first, second);
    assert_ne!(first, [0; 32]);
    lean_multisig::SecretKey::from_seed(first, 0..=MAX_DISTINCT_CLAIMS as u32).unwrap();
    lighthouse_bls::SecretKey::deserialize(&first).unwrap();
}

#[test]
fn same_claim_bls_aggregate_verifies_for_exact_fixture_context() {
    let fixtures = FixtureSet::same_claim(3).unwrap();
    let aggregate = fixtures.bls_aggregate();

    assert!(fixtures.verify_bls_same_claim_aggregate(&aggregate));

    let wrong = FixtureSet::same_claim(2).unwrap();
    assert!(!wrong.verify_bls_same_claim_aggregate(&aggregate));
}

#[test]
fn distinct_claim_bls_aggregate_verifies_for_exact_fixture_context() {
    let fixtures = FixtureSet::distinct_claims(3).unwrap();
    let aggregate = fixtures.bls_aggregate();

    assert!(fixtures.verify_bls_distinct_claim_aggregate(&aggregate));

    let wrong = FixtureSet::distinct_claims(2).unwrap();
    assert!(!wrong.verify_bls_distinct_claim_aggregate(&aggregate));
}

#[test]
fn borrowed_signature_sets_verify_as_a_batch() {
    let fixtures = FixtureSet::distinct_claims(3).unwrap();
    let signature_sets = fixtures.bls_signature_sets();

    assert_eq!(signature_sets.len(), fixtures.len());
    assert!(lighthouse_bls::verify_signature_sets(signature_sets.iter()));
}
