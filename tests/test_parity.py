import hashlib

import py_lean_multisig as lm

SEED = b"\x00" * 32
SLOT_RANGE = (0, 7)
MESSAGE = b"\x42" * 32
SIGN_SLOT = 5

EXPECTED_PUBKEY_HEX = (
    "42282a28324c5d71c1f4897aef86d5241341dc7367ce3a7285494c0a2637e729"
)
EXPECTED_PUBKEY_SHA256 = (
    "b012d428705b56f5c0a588c6b3989ce45b29f6eafb34cfbced55165625fbf278"
)
EXPECTED_SIGNATURE_SHA256 = (
    "b25e45bfd645656bbab296fcd23bba8534240d90ed9522c73c40514300b4831d"
)


def test_pubkey_bytes_are_stable():
    _, pk = lm.keygen(SEED, *SLOT_RANGE)
    assert pk.to_bytes().hex() == EXPECTED_PUBKEY_HEX
    assert hashlib.sha256(pk.to_bytes()).hexdigest() == EXPECTED_PUBKEY_SHA256


def test_signature_bytes_are_stable():
    sk, _ = lm.keygen(SEED, *SLOT_RANGE)
    sig = lm.sign(sk, MESSAGE, SIGN_SLOT)
    assert hashlib.sha256(sig.to_bytes()).hexdigest() == EXPECTED_SIGNATURE_SHA256


def test_full_cycle_with_stable_fixtures():
    """A signature produced from the fixed inputs must verify with the
    fixed pubkey. Catches any layered drift across keygen/sign/verify."""
    sk, pk = lm.keygen(SEED, *SLOT_RANGE)
    sig = lm.sign(sk, MESSAGE, SIGN_SLOT)
    assert lm.verify(pk, MESSAGE, sig, SIGN_SLOT) is None
