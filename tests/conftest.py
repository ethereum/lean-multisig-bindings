import pytest

import py_lean_multisig as lm


def make_signers(n: int, msg: bytes, slot: int, seed_offset: int = 0):
    """Generate n distinct signers + sigs for msg at slot.

    `seed_offset` shifts the keygen seed, so calls with disjoint offsets
    produce disjoint signer sets — useful for hierarchical aggregation
    tests where each child batch must have unique pubkeys.
    """
    pairs = [
        lm.keygen(bytes([(i + 1 + seed_offset) % 256]) * 32, slot, slot + 1)
        for i in range(n)
    ]
    pks = [pk for _, pk in pairs]
    sigs = [lm.sign(sk, msg, slot) for sk, _ in pairs]
    return pks, sigs


@pytest.fixture(scope="module")
def prover():
    return lm.Prover(log_inv_rate=lm.MAX_LOG_INV_RATE)  # smallest proof, fastest aggregate


@pytest.fixture(scope="module")
def verifier():
    return lm.Verifier()
