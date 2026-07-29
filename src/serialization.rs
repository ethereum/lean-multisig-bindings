//! Byte encodings are delegated to upstream so the wire formats stay
//! interchangeable with other leanMultisig consumers:
//!
//! - `PublicKey` / `Signature`: upstream's SSZ encoding (fixed-length,
//!   little-endian canonical field elements).
//! - `SingleMessageSignature` / `MultiMessageSignature`: upstream's postcard
//!   encoding, either self-contained (`to_bytes`) or with the pubkeys
//!   stripped (`to_bytes_without_pubkeys`) for receivers that already know
//!   the signer set.
//!
//! Decoding an aggregated signature recomputes its bytecode claim, which
//! requires the aggregation bytecode: every decode path here calls
//! `init_aggregation_bytecode()` first (idempotent, but the initial call
//! compiles the zkVM program).

use pyo3::prelude::*;
use rec_aggregation::{
    init_aggregation_bytecode, MultiMessageAggregateSignature, SingleMessageAggregateSignature,
};
use ssz::{Decode, Encode};
use xmss::{XmssPublicKey, XmssSignature};

use crate::error::SerializationError;

pub fn encode_public_key(pk: &XmssPublicKey) -> Vec<u8> {
    pk.as_ssz_bytes()
}

pub fn decode_public_key(bytes: &[u8]) -> PyResult<XmssPublicKey> {
    XmssPublicKey::from_ssz_bytes(bytes)
        .map_err(|e| SerializationError::new_err(format!("failed to decode PublicKey: {:?}", e)))
}

pub fn encode_signature(sig: &XmssSignature) -> Vec<u8> {
    sig.as_ssz_bytes()
}

pub fn decode_signature(bytes: &[u8]) -> PyResult<XmssSignature> {
    XmssSignature::from_ssz_bytes(bytes)
        .map_err(|e| SerializationError::new_err(format!("failed to decode Signature: {:?}", e)))
}

pub fn decode_single_message_signature(bytes: &[u8]) -> PyResult<SingleMessageAggregateSignature> {
    init_aggregation_bytecode();
    SingleMessageAggregateSignature::from_bytes(bytes)
        .ok_or_else(|| SerializationError::new_err("failed to decode SingleMessageSignature"))
}

pub fn decode_single_message_signature_without_pubkeys(
    bytes: &[u8],
    pubkeys: Vec<XmssPublicKey>,
) -> PyResult<SingleMessageAggregateSignature> {
    init_aggregation_bytecode();
    SingleMessageAggregateSignature::from_bytes_without_pubkeys(bytes, pubkeys).ok_or_else(|| {
        SerializationError::new_err("failed to decode SingleMessageSignature (pubkey-free form)")
    })
}

pub fn decode_multi_message_signature(bytes: &[u8]) -> PyResult<MultiMessageAggregateSignature> {
    init_aggregation_bytecode();
    MultiMessageAggregateSignature::from_bytes(bytes)
        .ok_or_else(|| SerializationError::new_err("failed to decode MultiMessageSignature"))
}

pub fn decode_multi_message_signature_without_pubkeys(
    bytes: &[u8],
    pubkeys_per_component: Vec<Vec<XmssPublicKey>>,
) -> PyResult<MultiMessageAggregateSignature> {
    init_aggregation_bytecode();
    MultiMessageAggregateSignature::from_bytes_without_pubkeys(bytes, pubkeys_per_component)
        .ok_or_else(|| {
            SerializationError::new_err("failed to decode MultiMessageSignature (pubkey-free form)")
        })
}
