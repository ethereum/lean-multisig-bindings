//! Benchmark support for comparing lean-multisig with Lighthouse BLS.
//!
//! This crate supports the repository's comparison benchmarks and is not a
//! supported public API.

use anyhow::{anyhow, ensure, Context, Result};

pub const MAX_DISTINCT_CLAIMS: usize = lean_multisig::MAX_CLAIMS;

pub struct FixtureSet {
    lean_claims: Vec<lean_multisig::Claim>,
    lean_keys: Vec<lean_multisig::SecretKey>,
    lean_signatures: Vec<lean_multisig::Signature>,
    lean_public_keys: Vec<lean_multisig::PublicKey>,
    bls_messages: Vec<lighthouse_bls::Hash256>,
    bls_keys: Vec<lighthouse_bls::SecretKey>,
    bls_signatures: Vec<lighthouse_bls::Signature>,
    bls_public_keys: Vec<lighthouse_bls::PublicKey>,
}

impl FixtureSet {
    pub fn same_claim(count: usize) -> Result<Self> {
        Self::build(count, false)
    }

    pub fn distinct_claims(count: usize) -> Result<Self> {
        if count > MAX_DISTINCT_CLAIMS {
            return Err(anyhow!(
                "distinct-claim fixture count {count} exceeds maximum {MAX_DISTINCT_CLAIMS}"
            ));
        }

        Self::build(count, true)
    }

    fn build(count: usize, distinct_claims: bool) -> Result<Self> {
        ensure!(count > 0, "fixture count must be greater than zero");

        let mut lean_claims = Vec::with_capacity(count);
        let mut lean_keys = Vec::with_capacity(count);
        let mut lean_signatures = Vec::with_capacity(count);
        let mut lean_public_keys = Vec::with_capacity(count);
        let mut bls_messages = Vec::with_capacity(count);
        let mut bls_keys = Vec::with_capacity(count);
        let mut bls_signatures = Vec::with_capacity(count);
        let mut bls_public_keys = Vec::with_capacity(count);

        for index in 0..count {
            let message_index = if distinct_claims { index } else { 0 };
            let message = indexed_bytes(message_index)
                .with_context(|| format!("failed to create message for signer {index}"))?;
            let slot = if distinct_claims {
                u32::try_from(index)
                    .with_context(|| format!("slot does not fit in u32 for signer {index}"))?
            } else {
                0
            };
            let claim = lean_multisig::Claim::new(message, slot);

            let lean_seed = indexed_bytes(index)
                .with_context(|| format!("failed to create XMSS seed for signer {index}"))?;
            let lean_key =
                lean_multisig::SecretKey::from_seed(lean_seed, 0..=MAX_DISTINCT_CLAIMS as u32)
                    .with_context(|| format!("failed to create XMSS key for signer {index}"))?;
            let lean_public_key = lean_key.public_key();
            let lean_signature = lean_key
                .sign(&claim)
                .with_context(|| format!("failed to create XMSS signature for signer {index}"))?;

            let bls_secret = indexed_bytes(index)
                .with_context(|| format!("failed to create BLS secret for signer {index}"))?;
            let bls_key = lighthouse_bls::SecretKey::deserialize(&bls_secret).map_err(|error| {
                anyhow!("failed to deserialize BLS key for signer {index}: {error:?}")
            })?;
            let bls_public_key = bls_key.public_key();
            let bls_message = lighthouse_bls::Hash256::from(message);
            let bls_signature = bls_key.sign(bls_message);

            lean_claims.push(claim);
            lean_keys.push(lean_key);
            lean_signatures.push(lean_signature);
            lean_public_keys.push(lean_public_key);
            bls_messages.push(bls_message);
            bls_keys.push(bls_key);
            bls_signatures.push(bls_signature);
            bls_public_keys.push(bls_public_key);
        }

        let fixtures = Self {
            lean_claims,
            lean_keys,
            lean_signatures,
            lean_public_keys,
            bls_messages,
            bls_keys,
            bls_signatures,
            bls_public_keys,
        };
        fixtures.validate_raw()?;
        Ok(fixtures)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lean_keys.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lean_keys.is_empty()
    }

    #[must_use]
    pub fn lean_claims(&self) -> &[lean_multisig::Claim] {
        &self.lean_claims
    }

    #[must_use]
    pub fn lean_keys(&self) -> &[lean_multisig::SecretKey] {
        &self.lean_keys
    }

    #[must_use]
    pub fn lean_signatures(&self) -> &[lean_multisig::Signature] {
        &self.lean_signatures
    }

    #[must_use]
    pub fn lean_public_keys(&self) -> &[lean_multisig::PublicKey] {
        &self.lean_public_keys
    }

    #[must_use]
    pub fn bls_messages(&self) -> &[lighthouse_bls::Hash256] {
        &self.bls_messages
    }

    #[must_use]
    pub fn bls_keys(&self) -> &[lighthouse_bls::SecretKey] {
        &self.bls_keys
    }

    #[must_use]
    pub fn bls_signatures(&self) -> &[lighthouse_bls::Signature] {
        &self.bls_signatures
    }

    #[must_use]
    pub fn bls_public_keys(&self) -> &[lighthouse_bls::PublicKey] {
        &self.bls_public_keys
    }

    pub fn validate_raw(&self) -> Result<()> {
        let expected_len = self.len();
        ensure!(
            [
                self.lean_claims.len(),
                self.lean_signatures.len(),
                self.lean_public_keys.len(),
                self.bls_messages.len(),
                self.bls_keys.len(),
                self.bls_signatures.len(),
                self.bls_public_keys.len(),
            ]
            .into_iter()
            .all(|len| len == expected_len),
            "fixture vectors have inconsistent lengths"
        );

        for index in 0..expected_len {
            lean_multisig::verify(
                &self.lean_signatures[index],
                std::slice::from_ref(&self.lean_public_keys[index]),
                &self.lean_claims[index],
            )
            .with_context(|| format!("XMSS signature failed validation for signer {index}"))?;

            ensure!(
                self.bls_signatures[index]
                    .verify(&self.bls_public_keys[index], self.bls_messages[index]),
                "BLS signature failed validation for signer {index}"
            );
        }

        Ok(())
    }
}

fn indexed_bytes(index: usize) -> Result<[u8; 32]> {
    let value = u64::try_from(index)
        .ok()
        .and_then(|index| index.checked_add(1))
        .with_context(|| format!("signer index {index} cannot be encoded as index + 1"))?;
    let mut bytes = [0; 32];
    bytes[24..].copy_from_slice(&value.to_be_bytes());
    Ok(bytes)
}
