import pytest
from conftest import make_signers

import py_lean_multisig as lm

# Messages are arbitrary 32-byte strings; upstream hashes the raw bytes.
MSG = bytes(range(32))
SLOT = 111


def _signers(n: int, seed_offset: int = 0):
    return make_signers(n, MSG, SLOT, seed_offset)


@pytest.fixture(scope="module")
def child_proofs(prover):
    """Two pre-aggregated child proofs over disjoint 2-signer batches,
    shared across the hierarchical-aggregation tests so we don't pay
    ~1.5s × 2 of redundant proving per test. Each proof carries its own
    signer set via `.pubkeys`."""
    pks_a, sigs_a = _signers(2)
    pks_b, sigs_b = _signers(2, seed_offset=49)
    _, agg_a = prover.aggregate(pks_a, sigs_a, MSG, SLOT)
    _, agg_b = prover.aggregate(pks_b, sigs_b, MSG, SLOT)
    return agg_a, agg_b


@pytest.fixture(scope="module")
def pubkey_free_agg(prover):
    """One aggregate shared by the serialization and wrong-input tests
    (aggregation is deterministic, so re-proving would be pure waste)."""
    pks, sigs = _signers(2, seed_offset=99)
    return prover.aggregate(pks, sigs, MSG, SLOT)


def test_aggregate_then_verify_4_sigs(prover, verifier):
    pks, sigs = _signers(4)
    sorted_pks, agg = prover.aggregate(pks, sigs, MSG, SLOT)
    assert isinstance(agg, lm.SingleMessageSignature)
    assert isinstance(sorted_pks, list)
    assert all(isinstance(p, lm.PublicKey) for p in sorted_pks)
    assert len(sorted_pks) == 4
    # Returns None on success
    assert verifier.verify(sorted_pks, MSG, agg, SLOT) is None
    # Bound info is exposed on the typed signature
    assert agg.message == MSG
    assert agg.slot == SLOT
    assert [p.to_bytes() for p in agg.pubkeys] == [p.to_bytes() for p in sorted_pks]


def test_aggregate_returns_sorted_pks(prover):
    pks, sigs = _signers(4)
    sorted_pks, _ = prover.aggregate(pks, sigs, MSG, SLOT)
    # The order must match what verifier expects — i.e. sorted by upstream's
    # XmssPublicKey Ord, which we don't replicate in Python. Just check we
    # got back the same set.
    assert set(p.to_bytes() for p in sorted_pks) == set(p.to_bytes() for p in pks)


def test_aggregate_mismatched_lengths_raises_value_error(prover):
    pks, sigs = _signers(3)
    with pytest.raises(ValueError):
        prover.aggregate(pks, sigs[:2], MSG, SLOT)
    with pytest.raises(ValueError):
        prover.aggregate(pks[:2], sigs, MSG, SLOT)


def test_prover_log_inv_rate_validation():
    with pytest.raises(ValueError):
        lm.Prover(log_inv_rate=lm.MIN_LOG_INV_RATE - 1)
    with pytest.raises(ValueError):
        lm.Prover(log_inv_rate=lm.MAX_LOG_INV_RATE + 1)


def test_aggregate_short_message_raises_serialization_error(prover):
    pks, sigs = _signers(4)
    with pytest.raises(lm.SerializationError):
        prover.aggregate(pks, sigs, b"\x00" * 31, SLOT)


def test_single_message_signature_round_trip(pubkey_free_agg):
    _, agg = pubkey_free_agg
    raw = agg.to_bytes()
    assert isinstance(raw, bytes)
    assert len(raw) > 0
    agg2 = lm.SingleMessageSignature.from_bytes(raw)
    assert agg2.to_bytes() == raw


def test_single_message_signature_from_bytes_garbage_raises():
    with pytest.raises(lm.SerializationError):
        lm.SingleMessageSignature.from_bytes(b"not a valid postcard payload")


def test_single_message_signature_pubkey_free_round_trip(pubkey_free_agg):
    """The pubkey-free wire form drops the signer set; the receiver
    supplies it at decode time (e.g. from a validator registry)."""
    sorted_pks, agg = pubkey_free_agg
    raw = agg.to_bytes()
    compact = agg.to_bytes_without_pubkeys()
    assert len(compact) < len(raw)
    agg2 = lm.SingleMessageSignature.from_bytes_without_pubkeys(compact, sorted_pks)
    assert agg2.to_bytes() == raw


def test_single_message_signature_pubkey_free_wrong_set_fails(verifier, pubkey_free_agg):
    """Decoding with a signer set different from the aggregated one must
    not verify (binding is via the proof, not the serialized keys)."""
    _, agg = pubkey_free_agg
    other_pks, _ = _signers(2, seed_offset=199)
    compact = agg.to_bytes_without_pubkeys()
    try:
        forged = lm.SingleMessageSignature.from_bytes_without_pubkeys(compact, other_pks)
    except lm.SerializationError:
        return  # rejected at decode — also fine
    with pytest.raises(lm.VerifyError):
        verifier.verify(forged.pubkeys, MSG, forged, SLOT)


def test_verify_tampered_aggregated_signature_raises(prover, verifier):
    pks, sigs = _signers(4)
    sorted_pks, agg = prover.aggregate(pks, sigs, MSG, SLOT)
    # Round-trip through bytes, flip a bit, decode, verify
    raw = bytearray(agg.to_bytes())
    raw[len(raw) // 2] ^= 0x01
    try:
        tampered = lm.SingleMessageSignature.from_bytes(bytes(raw))
    except lm.SerializationError:
        # If the flipped bit breaks the postcard structure, decoding fails
        # before we get to the verifier. That's still a rejection of a
        # tampered signature — pass.
        return
    with pytest.raises(lm.VerifyError):
        verifier.verify(sorted_pks, MSG, tampered, SLOT)


def test_verify_wrong_slot_raises(verifier, pubkey_free_agg):
    sorted_pks, agg = pubkey_free_agg
    with pytest.raises(lm.VerifyError):
        verifier.verify(sorted_pks, MSG, agg, SLOT + 1)


def test_verify_wrong_message_raises(verifier, pubkey_free_agg):
    sorted_pks, agg = pubkey_free_agg
    other_msg = bytes(range(100, 132))
    with pytest.raises(lm.VerifyError):
        verifier.verify(sorted_pks, other_msg, agg, SLOT)


def test_verify_short_message_raises_serialization_error(verifier, child_proofs):
    """Verifier.verify rejects a wrong-length message at the boundary
    (before any zkVM work)."""
    agg_a, _ = child_proofs
    with pytest.raises(lm.SerializationError):
        verifier.verify(agg_a.pubkeys, b"\x00" * 31, agg_a, SLOT)


def test_hierarchical_aggregation(prover, verifier, child_proofs):
    """Aggregate two leaves (2 sigs each) into child proofs, then
    aggregate the children at the top level via the `children=` kwarg.
    Verifier sees the union of all leaf pubkeys."""
    agg_a, agg_b = child_proofs

    sorted_pks_top, agg_top = prover.aggregate(
        [], [], MSG, SLOT,
        children=[agg_a, agg_b],
    )

    verifier.verify(sorted_pks_top, MSG, agg_top, SLOT)


def test_hierarchical_aggregation_with_fresh_raw_sigs(prover, verifier, child_proofs):
    """Mixing raw signatures with children at the same level: fold two
    existing child aggregates plus a fresh batch of raw signatures into
    one combined proof in a single aggregate() call. Verifier sees the
    union of all signers (children's leaves + the fresh raw ones)."""
    agg_a, agg_b = child_proofs
    # A fresh batch of raw signers — disjoint seed range so pubkeys
    # don't collide with either child.
    pks_c, sigs_c = _signers(2, seed_offset=149)

    sorted_pks_top, agg_top = prover.aggregate(
        pks_c, sigs_c, MSG, SLOT,
        children=[agg_a, agg_b],
    )

    assert len(sorted_pks_top) == 6
    verifier.verify(sorted_pks_top, MSG, agg_top, SLOT)
