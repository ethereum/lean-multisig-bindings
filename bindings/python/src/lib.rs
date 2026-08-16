use std::sync::Arc;

use lean_multisig_api as api;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyBytes;
use pyo3::types::PyType;
use pyo3::wrap_pyfunction;

create_exception!(py_lean_multisig, LeanMultisigError, PyException);

fn api_error(error: api::Error) -> PyErr {
    LeanMultisigError::new_err(error.to_string())
}

fn bytes32(bytes: &[u8], name: &str) -> PyResult<[u8; 32]> {
    bytes.try_into().map_err(|_| {
        PyValueError::new_err(format!(
            "{name} must be exactly 32 bytes, got {}",
            bytes.len()
        ))
    })
}

fn public_keys(signers: Vec<Vec<u8>>) -> PyResult<Vec<api::PublicKey>> {
    signers
        .iter()
        .map(|signer| bytes32(signer, "public key"))
        .collect()
}

#[pyclass(
    name = "Claim",
    frozen,
    module = "py_lean_multisig",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyClaim {
    inner: api::Claim,
}

#[pymethods]
impl PyClaim {
    #[new]
    fn new(message: &[u8], slot: u32) -> PyResult<Self> {
        Ok(Self {
            inner: api::Claim::new(bytes32(message, "message")?, slot),
        })
    }

    #[getter]
    fn message<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.message())
    }

    #[getter]
    fn slot(&self) -> u32 {
        self.inner.slot()
    }

    fn __repr__(&self) -> String {
        format!("Claim(slot={})", self.inner.slot())
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self.inner == other.inner),
            CompareOp::Ne => Ok(self.inner != other.inner),
            _ => Err(PyValueError::new_err("Claim only supports == and !=")),
        }
    }
}

#[pyclass(name = "SecretKey", module = "py_lean_multisig")]
struct PySecretKey {
    inner: api::SecretKey,
}

#[pymethods]
impl PySecretKey {
    #[classmethod]
    fn generate(
        _cls: &Bound<'_, PyType>,
        py: Python<'_>,
        slot_start: u32,
        slot_end: u32,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: py
                .detach(move || api::SecretKey::generate(slot_start..=slot_end))
                .map_err(api_error)?,
        })
    }

    #[classmethod]
    fn from_seed(
        _cls: &Bound<'_, PyType>,
        py: Python<'_>,
        seed: &[u8],
        slot_start: u32,
        slot_end: u32,
    ) -> PyResult<Self> {
        let seed = bytes32(seed, "seed")?;
        Ok(Self {
            inner: py
                .detach(move || api::SecretKey::from_seed(seed, slot_start..=slot_end))
                .map_err(api_error)?,
        })
    }

    #[classmethod]
    fn from_bytes(_cls: &Bound<'_, PyType>, py: Python<'_>, data: &[u8]) -> PyResult<Self> {
        let data = data.to_vec();
        Ok(Self {
            inner: py
                .detach(move || api::SecretKey::from_bytes(&data))
                .map_err(api_error)?,
        })
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let bytes = py.detach(|| self.inner.to_bytes());
        PyBytes::new(py, &bytes)
    }

    #[getter]
    fn public_key<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.public_key())
    }

    #[getter]
    fn slot_start(&self) -> u32 {
        *self.inner.slots().start()
    }

    #[getter]
    fn slot_end(&self) -> u32 {
        *self.inner.slots().end()
    }

    fn prepare(&self, py: Python<'_>, slot: u32) -> PyResult<()> {
        py.detach(|| self.inner.prepare(slot)).map_err(api_error)
    }

    fn sign(&self, py: Python<'_>, claim: &PyClaim) -> PyResult<PySignature> {
        let claim = claim.inner;
        Ok(PySignature {
            inner: Arc::new(py.detach(|| self.inner.sign(&claim)).map_err(api_error)?),
        })
    }
}

#[pyclass(name = "Signature", module = "py_lean_multisig", skip_from_py_object)]
#[derive(Clone)]
struct PySignature {
    inner: Arc<api::Signature>,
}

#[pymethods]
impl PySignature {
    #[classmethod]
    fn from_bytes(
        _cls: &Bound<'_, PyType>,
        data: &[u8],
        claim: &PyClaim,
        signers: Vec<Vec<u8>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: Arc::new(
                api::Signature::from_bytes(data, &claim.inner, &public_keys(signers)?)
                    .map_err(api_error)?,
            ),
        })
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.to_bytes())
    }

    #[getter]
    fn claim(&self) -> PyClaim {
        PyClaim {
            inner: self.inner.claim(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Signature({:?})", self.inner.claim())
    }
}

#[pyclass(
    name = "ClaimSigners",
    module = "py_lean_multisig",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyClaimSigners {
    inner: api::ClaimSigners,
}

#[pymethods]
impl PyClaimSigners {
    #[new]
    fn new(claim: &PyClaim, signers: Vec<Vec<u8>>) -> PyResult<Self> {
        Ok(Self {
            inner: api::ClaimSigners {
                claim: claim.inner,
                signers: public_keys(signers)?,
            },
        })
    }

    #[getter]
    fn claim(&self) -> PyClaim {
        PyClaim {
            inner: self.inner.claim,
        }
    }

    #[getter]
    fn signers(&self) -> Vec<Vec<u8>> {
        self.inner
            .signers
            .iter()
            .map(|signer| signer.to_vec())
            .collect()
    }
}

#[pyclass(
    name = "MultiClaimProof",
    module = "py_lean_multisig",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyMultiClaimProof {
    inner: Arc<api::MultiClaimProof>,
}

#[pymethods]
impl PyMultiClaimProof {
    #[classmethod]
    fn from_bytes(
        _cls: &Bound<'_, PyType>,
        data: &[u8],
        groups: Vec<PyRef<'_, PyClaimSigners>>,
    ) -> PyResult<Self> {
        let groups = groups
            .iter()
            .map(|group| group.inner.clone())
            .collect::<Vec<_>>();
        Ok(Self {
            inner: Arc::new(api::MultiClaimProof::from_bytes(data, &groups).map_err(api_error)?),
        })
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.to_bytes())
    }
}

#[pyfunction]
fn setup(py: Python<'_>) {
    py.detach(api::setup);
}

#[pyfunction]
fn aggregate(
    py: Python<'_>,
    signatures: Vec<PyRef<'_, PySignature>>,
    claim: &PyClaim,
) -> PyResult<PySignature> {
    let signatures = signatures
        .iter()
        .map(|signature| (*signature.inner).clone())
        .collect::<Vec<_>>();
    let claim = claim.inner;
    py.detach(|| api::aggregate(signatures, &claim))
        .map(|inner| PySignature {
            inner: Arc::new(inner),
        })
        .map_err(api_error)
}

#[pyfunction]
fn verified_signers(
    py: Python<'_>,
    signature: &PySignature,
    claim: &PyClaim,
) -> PyResult<Vec<Vec<u8>>> {
    let signature = signature.inner.clone();
    let claim = claim.inner;
    py.detach(|| api::verified_signers(&signature, &claim))
        .map(|signers| signers.into_iter().map(|signer| signer.to_vec()).collect())
        .map_err(api_error)
}

#[pyfunction]
fn verify(
    py: Python<'_>,
    signature: &PySignature,
    expected_signers: Vec<Vec<u8>>,
    claim: &PyClaim,
) -> PyResult<()> {
    let signature = signature.inner.clone();
    let expected_signers = public_keys(expected_signers)?;
    let claim = claim.inner;
    py.detach(|| api::verify(&signature, &expected_signers, &claim))
        .map_err(api_error)
}

#[pyfunction]
fn merge_claims(
    py: Python<'_>,
    signatures: Vec<PyRef<'_, PySignature>>,
) -> PyResult<PyMultiClaimProof> {
    let signatures = signatures
        .iter()
        .map(|signature| (*signature.inner).clone())
        .collect::<Vec<_>>();
    py.detach(|| api::merge_claims(signatures))
        .map(|inner| PyMultiClaimProof {
            inner: Arc::new(inner),
        })
        .map_err(api_error)
}

#[pyfunction]
fn verified_claims(py: Python<'_>, proof: &PyMultiClaimProof) -> PyResult<Vec<PyClaimSigners>> {
    let proof = proof.inner.clone();
    py.detach(|| api::verified_claims(&proof))
        .map(|groups| {
            groups
                .into_iter()
                .map(|inner| PyClaimSigners { inner })
                .collect()
        })
        .map_err(api_error)
}

#[pyfunction]
fn verify_claims(
    py: Python<'_>,
    proof: &PyMultiClaimProof,
    expected: Vec<PyRef<'_, PyClaimSigners>>,
) -> PyResult<()> {
    let proof = proof.inner.clone();
    let expected = expected
        .iter()
        .map(|group| group.inner.clone())
        .collect::<Vec<_>>();
    py.detach(|| api::verify_claims(&proof, &expected))
        .map_err(api_error)
}

#[pymodule]
fn py_lean_multisig(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add("LeanMultisigError", py.get_type::<LeanMultisigError>())?;
    module.add_class::<PyClaim>()?;
    module.add_class::<PySecretKey>()?;
    module.add_class::<PySignature>()?;
    module.add_class::<PyClaimSigners>()?;
    module.add_class::<PyMultiClaimProof>()?;
    module.add_function(wrap_pyfunction!(setup, module)?)?;
    module.add_function(wrap_pyfunction!(aggregate, module)?)?;
    module.add_function(wrap_pyfunction!(verified_signers, module)?)?;
    module.add_function(wrap_pyfunction!(verify, module)?)?;
    module.add_function(wrap_pyfunction!(merge_claims, module)?)?;
    module.add_function(wrap_pyfunction!(verified_claims, module)?)?;
    module.add_function(wrap_pyfunction!(verify_claims, module)?)?;
    Ok(())
}
