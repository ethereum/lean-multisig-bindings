package leanmultisig

import (
	"reflect"
	"testing"
)

func TestSignAggregateAndRestore(t *testing.T) {
	if err := Setup(); err != nil {
		t.Fatal(err)
	}
	claim := Claim{Message: bytes32(42), Slot: 100}

	alice, err := SecretKeyFromSeed(bytes32(1), 100, 115)
	if err != nil {
		t.Fatal(err)
	}
	defer alice.Close()
	bob, err := SecretKeyFromSeed(bytes32(2), 100, 115)
	if err != nil {
		t.Fatal(err)
	}
	defer bob.Close()

	aliceSignature, err := alice.Sign(claim)
	if err != nil {
		t.Fatal(err)
	}
	defer aliceSignature.Close()
	bobSignature, err := bob.Sign(claim)
	if err != nil {
		t.Fatal(err)
	}
	defer bobSignature.Close()
	aggregate, err := Aggregate([]*Signature{aliceSignature, bobSignature}, claim)
	if err != nil {
		t.Fatal(err)
	}
	defer aggregate.Close()

	encoded, err := aggregate.Bytes()
	if err != nil {
		t.Fatal(err)
	}
	alicePublicKey := mustPublicKey(t, alice)
	bobPublicKey := mustPublicKey(t, bob)
	restored, err := SignatureFromBytes(encoded, claim, [][32]byte{alicePublicKey, bobPublicKey})
	if err != nil {
		t.Fatal(err)
	}
	defer restored.Close()

	valid, err := Verify(restored, [][32]byte{alicePublicKey, bobPublicKey}, claim)
	if err != nil {
		t.Fatal(err)
	}
	if !valid {
		t.Fatal("restored aggregate must verify")
	}
}

func TestMergeClaims(t *testing.T) {
	if err := Setup(); err != nil {
		t.Fatal(err)
	}
	first := Claim{Message: bytes32(3), Slot: 100}
	second := Claim{Message: bytes32(4), Slot: 101}
	key, err := SecretKeyFromSeed(bytes32(5), 100, 115)
	if err != nil {
		t.Fatal(err)
	}
	defer key.Close()
	firstSignature, err := key.Sign(first)
	if err != nil {
		t.Fatal(err)
	}
	defer firstSignature.Close()
	secondSignature, err := key.Sign(second)
	if err != nil {
		t.Fatal(err)
	}
	defer secondSignature.Close()
	proof, err := MergeClaims([]*Signature{firstSignature, secondSignature})
	if err != nil {
		t.Fatal(err)
	}
	defer proof.Close()

	publicKey := mustPublicKey(t, key)
	expected := []ClaimSigners{
		{Claim: first, Signers: [][32]byte{publicKey}},
		{Claim: second, Signers: [][32]byte{publicKey}},
	}
	valid, err := VerifyClaims(proof, expected)
	if err != nil {
		t.Fatal(err)
	}
	if !valid {
		t.Fatal("multi-claim proof must verify")
	}
	encoded, err := proof.Bytes()
	if err != nil {
		t.Fatal(err)
	}
	restored, err := MultiClaimProofFromBytes(encoded, expected)
	if err != nil {
		t.Fatal(err)
	}
	defer restored.Close()
	actual, err := VerifiedClaims(restored)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(expected, actual) {
		t.Fatalf("unexpected verified claims: %#v", actual)
	}
}

func mustPublicKey(t *testing.T, key *SecretKey) [32]byte {
	t.Helper()
	publicKey, err := key.PublicKey()
	if err != nil {
		t.Fatal(err)
	}
	return publicKey
}

func bytes32(value byte) [32]byte {
	var bytes [32]byte
	for index := range bytes {
		bytes[index] = value
	}
	return bytes
}
