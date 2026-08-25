use lean_multisig_comparison::{FixtureSet, MAX_DISTINCT_CLAIMS};

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
