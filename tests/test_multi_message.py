"""Tests for the MultiMessage API: Prover.merge / split and
Verifier.verify_multi."""
import pytest
from conftest import make_signers

import py_lean_multisig as lm

# Three distinct (message, slot) pairs — each gets its own SingleMessage.
SLOT_A, SLOT_B, SLOT_C = 50, 60, 70
MSG_A = bytes(range(32))
MSG_B = bytes(range(100, 132))
MSG_C = bytes(range(200, 232))


def _components(multi):
    """The (pubkeys, message, slot) triples verify_multi expects."""
    return [(c.pubkeys, c.message, c.slot) for c in multi.components]


@pytest.fixture(scope="module")
def three_singles(prover):
    """Three SingleMessage proofs over disjoint (message, slot) pairs.
    Shared so the merge/split tests don't pay aggregate cost three times."""
    a_pks, a_sigs = make_signers(2, MSG_A, SLOT_A, seed_offset=0)
    b_pks, b_sigs = make_signers(2, MSG_B, SLOT_B, seed_offset=10)
    c_pks, c_sigs = make_signers(2, MSG_C, SLOT_C, seed_offset=20)
    _, a = prover.aggregate(a_pks, a_sigs, MSG_A, SLOT_A)
    _, b = prover.aggregate(b_pks, b_sigs, MSG_B, SLOT_B)
    _, c = prover.aggregate(c_pks, c_sigs, MSG_C, SLOT_C)
    return a, b, c


@pytest.fixture(scope="module")
def merged(prover, three_singles):
    a, b, c = three_singles
    return prover.merge([a, b, c])


def test_merge_returns_multi_message(merged):
    assert isinstance(merged, lm.MultiMessageSignature)
    assert len(merged) == 3


def test_merge_components_carry_per_component_info(merged):
    components = merged.components
    assert len(components) == 3
    assert isinstance(components[0], lm.ComponentInfo)
    assert components[0].message == MSG_A
    assert components[0].slot == SLOT_A
    assert components[1].message == MSG_B
    assert components[1].slot == SLOT_B
    assert components[2].message == MSG_C
    assert components[2].slot == SLOT_C


def test_merge_empty_raises_value_error(prover):
    with pytest.raises(ValueError):
        prover.merge([])


def test_verify_multi_succeeds(verifier, merged):
    components = _components(merged)
    assert verifier.verify_multi(components, merged) is None


def test_verify_multi_wrong_length_raises(verifier, merged):
    components = _components(merged)
    with pytest.raises(lm.VerifyError):
        verifier.verify_multi(components[:2], merged)


def test_verify_multi_wrong_message_raises(verifier, merged):
    components = _components(merged)
    components[1] = (components[1][0], MSG_C, components[1][2])  # B's slot, C's msg
    with pytest.raises(lm.VerifyError):
        verifier.verify_multi(components, merged)


def test_verify_multi_wrong_slot_raises(verifier, merged):
    components = _components(merged)
    components[0] = (components[0][0], components[0][1], components[0][2] + 1)
    with pytest.raises(lm.VerifyError):
        verifier.verify_multi(components, merged)


def test_verify_multi_wrong_pubkeys_raises(verifier, merged):
    components = _components(merged)
    # Swap pubkeys between components 0 and 1.
    swapped = (components[1][0], components[0][1], components[0][2])
    components[0] = swapped
    with pytest.raises(lm.VerifyError):
        verifier.verify_multi(components, merged)


def test_split_recovers_a_single_message(prover, verifier, merged):
    recovered = prover.split(merged, 1)
    assert isinstance(recovered, lm.SingleMessageSignature)
    assert recovered.message == MSG_B
    assert recovered.slot == SLOT_B
    # Verifies as a standalone single-message signature
    verifier.verify(recovered.pubkeys, MSG_B, recovered, SLOT_B)


def test_split_out_of_bounds_raises(prover, merged):
    with pytest.raises(ValueError):
        prover.split(merged, 99)


def test_promote_single_to_multi_with_one_component(prover, verifier, three_singles):
    """Single-component MultiMessage is legal (n_components == 1)."""
    a, _, _ = three_singles
    promoted = prover.merge([a])
    assert len(promoted) == 1
    assert promoted.components[0].message == MSG_A
    assert promoted.components[0].slot == SLOT_A
    components = [(promoted.components[0].pubkeys, MSG_A, SLOT_A)]
    verifier.verify_multi(components, promoted)


def test_multi_message_round_trip(merged):
    raw = merged.to_bytes()
    decoded = lm.MultiMessageSignature.from_bytes(raw)
    assert decoded.to_bytes() == raw
    assert len(decoded) == len(merged)


def test_multi_message_from_bytes_garbage_raises():
    with pytest.raises(lm.SerializationError):
        lm.MultiMessageSignature.from_bytes(b"not a valid postcard payload")


def test_multi_message_pubkey_free_round_trip(merged):
    """The pubkey-free wire form drops every component's signer set; the
    receiver supplies them at decode time, in component order."""
    raw = merged.to_bytes()
    compact = merged.to_bytes_without_pubkeys()
    assert len(compact) < len(raw)
    pubkeys_per_component = [c.pubkeys for c in merged.components]
    decoded = lm.MultiMessageSignature.from_bytes_without_pubkeys(
        compact, pubkeys_per_component
    )
    assert decoded.to_bytes() == raw
