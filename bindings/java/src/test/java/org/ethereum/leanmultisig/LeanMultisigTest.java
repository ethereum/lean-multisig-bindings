package org.ethereum.leanmultisig;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

class LeanMultisigTest {
    @BeforeAll
    static void setUp() {
        LeanMultisig.setup();
    }

    @Test
    void signsAggregatesAndRestoresAClaim() {
        Claim claim = new Claim(bytes(42), 100);
        try (SecretKey alice = SecretKey.fromSeed(bytes(1), 100, 115);
             SecretKey bob = SecretKey.fromSeed(bytes(2), 100, 115);
             Signature aliceSignature = alice.sign(claim);
             Signature bobSignature = bob.sign(claim);
             Signature aggregate = LeanMultisig.aggregate(List.of(aliceSignature, bobSignature), claim);
             Signature restored = Signature.fromBytes(
                 aggregate.toBytes(), claim, List.of(alice.publicKey(), bob.publicKey()))) {
            assertTrue(LeanMultisig.verify(restored, List.of(alice.publicKey(), bob.publicKey()), claim));
            assertEquals(2, LeanMultisig.verifiedSigners(restored, claim).size());
        }
    }

    @Test
    void mergesClaimsAndPreservesTheirContexts() {
        Claim first = new Claim(bytes(3), 100);
        Claim second = new Claim(bytes(4), 101);
        try (SecretKey key = SecretKey.fromSeed(bytes(5), 100, 115);
             Signature firstSignature = key.sign(first);
             Signature secondSignature = key.sign(second);
             MultiClaimProof proof = LeanMultisig.mergeClaims(List.of(firstSignature, secondSignature));
             MultiClaimProof restored = MultiClaimProof.fromBytes(
                 proof.toBytes(),
                 List.of(new ClaimSigners(first, List.of(key.publicKey())), new ClaimSigners(second, List.of(key.publicKey()))))) {
            List<ClaimSigners> expected = List.of(
                new ClaimSigners(first, List.of(key.publicKey())),
                new ClaimSigners(second, List.of(key.publicKey())));
            assertTrue(LeanMultisig.verifyClaims(restored, expected));
            assertEquals(expected, LeanMultisig.verifiedClaims(restored));
        }
    }

    @Test
    void valuesDefensivelyCopyPublicBytes() {
        byte[] message = bytes(9);
        Claim claim = new Claim(message, 1);
        message[0] = 0;
        assertArrayEquals(bytes(9), claim.message());
    }

    private static byte[] bytes(int value) {
        byte[] bytes = new byte[32];
        java.util.Arrays.fill(bytes, (byte) value);
        return bytes;
    }
}
