//! A deliberately small C ABI for the Java 25 FFM binding.
//!
//! This is not a language-neutral public API.  Its opaque handles, owned buffers, and compact
//! claim-context encoding exist solely so the Java wrapper can offer normal managed objects.
#![allow(clippy::missing_safety_doc)] // Safety contract is documented in include/lean_multisig_java.h.

use lean_multisig_api as api;
use std::cell::RefCell;
use std::ffi::c_int;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

pub const LMS_OK: c_int = 0;
pub const LMS_ERROR: c_int = 1;
pub const LMS_PANIC: c_int = 2;

const CLAIM_CONTEXT_MAGIC: &[u8; 4] = b"LMCG";
const CLAIM_CONTEXT_VERSION: u8 = 1;
const PUBLIC_KEY_LEN: usize = 32;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

#[repr(C)]
#[derive(Default)]
pub struct LmsBuffer {
    pub data: *mut u8,
    pub len: usize,
}

/// Opaque ownership handle. Java only ever passes its address back to this library.
pub struct SecretKeyHandle(api::SecretKey);
/// Opaque ownership handle. Java only ever passes its address back to this library.
pub struct SignatureHandle(api::Signature);
/// Opaque ownership handle. Java only ever passes its address back to this library.
pub struct MultiClaimProofHandle(api::MultiClaimProof);

fn set_error(message: impl Into<String>) {
    LAST_ERROR.with(|error| *error.borrow_mut() = message.into());
}

fn ffi(operation: impl FnOnce() -> Result<(), String>) -> c_int {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => {
            set_error("");
            LMS_OK
        }
        Ok(Err(error)) => {
            set_error(error);
            LMS_ERROR
        }
        Err(_) => {
            set_error("native lean-multisig operation panicked");
            LMS_PANIC
        }
    }
}

unsafe fn bytes<'a>(data: *const u8, len: usize, name: &str) -> Result<&'a [u8], String> {
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(format!(
            "{name} must not be null when its length is non-zero"
        ));
    }
    Ok(std::slice::from_raw_parts(data, len))
}

unsafe fn bytes32(data: *const u8, len: usize, name: &str) -> Result<[u8; PUBLIC_KEY_LEN], String> {
    bytes(data, len, name)?
        .try_into()
        .map_err(|_| format!("{name} must be exactly {PUBLIC_KEY_LEN} bytes, got {len}"))
}

unsafe fn keys(data: *const u8, count: usize) -> Result<Vec<api::PublicKey>, String> {
    let len = count
        .checked_mul(PUBLIC_KEY_LEN)
        .ok_or_else(|| "public-key length overflow".to_owned())?;
    let keys = bytes(data, len, "public keys")?
        .chunks_exact(PUBLIC_KEY_LEN)
        .map(|key| key.try_into().expect("chunk has public-key length"))
        .collect::<Vec<_>>();
    Ok(keys)
}

unsafe fn secret_key<'a>(handle: *mut SecretKeyHandle) -> Result<&'a SecretKeyHandle, String> {
    handle
        .as_ref()
        .ok_or_else(|| "secret-key handle must not be null".to_owned())
}

unsafe fn signature<'a>(handle: *mut SignatureHandle) -> Result<&'a SignatureHandle, String> {
    handle
        .as_ref()
        .ok_or_else(|| "signature handle must not be null".to_owned())
}

unsafe fn proof<'a>(
    handle: *mut MultiClaimProofHandle,
) -> Result<&'a MultiClaimProofHandle, String> {
    handle
        .as_ref()
        .ok_or_else(|| "multi-claim-proof handle must not be null".to_owned())
}

unsafe fn put_handle<T>(out: *mut *mut T, value: T, name: &str) -> Result<(), String> {
    let out = out
        .as_mut()
        .ok_or_else(|| format!("{name} output pointer must not be null"))?;
    *out = Box::into_raw(Box::new(value));
    Ok(())
}

unsafe fn put_buffer(out: *mut LmsBuffer, value: Vec<u8>) -> Result<(), String> {
    let out = out
        .as_mut()
        .ok_or_else(|| "buffer output pointer must not be null".to_owned())?;
    let len = value.len();
    let data = Box::into_raw(value.into_boxed_slice()) as *mut u8;
    *out = LmsBuffer { data, len };
    Ok(())
}

unsafe fn signature_handles(
    handles: *const *mut SignatureHandle,
    count: usize,
) -> Result<Vec<api::Signature>, String> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if handles.is_null() {
        return Err("signature handles must not be null when count is non-zero".to_owned());
    }
    std::slice::from_raw_parts(handles, count)
        .iter()
        .map(|handle| signature(*handle).map(|handle| handle.0.clone()))
        .collect()
}

fn encode_groups(groups: &[api::ClaimSigners]) -> Result<Vec<u8>, String> {
    let count = u32::try_from(groups.len()).map_err(|_| "too many claim groups".to_owned())?;
    let mut result = Vec::new();
    result.extend_from_slice(CLAIM_CONTEXT_MAGIC);
    result.push(CLAIM_CONTEXT_VERSION);
    result.extend_from_slice(&count.to_le_bytes());
    for group in groups {
        let signer_count =
            u32::try_from(group.signers.len()).map_err(|_| "too many signers".to_owned())?;
        result.extend_from_slice(group.claim.message());
        result.extend_from_slice(&group.claim.slot().to_le_bytes());
        result.extend_from_slice(&signer_count.to_le_bytes());
        for signer in &group.signers {
            result.extend_from_slice(signer);
        }
    }
    Ok(result)
}

fn take_u32(bytes: &[u8], cursor: &mut usize, field: &str) -> Result<u32, String> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| "claim context length overflow".to_owned())?;
    let value = bytes
        .get(*cursor..end)
        .ok_or_else(|| format!("claim context ends while reading {field}"))?;
    *cursor = end;
    Ok(u32::from_le_bytes(value.try_into().expect("u32 length")))
}

fn take_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
    field: &str,
) -> Result<&'a [u8], String> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| "claim context length overflow".to_owned())?;
    let value = bytes
        .get(*cursor..end)
        .ok_or_else(|| format!("claim context ends while reading {field}"))?;
    *cursor = end;
    Ok(value)
}

fn decode_groups(bytes: &[u8]) -> Result<Vec<api::ClaimSigners>, String> {
    if bytes.len() < 9 || &bytes[..4] != CLAIM_CONTEXT_MAGIC || bytes[4] != CLAIM_CONTEXT_VERSION {
        return Err("malformed Java claim context".to_owned());
    }
    let mut cursor = 5;
    let count =
        usize::try_from(take_u32(bytes, &mut cursor, "group count")?).expect("u32 fits usize");
    if count > api::MAX_CLAIMS {
        return Err(format!("too many claim groups: {count}"));
    }
    let mut groups = Vec::with_capacity(count);
    for _ in 0..count {
        let message: [u8; PUBLIC_KEY_LEN] =
            take_bytes(bytes, &mut cursor, PUBLIC_KEY_LEN, "message")?
                .try_into()
                .expect("message length");
        let slot = take_u32(bytes, &mut cursor, "slot")?;
        let signer_count =
            usize::try_from(take_u32(bytes, &mut cursor, "signer count")?).expect("u32 fits usize");
        let signer_len = signer_count
            .checked_mul(PUBLIC_KEY_LEN)
            .ok_or_else(|| "signer list length overflow".to_owned())?;
        let signer_bytes = take_bytes(bytes, &mut cursor, signer_len, "signers")?;
        let signers = signer_bytes
            .chunks_exact(PUBLIC_KEY_LEN)
            .map(|key| key.try_into().expect("key length"))
            .collect();
        groups.push(api::ClaimSigners {
            claim: api::Claim::new(message, slot),
            signers,
        });
    }
    if cursor != bytes.len() {
        return Err("trailing bytes in Java claim context".to_owned());
    }
    Ok(groups)
}

#[no_mangle]
pub extern "C" fn lms_setup() -> c_int {
    ffi(|| {
        api::setup();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_last_error(out: *mut LmsBuffer) -> c_int {
    ffi(|| {
        let bytes = LAST_ERROR.with(|error| error.borrow().as_bytes().to_vec());
        put_buffer(out, bytes)
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_buffer_free(data: *mut u8, len: usize) {
    if !data.is_null() && len != 0 {
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(data, len)));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_generate(
    slot_start: u32,
    slot_end: u32,
    out: *mut *mut SecretKeyHandle,
) -> c_int {
    ffi(|| {
        let key =
            api::SecretKey::generate(slot_start..=slot_end).map_err(|error| error.to_string())?;
        put_handle(out, SecretKeyHandle(key), "secret-key")
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_from_seed(
    seed: *const u8,
    seed_len: usize,
    slot_start: u32,
    slot_end: u32,
    out: *mut *mut SecretKeyHandle,
) -> c_int {
    ffi(|| {
        let seed = bytes32(seed, seed_len, "seed")?;
        let key = api::SecretKey::from_seed(seed, slot_start..=slot_end)
            .map_err(|error| error.to_string())?;
        put_handle(out, SecretKeyHandle(key), "secret-key")
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_from_bytes(
    data: *const u8,
    len: usize,
    out: *mut *mut SecretKeyHandle,
) -> c_int {
    ffi(|| {
        let key = api::SecretKey::from_bytes(bytes(data, len, "secret key")?)
            .map_err(|error| error.to_string())?;
        put_handle(out, SecretKeyHandle(key), "secret-key")
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_to_bytes(
    handle: *mut SecretKeyHandle,
    out: *mut LmsBuffer,
) -> c_int {
    ffi(|| put_buffer(out, secret_key(handle)?.0.to_bytes()))
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_public_key(
    handle: *mut SecretKeyHandle,
    out: *mut LmsBuffer,
) -> c_int {
    ffi(|| put_buffer(out, secret_key(handle)?.0.public_key().to_vec()))
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_slots(
    handle: *mut SecretKeyHandle,
    slot_start: *mut u32,
    slot_end: *mut u32,
) -> c_int {
    ffi(|| {
        let slot_start = slot_start
            .as_mut()
            .ok_or_else(|| "slot-start output pointer must not be null".to_owned())?;
        let slot_end = slot_end
            .as_mut()
            .ok_or_else(|| "slot-end output pointer must not be null".to_owned())?;
        let slots = secret_key(handle)?.0.slots();
        *slot_start = *slots.start();
        *slot_end = *slots.end();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_prepare(handle: *mut SecretKeyHandle, slot: u32) -> c_int {
    ffi(|| {
        secret_key(handle)?
            .0
            .prepare(slot)
            .map_err(|error| error.to_string())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_sign(
    handle: *mut SecretKeyHandle,
    message: *const u8,
    message_len: usize,
    slot: u32,
    out: *mut *mut SignatureHandle,
) -> c_int {
    ffi(|| {
        let claim = api::Claim::new(bytes32(message, message_len, "message")?, slot);
        let signature = secret_key(handle)?
            .0
            .sign(&claim)
            .map_err(|error| error.to_string())?;
        put_handle(out, SignatureHandle(signature), "signature")
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_secret_key_destroy(handle: *mut SecretKeyHandle) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(handle))));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lms_signature_from_bytes(
    data: *const u8,
    len: usize,
    message: *const u8,
    message_len: usize,
    slot: u32,
    signer_bytes: *const u8,
    signer_count: usize,
    out: *mut *mut SignatureHandle,
) -> c_int {
    ffi(|| {
        let claim = api::Claim::new(bytes32(message, message_len, "message")?, slot);
        let signature = api::Signature::from_bytes(
            bytes(data, len, "signature")?,
            &claim,
            &keys(signer_bytes, signer_count)?,
        )
        .map_err(|error| error.to_string())?;
        put_handle(out, SignatureHandle(signature), "signature")
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_signature_to_bytes(
    handle: *mut SignatureHandle,
    out: *mut LmsBuffer,
) -> c_int {
    ffi(|| put_buffer(out, signature(handle)?.0.to_bytes()))
}

#[no_mangle]
pub unsafe extern "C" fn lms_signature_aggregate(
    handles: *const *mut SignatureHandle,
    count: usize,
    message: *const u8,
    message_len: usize,
    slot: u32,
    out: *mut *mut SignatureHandle,
) -> c_int {
    ffi(|| {
        let claim = api::Claim::new(bytes32(message, message_len, "message")?, slot);
        let signature = api::aggregate(signature_handles(handles, count)?, &claim)
            .map_err(|error| error.to_string())?;
        put_handle(out, SignatureHandle(signature), "signature")
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_signature_verified_signers(
    handle: *mut SignatureHandle,
    message: *const u8,
    message_len: usize,
    slot: u32,
    out: *mut LmsBuffer,
) -> c_int {
    ffi(|| {
        let claim = api::Claim::new(bytes32(message, message_len, "message")?, slot);
        let signers = api::verified_signers(&signature(handle)?.0, &claim)
            .map_err(|error| error.to_string())?;
        put_buffer(out, signers.into_iter().flatten().collect())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_signature_verify(
    handle: *mut SignatureHandle,
    signer_bytes: *const u8,
    signer_count: usize,
    message: *const u8,
    message_len: usize,
    slot: u32,
) -> c_int {
    ffi(|| {
        let claim = api::Claim::new(bytes32(message, message_len, "message")?, slot);
        api::verify(
            &signature(handle)?.0,
            &keys(signer_bytes, signer_count)?,
            &claim,
        )
        .map_err(|error| error.to_string())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_signature_destroy(handle: *mut SignatureHandle) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(handle))));
    }
}

#[no_mangle]
pub unsafe extern "C" fn lms_multi_claim_proof_merge(
    handles: *const *mut SignatureHandle,
    count: usize,
    out: *mut *mut MultiClaimProofHandle,
) -> c_int {
    ffi(|| {
        let proof = api::merge_claims(signature_handles(handles, count)?)
            .map_err(|error| error.to_string())?;
        put_handle(out, MultiClaimProofHandle(proof), "multi-claim-proof")
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_multi_claim_proof_from_bytes(
    data: *const u8,
    len: usize,
    context: *const u8,
    context_len: usize,
    out: *mut *mut MultiClaimProofHandle,
) -> c_int {
    ffi(|| {
        let groups = decode_groups(bytes(context, context_len, "claim context")?)?;
        let proof =
            api::MultiClaimProof::from_bytes(bytes(data, len, "multi-claim proof")?, &groups)
                .map_err(|error| error.to_string())?;
        put_handle(out, MultiClaimProofHandle(proof), "multi-claim-proof")
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_multi_claim_proof_to_bytes(
    handle: *mut MultiClaimProofHandle,
    out: *mut LmsBuffer,
) -> c_int {
    ffi(|| put_buffer(out, proof(handle)?.0.to_bytes()))
}

#[no_mangle]
pub unsafe extern "C" fn lms_multi_claim_proof_verified_claims(
    handle: *mut MultiClaimProofHandle,
    out: *mut LmsBuffer,
) -> c_int {
    ffi(|| {
        let groups = api::verified_claims(&proof(handle)?.0).map_err(|error| error.to_string())?;
        put_buffer(out, encode_groups(&groups)?)
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_multi_claim_proof_verify(
    handle: *mut MultiClaimProofHandle,
    context: *const u8,
    context_len: usize,
) -> c_int {
    ffi(|| {
        let groups = decode_groups(bytes(context, context_len, "claim context")?)?;
        api::verify_claims(&proof(handle)?.0, &groups).map_err(|error| error.to_string())
    })
}

#[no_mangle]
pub unsafe extern "C" fn lms_multi_claim_proof_destroy(handle: *mut MultiClaimProofHandle) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(handle))));
    }
}
