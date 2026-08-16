import py_lean_multisig as lm
import pytest


def test_claim_binds_a_32_byte_message_to_a_slot():
    claim = lm.Claim(b"\x42" * 32, 7)

    assert claim.message == b"\x42" * 32
    assert claim.slot == 7


def test_secret_key_signs_and_serializes_with_explicit_signature_context():
    claim = lm.Claim(b"\x42" * 32, 7)
    key = lm.SecretKey.from_seed(b"\x01" * 32, 0, 7)

    signature = key.sign(claim)
    restored_key = lm.SecretKey.from_bytes(key.to_bytes())
    restored_signature = lm.Signature.from_bytes(
        signature.to_bytes(), claim, [key.public_key]
    )

    assert restored_key.public_key == key.public_key
    assert restored_key.sign(claim).to_bytes() == signature.to_bytes()
    assert lm.verified_signers(restored_signature, claim) == [key.public_key]
    assert lm.verify(restored_signature, [key.public_key], claim) is None


def test_verification_rejects_a_different_claim_or_signer_set():
    claim = lm.Claim(b"\x42" * 32, 7)
    other_claim = lm.Claim(b"\x43" * 32, 7)
    key = lm.SecretKey.from_seed(b"\x01" * 32, 0, 7)
    other_key = lm.SecretKey.from_seed(b"\x02" * 32, 0, 7)
    signature = key.sign(claim)

    with pytest.raises(lm.LeanMultisigError):
        lm.verify(signature, [other_key.public_key], claim)
    with pytest.raises(lm.LeanMultisigError):
        lm.verify(signature, [key.public_key], other_claim)


def test_aggregation_carries_signers_and_requires_the_exact_expected_set():
    lm.setup()
    claim = lm.Claim(b"\x42" * 32, 7)
    keys = [lm.SecretKey.from_seed(bytes([seed]) * 32, 0, 7) for seed in (1, 2)]
    aggregate = lm.aggregate([key.sign(claim) for key in keys], claim)
    expected = [key.public_key for key in keys]

    assert set(lm.verified_signers(aggregate, claim)) == set(expected)
    assert lm.verify(aggregate, expected, claim) is None
    with pytest.raises(lm.LeanMultisigError):
        lm.verify(aggregate, expected[:1], claim)


def test_multi_claim_proof_round_trips_with_supplied_claim_contexts():
    lm.setup()
    first_claim = lm.Claim(b"\x11" * 32, 3)
    second_claim = lm.Claim(b"\x22" * 32, 4)
    first_key = lm.SecretKey.from_seed(b"\x01" * 32, 0, 7)
    second_key = lm.SecretKey.from_seed(b"\x02" * 32, 0, 7)
    expected = [
        lm.ClaimSigners(first_claim, [first_key.public_key]),
        lm.ClaimSigners(second_claim, [second_key.public_key]),
    ]

    proof = lm.merge_claims([first_key.sign(first_claim), second_key.sign(second_claim)])
    restored = lm.MultiClaimProof.from_bytes(proof.to_bytes(), expected)

    assert lm.verify_claims(restored, list(reversed(expected))) is None
    proved = lm.verified_claims(restored)
    assert [group.claim for group in proved] == [first_claim, second_claim]
