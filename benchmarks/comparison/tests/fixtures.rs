use lean_multisig_comparison::{
    deterministic_key_material, BlsFixtureSet, FixtureSet, MAX_DISTINCT_CLAIMS,
    MAX_SAME_CLAIM_SIGNERS, MIXED_CLAIM_COUNTS, MIXED_CLAIM_SIGNATURES,
};

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
fn bls_only_same_claim_fixtures_scale_to_512_signers() {
    let fixtures = BlsFixtureSet::same_claim(MAX_SAME_CLAIM_SIGNERS).unwrap();

    assert_eq!(fixtures.len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.messages().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.keys().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.signatures().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.public_keys().len(), MAX_SAME_CLAIM_SIGNERS);
    assert!(fixtures.verify_same_claim_aggregate(&fixtures.aggregate()));
}

#[test]
fn bls_only_distinct_claim_fixtures_use_and_verify_distinct_messages() {
    let fixtures = BlsFixtureSet::distinct_claims(3).unwrap();
    let aggregate = fixtures.aggregate();

    assert_ne!(fixtures.messages()[0], fixtures.messages()[1]);
    assert!(fixtures.verify_distinct_claim_aggregate(&aggregate));
}

#[test]
fn bls_only_fixtures_expose_borrowed_signature_sets() {
    let fixtures = BlsFixtureSet::distinct_claims(3).unwrap();
    let signature_sets = fixtures.signature_sets();

    assert_eq!(signature_sets.len(), fixtures.len());
    assert!(lighthouse_bls::verify_signature_sets(signature_sets.iter()));
}

#[test]
#[ignore = "validates 512 XMSS fixtures and is intentionally slow"]
fn same_claim_fixtures_support_the_expanded_signer_limit() {
    let fixtures = FixtureSet::same_claim(MAX_SAME_CLAIM_SIGNERS).unwrap();

    assert_eq!(fixtures.lean_claims().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.lean_keys().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.lean_signatures().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.lean_public_keys().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.bls_messages().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.bls_keys().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.bls_signatures().len(), MAX_SAME_CLAIM_SIGNERS);
    assert_eq!(fixtures.bls_public_keys().len(), MAX_SAME_CLAIM_SIGNERS);
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
fn mixed_claim_fixtures_distribute_signers_evenly() {
    let fixtures = FixtureSet::mixed_claims(8, 2).unwrap();
    let groups = fixtures.lean_claim_groups();

    assert_eq!(fixtures.claim_count(), 2);
    assert_eq!(groups.len(), 2);
    assert!(groups.iter().all(|group| group.signers.len() == 4));
    assert_eq!(fixtures.lean_claims()[0], fixtures.lean_claims()[2]);
    assert_ne!(fixtures.lean_claims()[0], fixtures.lean_claims()[1]);
    assert_eq!(fixtures.bls_messages()[0], fixtures.bls_messages()[2]);
    assert_ne!(fixtures.bls_messages()[0], fixtures.bls_messages()[1]);

    let bls_aggregate = fixtures.bls_aggregate();
    assert!(fixtures
        .verify_bls_grouped_claim_aggregate(&bls_aggregate)
        .unwrap());
}

#[test]
fn mixed_claim_fixtures_validate_the_requested_shape() {
    assert!(FixtureSet::mixed_claims(8, 0).is_err());
    assert!(FixtureSet::mixed_claims(8, 3).is_err());
    assert!(FixtureSet::mixed_claims(8, 9).is_err());
    assert!(FixtureSet::mixed_claims(8, MAX_DISTINCT_CLAIMS + 1).is_err());
    assert!(FixtureSet::mixed_claims(MAX_SAME_CLAIM_SIGNERS + 1, 1).is_err());
    assert_eq!(MIXED_CLAIM_COUNTS, [8, 16]);
    assert_eq!(MIXED_CLAIM_SIGNATURES / MIXED_CLAIM_COUNTS[0], 64);
    assert_eq!(MIXED_CLAIM_SIGNATURES / MIXED_CLAIM_COUNTS[1], 32);
}

#[test]
#[ignore = "validates and groups two 512-signature XMSS fixtures and is intentionally slow"]
fn configured_mixed_claim_fixtures_have_even_signer_groups() {
    for claim_count in MIXED_CLAIM_COUNTS {
        let fixtures = FixtureSet::mixed_claims(MIXED_CLAIM_SIGNATURES, claim_count).unwrap();
        let groups = fixtures.lean_claim_groups();

        assert_eq!(groups.len(), claim_count);
        assert!(groups
            .iter()
            .all(|group| group.signers.len() == MIXED_CLAIM_SIGNATURES / claim_count));
    }
}

#[test]
fn fixture_xmss_keys_have_exactly_the_supported_claim_slots() {
    let fixtures = FixtureSet::same_claim(1).unwrap();
    let last_slot = u32::try_from(MAX_DISTINCT_CLAIMS - 1).unwrap();

    assert_eq!(fixtures.lean_keys()[0].slots(), 0..=last_slot);
}

#[test]
fn fixtures_reject_empty_and_excessive_counts() {
    assert!(BlsFixtureSet::same_claim(0).is_err());
    assert!(BlsFixtureSet::distinct_claims(0).is_err());
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
