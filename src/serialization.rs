//! Byte encodings are delegated to upstream so the wire formats stay
//! interchangeable with other leanMultisig consumers:
//!
//! - `PublicKey` / `Signature`: upstream's SSZ encoding (fixed-length,
//!   little-endian canonical field elements).
//! - `SingleMessageSignature` / `MultiMessageSignature`: upstream's postcard
//!   encoding, either self-contained (`to_bytes`) or with the pubkeys
//!   stripped (`to_bytes_without_pubkeys`) for receivers that already know
//!   the signer set.

use pyo3::prelude::*;
use rec_aggregation::init_aggregation_bytecode;
use ssz::Decode;
use xmss::{XmssPublicKey, XmssSignature};

use crate::error::SerializationError;

pub fn decode_public_key(bytes: &[u8]) -> PyResult<XmssPublicKey> {
    XmssPublicKey::from_ssz_bytes(bytes)
        .map_err(|e| SerializationError::new_err(format!("failed to decode PublicKey: {:?}", e)))
}

pub fn decode_signature(bytes: &[u8]) -> PyResult<XmssSignature> {
    XmssSignature::from_ssz_bytes(bytes)
        .map_err(|e| SerializationError::new_err(format!("failed to decode Signature: {:?}", e)))
}

/// Decode an aggregated signature via the given upstream constructor.
/// Decoding recomputes the signature's bytecode claim, which requires the
/// aggregation bytecode — initialized here so every decode path shares the
/// precondition (idempotent, but the first call compiles the zkVM program).
pub fn decode_aggregate<T>(what: &str, decode: impl FnOnce() -> Option<T> + Send) -> PyResult<T>
where
    T: Send,
{
    init_aggregation_bytecode();
    decode().ok_or_else(|| SerializationError::new_err(format!("failed to decode {what}")))
}
