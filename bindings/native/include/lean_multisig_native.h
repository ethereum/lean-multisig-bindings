/*
 * Internal ABI between managed-language wrappers and the Rust implementation.
 *
 * This header documents this internal ABI; it is not a supported public C binding. Every
 * non-empty input range must point to readable memory. Every output pointer must be writable.
 * Handles are opaque and must be destroyed exactly once with their matching destroy function.
 * Buffers returned through lms_buffer must be released with lms_buffer_free exactly once.
 */
#ifndef LEAN_MULTISIG_NATIVE_H
#define LEAN_MULTISIG_NATIVE_H

#include <stddef.h>
#include <stdint.h>

typedef struct { uint8_t *data; size_t len; } lms_buffer;
typedef struct lms_secret_key lms_secret_key;
typedef struct lms_signature lms_signature;
typedef struct lms_multi_claim_proof lms_multi_claim_proof;

/* Returns 0 on success, 1 for a checked error, and 2 when Rust caught a panic. */
int32_t lms_setup(void);
int32_t lms_last_error(lms_buffer *out);
void lms_buffer_free(uint8_t *data, size_t len);

int32_t lms_secret_key_generate(uint32_t start, uint32_t end, lms_secret_key **out);
int32_t lms_secret_key_from_seed(const uint8_t *seed, size_t seed_len, uint32_t start, uint32_t end, lms_secret_key **out);
int32_t lms_secret_key_from_bytes(const uint8_t *data, size_t len, lms_secret_key **out);
int32_t lms_secret_key_to_bytes(lms_secret_key *key, lms_buffer *out);
int32_t lms_secret_key_public_key(lms_secret_key *key, lms_buffer *out);
int32_t lms_secret_key_slots(lms_secret_key *key, uint32_t *start, uint32_t *end);
int32_t lms_secret_key_prepare(lms_secret_key *key, uint32_t slot);
int32_t lms_secret_key_sign(lms_secret_key *key, const uint8_t *message, size_t message_len, uint32_t slot, lms_signature **out);
void lms_secret_key_destroy(lms_secret_key *key);

int32_t lms_signature_from_bytes(const uint8_t *data, size_t len, const uint8_t *message, size_t message_len, uint32_t slot, const uint8_t *signers, size_t signer_count, lms_signature **out);
int32_t lms_signature_to_bytes(lms_signature *signature, lms_buffer *out);
int32_t lms_signature_aggregate(lms_signature *const *signatures, size_t count, const uint8_t *message, size_t message_len, uint32_t slot, lms_signature **out);
int32_t lms_signature_verified_signers(lms_signature *signature, const uint8_t *message, size_t message_len, uint32_t slot, lms_buffer *out);
int32_t lms_signature_verify(lms_signature *signature, const uint8_t *signers, size_t signer_count, const uint8_t *message, size_t message_len, uint32_t slot);
void lms_signature_destroy(lms_signature *signature);

/* Context is this bridge's private LMCG-v1 binary encoding of ClaimSigners values. */
int32_t lms_multi_claim_proof_merge(lms_signature *const *signatures, size_t count, lms_multi_claim_proof **out);
int32_t lms_multi_claim_proof_from_bytes(const uint8_t *data, size_t len, const uint8_t *context, size_t context_len, lms_multi_claim_proof **out);
int32_t lms_multi_claim_proof_to_bytes(lms_multi_claim_proof *proof, lms_buffer *out);
int32_t lms_multi_claim_proof_verified_claims(lms_multi_claim_proof *proof, lms_buffer *out);
int32_t lms_multi_claim_proof_verify(lms_multi_claim_proof *proof, const uint8_t *context, size_t context_len);
void lms_multi_claim_proof_destroy(lms_multi_claim_proof *proof);

#endif
