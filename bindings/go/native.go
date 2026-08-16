//go:build cgo

// Package leanmultisig provides Go bindings for XMSS signatures and recursive aggregation.
package leanmultisig

// #cgo CFLAGS: -I${SRCDIR}/internal/native/include
// #include "lean_multisig_native.h"
import "C"

import (
	"encoding/binary"
	"errors"
	"fmt"
	"math"
	"runtime"
	"sort"
	"sync"
	"unsafe"
)

const (
	statusOK    = C.int32_t(0)
	statusError = C.int32_t(1)
)

// Claim is a 32-byte message paired with an unsigned 32-bit slot.
type Claim struct {
	Message [32]byte
	Slot    uint32
}

// ClaimSigners associates one claim with its exact authorized public-key set.
type ClaimSigners struct {
	Claim   Claim
	Signers [][32]byte
}

// SecretKey owns an XMSS secret key in the native bridge. Call Close when finished.
type SecretKey struct {
	mu     sync.Mutex
	handle *C.lms_secret_key
}

// Signature owns a raw or aggregated signature in the native bridge. Call Close when finished.
type Signature struct {
	mu     sync.Mutex
	handle *C.lms_signature
}

// MultiClaimProof owns a proof binding several claims to signer sets. Call Close when finished.
type MultiClaimProof struct {
	mu     sync.Mutex
	handle *C.lms_multi_claim_proof
}

// Setup initializes process-wide recursive-proof resources. It is safe to call repeatedly.
func Setup() error {
	return checkStatus(C.lms_setup())
}

// GenerateSecretKey creates a random key usable for the inclusive slot range.
func GenerateSecretKey(slotStart, slotEnd uint32) (*SecretKey, error) {
	var handle *C.lms_secret_key
	if err := checkStatus(C.lms_secret_key_generate(C.uint32_t(slotStart), C.uint32_t(slotEnd), &handle)); err != nil {
		return nil, err
	}
	return &SecretKey{handle: handle}, nil
}

// SecretKeyFromSeed deterministically creates a key usable for the inclusive slot range.
func SecretKeyFromSeed(seed [32]byte, slotStart, slotEnd uint32) (*SecretKey, error) {
	var handle *C.lms_secret_key
	status := C.lms_secret_key_from_seed(
		(*C.uint8_t)(unsafe.Pointer(&seed[0])), C.size_t(len(seed)),
		C.uint32_t(slotStart), C.uint32_t(slotEnd), &handle,
	)
	runtime.KeepAlive(seed)
	if err := checkStatus(status); err != nil {
		return nil, err
	}
	return &SecretKey{handle: handle}, nil
}

// SecretKeyFromBytes restores a key from its canonical byte encoding.
func SecretKeyFromBytes(bytes []byte) (*SecretKey, error) {
	var handle *C.lms_secret_key
	status := C.lms_secret_key_from_bytes(bytesPointer(bytes), C.size_t(len(bytes)), &handle)
	runtime.KeepAlive(bytes)
	if err := checkStatus(status); err != nil {
		return nil, err
	}
	return &SecretKey{handle: handle}, nil
}

// Bytes returns the canonical encoding of the secret key.
func (key *SecretKey) Bytes() ([]byte, error) {
	key.mu.Lock()
	defer key.mu.Unlock()
	if key.handle == nil {
		return nil, errClosed("secret key")
	}
	return bufferResult(func(output *C.lms_buffer) C.int32_t {
		return C.lms_secret_key_to_bytes(key.handle, output)
	})
}

// PublicKey returns the key's canonical 32-byte public key.
func (key *SecretKey) PublicKey() ([32]byte, error) {
	key.mu.Lock()
	defer key.mu.Unlock()
	if key.handle == nil {
		return [32]byte{}, errClosed("secret key")
	}
	encoded, err := bufferResult(func(output *C.lms_buffer) C.int32_t {
		return C.lms_secret_key_public_key(key.handle, output)
	})
	if err != nil {
		return [32]byte{}, err
	}
	if len(encoded) != 32 {
		return [32]byte{}, fmt.Errorf("leanmultisig: native public key has %d bytes", len(encoded))
	}
	var keyBytes [32]byte
	copy(keyBytes[:], encoded)
	return keyBytes, nil
}

// Slots returns the inclusive slot range accepted by the key.
func (key *SecretKey) Slots() (uint32, uint32, error) {
	key.mu.Lock()
	defer key.mu.Unlock()
	if key.handle == nil {
		return 0, 0, errClosed("secret key")
	}
	var start, end C.uint32_t
	if err := checkStatus(C.lms_secret_key_slots(key.handle, &start, &end)); err != nil {
		return 0, 0, err
	}
	return uint32(start), uint32(end), nil
}

// Prepare precomputes key material for a slot.
func (key *SecretKey) Prepare(slot uint32) error {
	key.mu.Lock()
	defer key.mu.Unlock()
	if key.handle == nil {
		return errClosed("secret key")
	}
	return checkStatus(C.lms_secret_key_prepare(key.handle, C.uint32_t(slot)))
}

// Sign creates a signature for claim.
func (key *SecretKey) Sign(claim Claim) (*Signature, error) {
	key.mu.Lock()
	defer key.mu.Unlock()
	if key.handle == nil {
		return nil, errClosed("secret key")
	}
	var handle *C.lms_signature
	status := C.lms_secret_key_sign(
		key.handle, (*C.uint8_t)(unsafe.Pointer(&claim.Message[0])), C.size_t(len(claim.Message)),
		C.uint32_t(claim.Slot), &handle,
	)
	runtime.KeepAlive(claim)
	if err := checkStatus(status); err != nil {
		return nil, err
	}
	return &Signature{handle: handle}, nil
}

// Close releases the native key. It is safe to call repeatedly.
func (key *SecretKey) Close() {
	if key == nil {
		return
	}
	key.mu.Lock()
	defer key.mu.Unlock()
	if key.handle != nil {
		C.lms_secret_key_destroy(key.handle)
		key.handle = nil
	}
}

// SignatureFromBytes restores a signature using its explicit claim and signer context.
func SignatureFromBytes(bytes []byte, claim Claim, signers [][32]byte) (*Signature, error) {
	flattened := flattenSigners(signers)
	var handle *C.lms_signature
	status := C.lms_signature_from_bytes(
		bytesPointer(bytes), C.size_t(len(bytes)),
		(*C.uint8_t)(unsafe.Pointer(&claim.Message[0])), C.size_t(len(claim.Message)), C.uint32_t(claim.Slot),
		bytesPointer(flattened), C.size_t(len(signers)), &handle,
	)
	runtime.KeepAlive(bytes)
	runtime.KeepAlive(claim)
	runtime.KeepAlive(flattened)
	if err := checkStatus(status); err != nil {
		return nil, err
	}
	return &Signature{handle: handle}, nil
}

// Bytes returns the cryptographic signature encoding without its claim/signer context.
func (signature *Signature) Bytes() ([]byte, error) {
	handle, release, err := signature.acquire()
	if err != nil {
		return nil, err
	}
	defer release()
	return bufferResult(func(output *C.lms_buffer) C.int32_t {
		return C.lms_signature_to_bytes(handle, output)
	})
}

// Close releases the native signature. It is safe to call repeatedly.
func (signature *Signature) Close() {
	if signature == nil {
		return
	}
	signature.mu.Lock()
	defer signature.mu.Unlock()
	if signature.handle != nil {
		C.lms_signature_destroy(signature.handle)
		signature.handle = nil
	}
}

// Aggregate combines raw or aggregated signatures for claim. Call Setup first.
func Aggregate(signatures []*Signature, claim Claim) (*Signature, error) {
	handles, release, err := acquireSignatures(signatures)
	if err != nil {
		return nil, err
	}
	defer release()
	var output *C.lms_signature
	status := C.lms_signature_aggregate(
		signaturePointer(handles), C.size_t(len(handles)),
		(*C.uint8_t)(unsafe.Pointer(&claim.Message[0])), C.size_t(len(claim.Message)), C.uint32_t(claim.Slot), &output,
	)
	runtime.KeepAlive(handles)
	runtime.KeepAlive(claim)
	if err := checkStatus(status); err != nil {
		return nil, err
	}
	return &Signature{handle: output}, nil
}

// VerifiedSigners verifies signature and returns its canonical signer set.
func VerifiedSigners(signature *Signature, claim Claim) ([][32]byte, error) {
	handle, release, err := signature.acquire()
	if err != nil {
		return nil, err
	}
	defer release()
	encoded, err := bufferResult(func(output *C.lms_buffer) C.int32_t {
		return C.lms_signature_verified_signers(
			handle, (*C.uint8_t)(unsafe.Pointer(&claim.Message[0])), C.size_t(len(claim.Message)), C.uint32_t(claim.Slot), output,
		)
	})
	runtime.KeepAlive(claim)
	if err != nil {
		return nil, err
	}
	return splitSigners(encoded)
}

// Verify reports whether signature proves exactly signers for claim. A false result is a normal verification failure.
func Verify(signature *Signature, signers [][32]byte, claim Claim) (bool, error) {
	handle, release, err := signature.acquire()
	if err != nil {
		return false, err
	}
	defer release()
	flattened := flattenSigners(signers)
	status := C.lms_signature_verify(
		handle, bytesPointer(flattened), C.size_t(len(signers)),
		(*C.uint8_t)(unsafe.Pointer(&claim.Message[0])), C.size_t(len(claim.Message)), C.uint32_t(claim.Slot),
	)
	runtime.KeepAlive(flattened)
	runtime.KeepAlive(claim)
	return verificationStatus(status)
}

// MergeClaims creates one proof covering the supplied signatures. Call Setup first.
func MergeClaims(signatures []*Signature) (*MultiClaimProof, error) {
	handles, release, err := acquireSignatures(signatures)
	if err != nil {
		return nil, err
	}
	defer release()
	var output *C.lms_multi_claim_proof
	if err := checkStatus(C.lms_multi_claim_proof_merge(signaturePointer(handles), C.size_t(len(handles)), &output)); err != nil {
		return nil, err
	}
	runtime.KeepAlive(handles)
	return &MultiClaimProof{handle: output}, nil
}

// MultiClaimProofFromBytes restores a proof using explicit claim/signer context.
func MultiClaimProofFromBytes(bytes []byte, groups []ClaimSigners) (*MultiClaimProof, error) {
	context, err := encodeGroups(groups)
	if err != nil {
		return nil, err
	}
	var output *C.lms_multi_claim_proof
	status := C.lms_multi_claim_proof_from_bytes(
		bytesPointer(bytes), C.size_t(len(bytes)), bytesPointer(context), C.size_t(len(context)), &output,
	)
	runtime.KeepAlive(bytes)
	runtime.KeepAlive(context)
	if err := checkStatus(status); err != nil {
		return nil, err
	}
	return &MultiClaimProof{handle: output}, nil
}

// Bytes returns the cryptographic proof encoding without its claim/signer context.
func (proof *MultiClaimProof) Bytes() ([]byte, error) {
	handle, release, err := proof.acquire()
	if err != nil {
		return nil, err
	}
	defer release()
	return bufferResult(func(output *C.lms_buffer) C.int32_t {
		return C.lms_multi_claim_proof_to_bytes(handle, output)
	})
}

// VerifiedClaims verifies proof and returns canonical claim/signer groups.
func VerifiedClaims(proof *MultiClaimProof) ([]ClaimSigners, error) {
	handle, release, err := proof.acquire()
	if err != nil {
		return nil, err
	}
	defer release()
	context, err := bufferResult(func(output *C.lms_buffer) C.int32_t {
		return C.lms_multi_claim_proof_verified_claims(handle, output)
	})
	if err != nil {
		return nil, err
	}
	return decodeGroups(context)
}

// VerifyClaims reports whether proof proves exactly groups. A false result is a normal verification failure.
func VerifyClaims(proof *MultiClaimProof, groups []ClaimSigners) (bool, error) {
	context, err := encodeGroups(groups)
	if err != nil {
		return false, err
	}
	handle, release, err := proof.acquire()
	if err != nil {
		return false, err
	}
	defer release()
	status := C.lms_multi_claim_proof_verify(handle, bytesPointer(context), C.size_t(len(context)))
	runtime.KeepAlive(context)
	return verificationStatus(status)
}

// Close releases the native proof. It is safe to call repeatedly.
func (proof *MultiClaimProof) Close() {
	if proof == nil {
		return
	}
	proof.mu.Lock()
	defer proof.mu.Unlock()
	if proof.handle != nil {
		C.lms_multi_claim_proof_destroy(proof.handle)
		proof.handle = nil
	}
}

func (signature *Signature) acquire() (*C.lms_signature, func(), error) {
	if signature == nil {
		return nil, nil, errors.New("leanmultisig: signature is nil")
	}
	signature.mu.Lock()
	if signature.handle == nil {
		signature.mu.Unlock()
		return nil, nil, errClosed("signature")
	}
	return signature.handle, signature.mu.Unlock, nil
}

func (proof *MultiClaimProof) acquire() (*C.lms_multi_claim_proof, func(), error) {
	if proof == nil {
		return nil, nil, errors.New("leanmultisig: multi-claim proof is nil")
	}
	proof.mu.Lock()
	if proof.handle == nil {
		proof.mu.Unlock()
		return nil, nil, errClosed("multi-claim proof")
	}
	return proof.handle, proof.mu.Unlock, nil
}

func acquireSignatures(signatures []*Signature) ([]*C.lms_signature, func(), error) {
	if len(signatures) == 0 {
		return nil, nil, errors.New("leanmultisig: signatures must not be empty")
	}
	unique := make(map[*Signature]struct{}, len(signatures))
	ordered := make([]*Signature, 0, len(signatures))
	for _, signature := range signatures {
		if signature == nil {
			return nil, nil, errors.New("leanmultisig: signature is nil")
		}
		if _, exists := unique[signature]; !exists {
			unique[signature] = struct{}{}
			ordered = append(ordered, signature)
		}
	}
	sort.Slice(ordered, func(left, right int) bool {
		return uintptr(unsafe.Pointer(ordered[left])) < uintptr(unsafe.Pointer(ordered[right]))
	})
	for _, signature := range ordered {
		signature.mu.Lock()
	}
	for _, signature := range ordered {
		if signature.handle == nil {
			for index := len(ordered) - 1; index >= 0; index-- {
				ordered[index].mu.Unlock()
			}
			return nil, nil, errClosed("signature")
		}
	}
	handles := make([]*C.lms_signature, len(signatures))
	for index, signature := range signatures {
		handles[index] = signature.handle
	}
	return handles, func() {
		for index := len(ordered) - 1; index >= 0; index-- {
			ordered[index].mu.Unlock()
		}
	}, nil
}

func checkStatus(status C.int32_t) error {
	if status == statusOK {
		return nil
	}
	return nativeError(status)
}

func verificationStatus(status C.int32_t) (bool, error) {
	if status == statusOK {
		return true, nil
	}
	if status == statusError {
		return false, nil
	}
	return false, nativeError(status)
}

func nativeError(status C.int32_t) error {
	var buffer C.lms_buffer
	if C.lms_last_error(&buffer) != statusOK {
		return fmt.Errorf("leanmultisig: native operation failed with status %d", status)
	}
	message, err := takeBuffer(&buffer)
	if err != nil || len(message) == 0 {
		return fmt.Errorf("leanmultisig: native operation failed with status %d", status)
	}
	return fmt.Errorf("leanmultisig: %s", message)
}

func bufferResult(call func(*C.lms_buffer) C.int32_t) ([]byte, error) {
	var buffer C.lms_buffer
	if err := checkStatus(call(&buffer)); err != nil {
		return nil, err
	}
	return takeBuffer(&buffer)
}

func takeBuffer(buffer *C.lms_buffer) ([]byte, error) {
	defer C.lms_buffer_free(buffer.data, buffer.len)
	if uint64(buffer.len) > uint64(1<<31-1) {
		return nil, errors.New("leanmultisig: native buffer is too large")
	}
	if buffer.len == 0 {
		return []byte{}, nil
	}
	return C.GoBytes(unsafe.Pointer(buffer.data), C.int(buffer.len)), nil
}

func bytesPointer(bytes []byte) *C.uint8_t {
	if len(bytes) == 0 {
		return nil
	}
	return (*C.uint8_t)(unsafe.Pointer(unsafe.SliceData(bytes)))
}

func flattenSigners(signers [][32]byte) []byte {
	result := make([]byte, len(signers)*32)
	for index := range signers {
		copy(result[index*32:], signers[index][:])
	}
	return result
}

func splitSigners(bytes []byte) ([][32]byte, error) {
	if len(bytes)%32 != 0 {
		return nil, errors.New("leanmultisig: native signer data is not a multiple of 32 bytes")
	}
	signers := make([][32]byte, len(bytes)/32)
	for index := range signers {
		copy(signers[index][:], bytes[index*32:])
	}
	return signers, nil
}

func signaturePointer(handles []*C.lms_signature) **C.lms_signature {
	if len(handles) == 0 {
		return nil
	}
	return (**C.lms_signature)(unsafe.Pointer(unsafe.SliceData(handles)))
}

func encodeGroups(groups []ClaimSigners) ([]byte, error) {
	if uint64(len(groups)) > math.MaxUint32 {
		return nil, errors.New("leanmultisig: too many claim groups")
	}
	length := uint64(9)
	for _, group := range groups {
		length += 40 + uint64(len(group.Signers))*32
		if length > math.MaxInt {
			return nil, errors.New("leanmultisig: claim context is too large")
		}
	}
	context := make([]byte, int(length))
	copy(context, "LMCG")
	context[4] = 1
	binary.LittleEndian.PutUint32(context[5:], uint32(len(groups)))
	cursor := 9
	for _, group := range groups {
		copy(context[cursor:], group.Claim.Message[:])
		cursor += 32
		binary.LittleEndian.PutUint32(context[cursor:], group.Claim.Slot)
		cursor += 4
		binary.LittleEndian.PutUint32(context[cursor:], uint32(len(group.Signers)))
		cursor += 4
		for _, signer := range group.Signers {
			copy(context[cursor:], signer[:])
			cursor += 32
		}
	}
	return context, nil
}

func decodeGroups(context []byte) ([]ClaimSigners, error) {
	if len(context) < 9 || string(context[:4]) != "LMCG" || context[4] != 1 {
		return nil, errors.New("leanmultisig: malformed native claim context")
	}
	groupCount := uint64(binary.LittleEndian.Uint32(context[5:]))
	if groupCount > uint64(math.MaxInt) {
		return nil, errors.New("leanmultisig: too many native claim groups")
	}
	groups := make([]ClaimSigners, 0, int(groupCount))
	cursor := 9
	for range groupCount {
		if len(context)-cursor < 40 {
			return nil, errors.New("leanmultisig: truncated native claim context")
		}
		var claim Claim
		copy(claim.Message[:], context[cursor:cursor+32])
		cursor += 32
		claim.Slot = binary.LittleEndian.Uint32(context[cursor:])
		cursor += 4
		signerCount := uint64(binary.LittleEndian.Uint32(context[cursor:]))
		cursor += 4
		if signerCount > uint64((len(context)-cursor)/32) {
			return nil, errors.New("leanmultisig: truncated native signer context")
		}
		signers := make([][32]byte, int(signerCount))
		for index := range signers {
			copy(signers[index][:], context[cursor:cursor+32])
			cursor += 32
		}
		groups = append(groups, ClaimSigners{Claim: claim, Signers: signers})
	}
	if cursor != len(context) {
		return nil, errors.New("leanmultisig: trailing native claim context bytes")
	}
	return groups, nil
}

func errClosed(kind string) error {
	return fmt.Errorf("leanmultisig: %s is closed", kind)
}
