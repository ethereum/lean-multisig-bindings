"""Type stubs for py_lean_multisig.
"""

from typing import final

__all__ = [
    "__version__",
    "MIN_LOG_INV_RATE",
    "MAX_LOG_INV_RATE",
    "PublicKey",
    "SecretKey",
    "Signature",
    "SingleMessageSignature",
    "MultiMessageSignature",
    "ComponentInfo",
    "Prover",
    "Verifier",
    "keygen",
    "sign",
    "verify",
    "LeanMultisigError",
    "KeygenError",
    "SignError",
    "VerifyError",
    "AggregationError",
    "SerializationError",
]

__version__: str

# Bounds on Prover(log_inv_rate=...). Re-exported from upstream's
# lean_vm::{MIN,MAX}_WHIR_LOG_INV_RATE — values outside this range will
# raise ValueError.
MIN_LOG_INV_RATE: int
MAX_LOG_INV_RATE: int

@final
class PublicKey:
    @classmethod
    def from_bytes(cls, data: bytes) -> "PublicKey": ...
    def to_bytes(self) -> bytes: ...
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...

@final
class SecretKey:
    @property
    def public_key(self) -> PublicKey: ...
    @property
    def slot_start(self) -> int: ...
    @property
    def slot_end(self) -> int: ...
    def __repr__(self) -> str: ...

@final
class Signature:
    @classmethod
    def from_bytes(cls, data: bytes) -> "Signature": ...
    def to_bytes(self) -> bytes: ...
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...

@final
class SingleMessageSignature:
    """Many XMSS sigs over one (message, slot) aggregated into a single proof.
    Wraps upstream's SingleMessageAggregateSignature."""
    @classmethod
    def from_bytes(cls, data: bytes) -> "SingleMessageSignature": ...
    def to_bytes(self) -> bytes: ...
    @classmethod
    def from_bytes_without_pubkeys(
        cls, data: bytes, pubkeys: list[PublicKey]
    ) -> "SingleMessageSignature":
        """Decode the pubkey-free wire form; the caller supplies the signer
        set (e.g. from a validator registry). A set different from the one
        aggregated fails verification."""
        ...
    def to_bytes_without_pubkeys(self) -> bytes: ...
    @property
    def message(self) -> bytes: ...
    @property
    def slot(self) -> int: ...
    @property
    def pubkeys(self) -> list[PublicKey]: ...
    def __repr__(self) -> str: ...

@final
class MultiMessageSignature:
    """Bundles n SingleMessageSignatures, each potentially over a different
    (message, slot). Wraps upstream's MultiMessageAggregateSignature."""
    @classmethod
    def from_bytes(cls, data: bytes) -> "MultiMessageSignature": ...
    def to_bytes(self) -> bytes: ...
    @classmethod
    def from_bytes_without_pubkeys(
        cls, data: bytes, pubkeys_per_component: list[list[PublicKey]]
    ) -> "MultiMessageSignature":
        """Decode the pubkey-free wire form; the caller supplies one signer
        set per component, in component order."""
        ...
    def to_bytes_without_pubkeys(self) -> bytes: ...
    @property
    def components(self) -> list["ComponentInfo"]: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

@final
class ComponentInfo:
    """Read-only view of one MultiMessageSignature component's bound info."""
    @property
    def message(self) -> bytes: ...
    @property
    def slot(self) -> int: ...
    @property
    def pubkeys(self) -> list[PublicKey]: ...
    def __repr__(self) -> str: ...

@final
class Prover:
    # PyO3 #[new] surfaces as Python __new__, not __init__.
    def __new__(cls, *, log_inv_rate: int = ...) -> "Prover": ...
    def aggregate(
        self,
        pub_keys: list[PublicKey],
        signatures: list[Signature],
        message: bytes,
        slot: int,
        *,
        children: list[SingleMessageSignature] | None = ...,
    ) -> tuple[list[PublicKey], SingleMessageSignature]: ...
    def merge(
        self,
        signatures: list[SingleMessageSignature],
    ) -> MultiMessageSignature: ...
    def split(
        self,
        agg: MultiMessageSignature,
        index: int,
    ) -> SingleMessageSignature: ...

@final
class Verifier:
    def __new__(cls) -> "Verifier": ...
    def verify(
        self,
        pub_keys: list[PublicKey],
        message: bytes,
        agg: SingleMessageSignature,
        slot: int,
    ) -> None: ...
    def verify_multi(
        self,
        components: list[tuple[list[PublicKey], bytes, int]],
        agg: MultiMessageSignature,
    ) -> None: ...

def keygen(seed: bytes, slot_start: int, slot_end: int) -> tuple[SecretKey, PublicKey]: ...
def sign(sk: SecretKey, message: bytes, slot: int) -> Signature:
    """Signing is deterministic: the encoding randomness is derived from
    the secret key's seed, so the same (key, message, slot) always yields
    the same signature."""
    ...
def verify(pk: PublicKey, message: bytes, sig: Signature, slot: int) -> None: ...

class LeanMultisigError(Exception): ...
class KeygenError(LeanMultisigError): ...
class SignError(LeanMultisigError): ...
class VerifyError(LeanMultisigError): ...
class AggregationError(LeanMultisigError): ...
class SerializationError(LeanMultisigError): ...
