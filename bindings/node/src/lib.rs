use std::sync::Arc;

use lean_multisig_api as api;
use napi::bindgen_prelude::{AsyncTask, Buffer, Error, Result, Status, Task};
use napi::Env;
use napi_derive::napi;

fn api_error(error: api::Error) -> Error {
    Error::new(Status::GenericFailure, error.to_string())
}

fn bytes32(bytes: &[u8], name: &str) -> Result<[u8; 32]> {
    bytes.try_into().map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("{name} must be exactly 32 bytes, got {}", bytes.len()),
        )
    })
}

fn public_keys(signers: Vec<Buffer>) -> Result<Vec<api::PublicKey>> {
    signers
        .iter()
        .map(|signer| bytes32(signer, "public key"))
        .collect()
}

#[napi]
pub struct Claim {
    inner: api::Claim,
}

#[napi]
impl Claim {
    #[napi(constructor)]
    pub fn new(message: Buffer, slot: u32) -> Result<Self> {
        Ok(Self {
            inner: api::Claim::new(bytes32(&message, "message")?, slot),
        })
    }

    #[napi(getter)]
    pub fn message(&self) -> Buffer {
        self.inner.message().to_vec().into()
    }

    #[napi(getter)]
    pub fn slot(&self) -> u32 {
        self.inner.slot()
    }
}

#[napi]
pub struct SecretKey {
    inner: Arc<api::SecretKey>,
}

#[napi]
impl SecretKey {
    #[napi]
    pub fn to_bytes(&self) -> Buffer {
        self.inner.to_bytes().into()
    }

    #[napi(getter)]
    pub fn public_key(&self) -> Buffer {
        self.inner.public_key().to_vec().into()
    }

    #[napi(getter)]
    pub fn slot_start(&self) -> u32 {
        *self.inner.slots().start()
    }

    #[napi(getter)]
    pub fn slot_end(&self) -> u32 {
        *self.inner.slots().end()
    }

    #[napi]
    pub fn prepare(&self, slot: u32) -> AsyncTask<PrepareKeyTask> {
        AsyncTask::new(PrepareKeyTask {
            key: self.inner.clone(),
            slot,
        })
    }

    #[napi]
    pub fn sign(&self, claim: &Claim) -> AsyncTask<SignTask> {
        AsyncTask::new(SignTask {
            key: self.inner.clone(),
            claim: claim.inner,
        })
    }
}

#[napi]
pub struct Signature {
    inner: Arc<api::Signature>,
}

#[napi]
impl Signature {
    #[napi(factory)]
    pub fn from_bytes(data: Buffer, claim: &Claim, signers: Vec<Buffer>) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(
                api::Signature::from_bytes(&data, &claim.inner, &public_keys(signers)?)
                    .map_err(api_error)?,
            ),
        })
    }

    #[napi]
    pub fn to_bytes(&self) -> Buffer {
        self.inner.to_bytes().into()
    }

    #[napi(getter)]
    pub fn claim(&self) -> Claim {
        Claim {
            inner: self.inner.claim(),
        }
    }
}

#[napi]
pub struct ClaimSigners {
    inner: api::ClaimSigners,
}

#[napi]
impl ClaimSigners {
    #[napi(constructor)]
    pub fn new(claim: &Claim, signers: Vec<Buffer>) -> Result<Self> {
        Ok(Self {
            inner: api::ClaimSigners {
                claim: claim.inner,
                signers: public_keys(signers)?,
            },
        })
    }

    #[napi(getter)]
    pub fn claim(&self) -> Claim {
        Claim {
            inner: self.inner.claim,
        }
    }

    #[napi(getter)]
    pub fn signers(&self) -> Vec<Buffer> {
        self.inner
            .signers
            .iter()
            .map(|signer| signer.to_vec().into())
            .collect()
    }
}

#[napi]
pub struct MultiClaimProof {
    inner: Arc<api::MultiClaimProof>,
}

#[napi]
impl MultiClaimProof {
    #[napi(factory)]
    pub fn from_bytes(data: Buffer, groups: Vec<&ClaimSigners>) -> Result<Self> {
        let groups = groups
            .iter()
            .map(|group| group.inner.clone())
            .collect::<Vec<_>>();
        Ok(Self {
            inner: Arc::new(api::MultiClaimProof::from_bytes(&data, &groups).map_err(api_error)?),
        })
    }

    #[napi]
    pub fn to_bytes(&self) -> Buffer {
        self.inner.to_bytes().into()
    }
}

pub struct GenerateKeyTask {
    slot_start: u32,
    slot_end: u32,
}

impl Task for GenerateKeyTask {
    type Output = api::SecretKey;
    type JsValue = SecretKey;

    fn compute(&mut self) -> Result<Self::Output> {
        api::SecretKey::generate(self.slot_start..=self.slot_end).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(SecretKey {
            inner: Arc::new(output),
        })
    }
}

pub struct SeedKeyTask {
    seed: [u8; 32],
    slot_start: u32,
    slot_end: u32,
}

impl Task for SeedKeyTask {
    type Output = api::SecretKey;
    type JsValue = SecretKey;

    fn compute(&mut self) -> Result<Self::Output> {
        api::SecretKey::from_seed(self.seed, self.slot_start..=self.slot_end).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(SecretKey {
            inner: Arc::new(output),
        })
    }
}

pub struct BytesKeyTask {
    data: Vec<u8>,
}

impl Task for BytesKeyTask {
    type Output = api::SecretKey;
    type JsValue = SecretKey;

    fn compute(&mut self) -> Result<Self::Output> {
        api::SecretKey::from_bytes(&self.data).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(SecretKey {
            inner: Arc::new(output),
        })
    }
}

pub struct PrepareKeyTask {
    key: Arc<api::SecretKey>,
    slot: u32,
}

impl Task for PrepareKeyTask {
    type Output = ();
    type JsValue = ();

    fn compute(&mut self) -> Result<Self::Output> {
        self.key.prepare(self.slot).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

pub struct SignTask {
    key: Arc<api::SecretKey>,
    claim: api::Claim,
}

impl Task for SignTask {
    type Output = api::Signature;
    type JsValue = Signature;

    fn compute(&mut self) -> Result<Self::Output> {
        self.key.sign(&self.claim).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(Signature {
            inner: Arc::new(output),
        })
    }
}

pub struct SetupTask;

impl Task for SetupTask {
    type Output = ();
    type JsValue = ();

    fn compute(&mut self) -> Result<Self::Output> {
        api::setup();
        Ok(())
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

pub struct AggregateTask {
    signatures: Vec<api::Signature>,
    claim: api::Claim,
}

pub struct VerifiedSignersTask {
    signature: Arc<api::Signature>,
    claim: api::Claim,
}

impl Task for VerifiedSignersTask {
    type Output = Vec<api::PublicKey>;
    type JsValue = Vec<Buffer>;

    fn compute(&mut self) -> Result<Self::Output> {
        api::verified_signers(&self.signature, &self.claim).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output
            .into_iter()
            .map(|signer| signer.to_vec().into())
            .collect())
    }
}

pub struct MergeClaimsTask {
    signatures: Vec<api::Signature>,
}

impl Task for MergeClaimsTask {
    type Output = api::MultiClaimProof;
    type JsValue = MultiClaimProof;

    fn compute(&mut self) -> Result<Self::Output> {
        api::merge_claims(self.signatures.clone()).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(MultiClaimProof {
            inner: Arc::new(output),
        })
    }
}

pub struct VerifiedClaimsTask {
    proof: Arc<api::MultiClaimProof>,
}

impl Task for VerifiedClaimsTask {
    type Output = Vec<api::ClaimSigners>;
    type JsValue = Vec<ClaimSigners>;

    fn compute(&mut self) -> Result<Self::Output> {
        api::verified_claims(&self.proof).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output
            .into_iter()
            .map(|inner| ClaimSigners { inner })
            .collect())
    }
}

pub struct VerifyClaimsTask {
    proof: Arc<api::MultiClaimProof>,
    expected: Vec<api::ClaimSigners>,
}

impl Task for VerifyClaimsTask {
    type Output = ();
    type JsValue = ();

    fn compute(&mut self) -> Result<Self::Output> {
        api::verify_claims(&self.proof, &self.expected).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

impl Task for AggregateTask {
    type Output = api::Signature;
    type JsValue = Signature;

    fn compute(&mut self) -> Result<Self::Output> {
        api::aggregate(self.signatures.clone(), &self.claim).map_err(api_error)
    }

    fn resolve(&mut self, _: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(Signature {
            inner: Arc::new(output),
        })
    }
}

#[napi]
pub fn setup() -> AsyncTask<SetupTask> {
    AsyncTask::new(SetupTask)
}

#[napi]
pub fn generate_secret_key(slot_start: u32, slot_end: u32) -> AsyncTask<GenerateKeyTask> {
    AsyncTask::new(GenerateKeyTask {
        slot_start,
        slot_end,
    })
}

#[napi]
pub fn secret_key_from_seed(
    seed: Buffer,
    slot_start: u32,
    slot_end: u32,
) -> Result<AsyncTask<SeedKeyTask>> {
    Ok(AsyncTask::new(SeedKeyTask {
        seed: bytes32(&seed, "seed")?,
        slot_start,
        slot_end,
    }))
}

#[napi]
pub fn secret_key_from_bytes(data: Buffer) -> AsyncTask<BytesKeyTask> {
    AsyncTask::new(BytesKeyTask {
        data: data.to_vec(),
    })
}

#[napi]
pub fn aggregate(signatures: Vec<&Signature>, claim: &Claim) -> AsyncTask<AggregateTask> {
    AsyncTask::new(AggregateTask {
        signatures: signatures
            .iter()
            .map(|signature| (*signature.inner).clone())
            .collect(),
        claim: claim.inner,
    })
}

#[napi]
pub fn verified_signers(signature: &Signature, claim: &Claim) -> AsyncTask<VerifiedSignersTask> {
    AsyncTask::new(VerifiedSignersTask {
        signature: signature.inner.clone(),
        claim: claim.inner,
    })
}

#[napi]
pub fn verify(signature: &Signature, expected_signers: Vec<Buffer>, claim: &Claim) -> Result<()> {
    api::verify(
        &signature.inner,
        &public_keys(expected_signers)?,
        &claim.inner,
    )
    .map_err(api_error)
}

#[napi]
pub fn merge_claims(signatures: Vec<&Signature>) -> AsyncTask<MergeClaimsTask> {
    AsyncTask::new(MergeClaimsTask {
        signatures: signatures
            .iter()
            .map(|signature| (*signature.inner).clone())
            .collect(),
    })
}

#[napi]
pub fn verified_claims(proof: &MultiClaimProof) -> AsyncTask<VerifiedClaimsTask> {
    AsyncTask::new(VerifiedClaimsTask {
        proof: proof.inner.clone(),
    })
}

#[napi]
pub fn verify_claims(
    proof: &MultiClaimProof,
    expected: Vec<&ClaimSigners>,
) -> AsyncTask<VerifyClaimsTask> {
    AsyncTask::new(VerifyClaimsTask {
        proof: proof.inner.clone(),
        expected: expected.iter().map(|group| group.inner.clone()).collect(),
    })
}
