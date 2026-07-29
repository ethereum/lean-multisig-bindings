use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;

create_exception!(py_lean_multisig, LeanMultisigError, PyException);
create_exception!(py_lean_multisig, KeygenError, LeanMultisigError);
create_exception!(py_lean_multisig, SignError, LeanMultisigError);
create_exception!(py_lean_multisig, VerifyError, LeanMultisigError);
create_exception!(py_lean_multisig, AggregationError, LeanMultisigError);
create_exception!(py_lean_multisig, SerializationError, LeanMultisigError);

/// Map upstream aggregation errors onto Python exceptions: argument-shaped
/// errors (bad sizes, indices, inconsistent inputs) become `ValueError`,
/// proving/verification failures become `AggregationError`. Upstream is the
/// single source of truth for the limits themselves — the bindings never
/// pre-check what it already validates.
pub fn aggregation_to_py_err(e: rec_aggregation::AggregationError) -> PyErr {
    use rec_aggregation::AggregationError as E;
    match e {
        E::EmptyAggregation { .. }
        | E::LimitExceeded { .. }
        | E::InvalidSplitIndex { .. }
        | E::InconsistentChildren { .. }
        | E::UnknownMessage
        | E::MultipleMessages => PyValueError::new_err(e.to_string()),
        E::Prover(_) | E::InvalidChildProof(_) => AggregationError::new_err(e.to_string()),
    }
}

pub fn register(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("LeanMultisigError", py.get_type::<LeanMultisigError>())?;
    m.add("KeygenError", py.get_type::<KeygenError>())?;
    m.add("SignError", py.get_type::<SignError>())?;
    m.add("VerifyError", py.get_type::<VerifyError>())?;
    m.add("AggregationError", py.get_type::<AggregationError>())?;
    m.add("SerializationError", py.get_type::<SerializationError>())?;
    Ok(())
}
