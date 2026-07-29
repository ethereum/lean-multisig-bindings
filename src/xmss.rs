use std::sync::Arc;

use pyo3::prelude::*;
use xmss::{
    xmss_key_gen_from_seed, xmss_sign, xmss_verify, XmssKeyGenError, XmssSignatureError,
    XmssVerifyError,
};

use crate::error::{KeygenError, SerializationError, SignError, VerifyError};
use crate::types::{message_array, PyPublicKey, PySecretKey, PySignature};

#[pyfunction]
pub fn keygen(seed: &[u8], slot_start: u32, slot_end: u32) -> PyResult<(PySecretKey, PyPublicKey)> {
    let seed_arr: [u8; 32] = seed.try_into().map_err(|_| {
        SerializationError::new_err(format!("seed must be 32 bytes, got {}", seed.len()))
    })?;
    // slot_end < slot_start yields 0 active slots, which upstream rejects as
    // InvalidRange — one error path for every bad range.
    let num_active_slots = (u64::from(slot_end) + 1).saturating_sub(u64::from(slot_start));
    let (pk, sk) = xmss_key_gen_from_seed(seed_arr, u64::from(slot_start), num_active_slots)
        .map_err(|e| match e {
            XmssKeyGenError::InvalidRange => KeygenError::new_err(format!(
                "invalid slot range: start={}, end={}",
                slot_start, slot_end
            )),
        })?;
    let py_pk = PyPublicKey {
        inner: Arc::new(pk),
    };
    let py_sk = PySecretKey {
        inner: Arc::new(sk),
    };
    Ok((py_sk, py_pk))
}

/// Signing is deterministic: the encoding randomness is derived from the
/// secret key's seed, so the same (key, message, slot) always yields the
/// same signature.
#[pyfunction]
pub fn sign(sk: &PySecretKey, message: &[u8], slot: u32) -> PyResult<PySignature> {
    let msg = message_array(message)?;
    let sig = xmss_sign(&sk.inner, slot, &msg).map_err(|e| match e {
        XmssSignatureError::SlotOutOfRange => {
            let slots = sk.inner.activation_slots();
            SignError::new_err(format!(
                "slot {} not in key range [{}, {}]",
                slot,
                slots.start(),
                slots.end()
            ))
        }
        XmssSignatureError::EncodingAttemptsExceeded => SignError::new_err(e.to_string()),
    })?;
    Ok(PySignature {
        inner: Arc::new(sig),
    })
}

#[pyfunction]
pub fn verify(pk: &PyPublicKey, message: &[u8], sig: &PySignature, slot: u32) -> PyResult<()> {
    let msg = message_array(message)?;
    xmss_verify(&pk.inner, slot, &msg, &sig.inner).map_err(|e| match e {
        XmssVerifyError::InvalidWots => VerifyError::new_err("WOTS recovery failed"),
        XmssVerifyError::InvalidMerklePath => {
            VerifyError::new_err("Merkle path does not match public key root")
        }
    })?;
    Ok(())
}
