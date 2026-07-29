use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyBytes;
use rec_aggregation::{
    MultiMessageAggregateSignature, SingleMessageAggregateSignature, SingleMessageInfo,
};
use ssz::Encode;
use xmss::{XmssPublicKey, XmssSecretKey, XmssSignature, MESSAGE_LEN_BYTES};

use crate::error::SerializationError;
use crate::serialization::{decode_aggregate, decode_public_key, decode_signature};

/// Length-check a Python `bytes` message into the fixed array upstream wants.
/// Any 32 bytes are valid — upstream hashes the raw bytes itself.
pub(crate) fn message_array(bytes: &[u8]) -> PyResult<[u8; MESSAGE_LEN_BYTES]> {
    bytes.try_into().map_err(|_| {
        SerializationError::new_err(format!(
            "message must be exactly {} bytes, got {}",
            MESSAGE_LEN_BYTES,
            bytes.len()
        ))
    })
}

pub(crate) fn wrap_pubkeys(pks: &[XmssPublicKey]) -> Vec<PyPublicKey> {
    pks.iter()
        .cloned()
        .map(|pk| PyPublicKey {
            inner: Arc::new(pk),
        })
        .collect()
}

pub(crate) fn unwrap_pubkeys(pks: &[PyRef<'_, PyPublicKey>]) -> Vec<XmssPublicKey> {
    pks.iter().map(|p| (*p.inner).clone()).collect()
}

fn short_hex(bytes: &[u8]) -> String {
    if bytes.len() <= 8 {
        format!("0x{}", hex::encode(bytes))
    } else {
        format!(
            "0x{}…{}",
            hex::encode(&bytes[..4]),
            hex::encode(&bytes[bytes.len() - 4..])
        )
    }
}

#[pyclass(
    name = "PublicKey",
    frozen,
    module = "py_lean_multisig",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPublicKey {
    pub(crate) inner: Arc<XmssPublicKey>,
}

#[pymethods]
impl PyPublicKey {
    #[classmethod]
    fn from_bytes(_cls: &Bound<'_, pyo3::types::PyType>, data: &[u8]) -> PyResult<Self> {
        let pk = decode_public_key(data)?;
        Ok(Self {
            inner: Arc::new(pk),
        })
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.as_ssz_bytes())
    }

    fn __repr__(&self) -> String {
        format!("PublicKey({})", short_hex(&self.inner.as_ssz_bytes()))
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self.inner == other.inner),
            CompareOp::Ne => Ok(self.inner != other.inner),
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "PublicKey only supports == and !=",
            )),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.inner.hash(&mut h);
        h.finish()
    }
}

#[pyclass(
    name = "Signature",
    frozen,
    module = "py_lean_multisig",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySignature {
    pub(crate) inner: Arc<XmssSignature>,
}

#[pymethods]
impl PySignature {
    #[classmethod]
    fn from_bytes(_cls: &Bound<'_, pyo3::types::PyType>, data: &[u8]) -> PyResult<Self> {
        let sig = decode_signature(data)?;
        Ok(Self {
            inner: Arc::new(sig),
        })
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.as_ssz_bytes())
    }

    fn __repr__(&self) -> String {
        // Avoid encoding the full ~1.2KB signature for a short identifier.
        format!("Signature(h=0x{:016x})", self.__hash__())
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self.inner == other.inner),
            CompareOp::Ne => Ok(self.inner != other.inner),
            _ => Err(pyo3::exceptions::PyTypeError::new_err(
                "Signature only supports == and !=",
            )),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.inner.hash(&mut h);
        h.finish()
    }
}

// Re-derive a key across processes by persisting (seed, slot_start, slot_end)
// and calling keygen() — it's deterministic.
#[pyclass(name = "SecretKey", frozen, module = "py_lean_multisig")]
pub struct PySecretKey {
    pub(crate) inner: Arc<XmssSecretKey>,
}

#[pymethods]
impl PySecretKey {
    #[getter]
    fn public_key(&self) -> PyPublicKey {
        PyPublicKey {
            inner: Arc::new(self.inner.public_key()),
        }
    }

    #[getter]
    pub(crate) fn slot_start(&self) -> u32 {
        *self.inner.activation_slots().start()
    }

    #[getter]
    pub(crate) fn slot_end(&self) -> u32 {
        *self.inner.activation_slots().end()
    }

    fn __repr__(&self) -> String {
        let pk_bytes = self.inner.public_key().as_ssz_bytes();
        format!(
            "SecretKey(slots={}..={}, pk={})",
            self.slot_start(),
            self.slot_end(),
            short_hex(&pk_bytes)
        )
    }
}

/// Many XMSS sigs over one `(message, slot)` aggregated into a single zkVM proof.
/// Wraps upstream's `SingleMessageAggregateSignature`.
#[pyclass(
    name = "SingleMessageSignature",
    frozen,
    module = "py_lean_multisig",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySingleMessageSignature {
    pub(crate) inner: Arc<SingleMessageAggregateSignature>,
}

#[pymethods]
impl PySingleMessageSignature {
    #[classmethod]
    fn from_bytes(cls: &Bound<'_, pyo3::types::PyType>, data: &[u8]) -> PyResult<Self> {
        let sig = cls.py().detach(|| {
            decode_aggregate("SingleMessageSignature", || {
                SingleMessageAggregateSignature::from_bytes(data)
            })
        })?;
        Ok(Self {
            inner: Arc::new(sig),
        })
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.to_bytes())
    }

    /// Decode the pubkey-free form; the caller supplies the signer set.
    /// A set different from the one aggregated fails verification.
    #[classmethod]
    fn from_bytes_without_pubkeys(
        cls: &Bound<'_, pyo3::types::PyType>,
        data: &[u8],
        pubkeys: Vec<PyRef<'_, PyPublicKey>>,
    ) -> PyResult<Self> {
        let pks = unwrap_pubkeys(&pubkeys);
        let sig = cls.py().detach(|| {
            decode_aggregate("SingleMessageSignature (pubkey-free form)", || {
                SingleMessageAggregateSignature::from_bytes_without_pubkeys(data, pks)
            })
        })?;
        Ok(Self {
            inner: Arc::new(sig),
        })
    }

    /// Compact wire form with the pubkeys stripped (receiver already knows
    /// the signer set, e.g. from a validator registry).
    fn to_bytes_without_pubkeys<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.to_bytes_without_pubkeys())
    }

    #[getter]
    fn message<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.info.core.message)
    }

    #[getter]
    fn slot(&self) -> u32 {
        self.inner.info.core.slot
    }

    #[getter]
    fn pubkeys(&self) -> Vec<PyPublicKey> {
        wrap_pubkeys(&self.inner.info.pubkeys)
    }

    fn __repr__(&self) -> String {
        format!(
            "SingleMessageSignature(slot={}, n_signers={})",
            self.inner.info.core.slot,
            self.inner.info.pubkeys.len()
        )
    }
}

/// Bundles n single-message proofs, each potentially over a different
/// `(message, slot)`. Wraps upstream's `MultiMessageAggregateSignature`.
#[pyclass(
    name = "MultiMessageSignature",
    frozen,
    module = "py_lean_multisig",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMultiMessageSignature {
    pub(crate) inner: Arc<MultiMessageAggregateSignature>,
}

#[pymethods]
impl PyMultiMessageSignature {
    #[classmethod]
    fn from_bytes(cls: &Bound<'_, pyo3::types::PyType>, data: &[u8]) -> PyResult<Self> {
        let sig = cls.py().detach(|| {
            decode_aggregate("MultiMessageSignature", || {
                MultiMessageAggregateSignature::from_bytes(data)
            })
        })?;
        Ok(Self {
            inner: Arc::new(sig),
        })
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.to_bytes())
    }

    /// Decode the pubkey-free form; the caller supplies one signer set per
    /// component, in component order.
    #[classmethod]
    fn from_bytes_without_pubkeys(
        cls: &Bound<'_, pyo3::types::PyType>,
        data: &[u8],
        pubkeys_per_component: Vec<Vec<PyRef<'_, PyPublicKey>>>,
    ) -> PyResult<Self> {
        let pks: Vec<Vec<XmssPublicKey>> = pubkeys_per_component
            .iter()
            .map(|component| unwrap_pubkeys(component))
            .collect();
        let sig = cls.py().detach(|| {
            decode_aggregate("MultiMessageSignature (pubkey-free form)", || {
                MultiMessageAggregateSignature::from_bytes_without_pubkeys(data, pks)
            })
        })?;
        Ok(Self {
            inner: Arc::new(sig),
        })
    }

    /// Compact wire form with all component pubkeys stripped.
    fn to_bytes_without_pubkeys<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.to_bytes_without_pubkeys())
    }

    #[getter]
    fn components(&self) -> Vec<PyComponentInfo> {
        // Views share the Arc rather than deep-cloning each component's
        // (potentially large) pubkey list.
        (0..self.inner.info.len())
            .map(|index| PyComponentInfo {
                inner: Arc::clone(&self.inner),
                index,
            })
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.info.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "MultiMessageSignature(n_components={})",
            self.inner.info.len()
        )
    }
}

/// Read-only view of one MultiMessageSignature component's bound info.
#[pyclass(
    name = "ComponentInfo",
    frozen,
    module = "py_lean_multisig",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyComponentInfo {
    inner: Arc<MultiMessageAggregateSignature>,
    index: usize,
}

impl PyComponentInfo {
    fn info(&self) -> &SingleMessageInfo {
        &self.inner.info[self.index]
    }
}

#[pymethods]
impl PyComponentInfo {
    #[getter]
    fn message<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.info().core.message)
    }

    #[getter]
    fn slot(&self) -> u32 {
        self.info().core.slot
    }

    #[getter]
    fn pubkeys(&self) -> Vec<PyPublicKey> {
        wrap_pubkeys(&self.info().pubkeys)
    }

    fn __repr__(&self) -> String {
        format!(
            "ComponentInfo(slot={}, n_signers={})",
            self.info().core.slot,
            self.info().pubkeys.len()
        )
    }
}
