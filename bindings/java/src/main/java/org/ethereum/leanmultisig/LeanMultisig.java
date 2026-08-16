package org.ethereum.leanmultisig;

import java.lang.foreign.MemorySegment;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

/** Entry points for aggregation and verification. Call {@link #setup()} before recursive proofs. */
public final class LeanMultisig {
    private LeanMultisig() {}

    /** Initializes the process-wide recursive-proof resources. Safe to call repeatedly. */
    public static void setup() {
        Native.setup();
    }

    public static Signature aggregate(List<Signature> signatures, Claim claim) {
        Objects.requireNonNull(claim, "claim");
        List<NativeHandle.Borrow> borrowed = borrowSignatures(signatures);
        try {
            return new Signature(Native.aggregate(pointers(borrowed), claim));
        } finally {
            closeAll(borrowed);
        }
    }

    /** Returns the canonical signer set after verifying the signature. */
    public static List<byte[]> verifiedSigners(Signature signature, Claim claim) {
        Objects.requireNonNull(signature, "signature");
        Objects.requireNonNull(claim, "claim");
        try (NativeHandle.Borrow borrowed = signature.borrow()) {
            return Native.verifiedSigners(borrowed.pointer(), claim).stream().map(byte[]::clone).toList();
        }
    }

    /** Returns whether the signature proves exactly the supplied public-key set for the claim. */
    public static boolean verify(Signature signature, List<byte[]> expected, Claim claim) {
        Objects.requireNonNull(signature, "signature");
        Objects.requireNonNull(claim, "claim");
        try (NativeHandle.Borrow borrowed = signature.borrow()) {
            Native.verify(borrowed.pointer(), expected, claim);
            return true;
        } catch (LeanMultisigException rejected) {
            return false;
        }
    }

    public static MultiClaimProof mergeClaims(List<Signature> signatures) {
        List<NativeHandle.Borrow> borrowed = borrowSignatures(signatures);
        try {
            return new MultiClaimProof(Native.mergeClaims(pointers(borrowed)));
        } finally {
            closeAll(borrowed);
        }
    }

    /** Returns canonical claim/signer groups after verifying the proof. */
    public static List<ClaimSigners> verifiedClaims(MultiClaimProof proof) {
        Objects.requireNonNull(proof, "proof");
        try (NativeHandle.Borrow borrowed = proof.borrow()) {
            return ClaimContexts.decode(Native.verifiedClaims(borrowed.pointer()));
        }
    }

    /** Returns whether the proof proves exactly the supplied claim/signer groups. */
    public static boolean verifyClaims(MultiClaimProof proof, List<ClaimSigners> expected) {
        Objects.requireNonNull(proof, "proof");
        byte[] context = ClaimContexts.encode(expected);
        try (NativeHandle.Borrow borrowed = proof.borrow()) {
            Native.verifyClaims(borrowed.pointer(), context);
            return true;
        } catch (LeanMultisigException rejected) {
            return false;
        }
    }

    private static List<NativeHandle.Borrow> borrowSignatures(List<Signature> signatures) {
        Objects.requireNonNull(signatures, "signatures");
        List<NativeHandle.Borrow> borrowed = new ArrayList<>(signatures.size());
        try {
            for (Signature signature : signatures) {
                borrowed.add(Objects.requireNonNull(signature, "signature").borrow());
            }
            return borrowed;
        } catch (RuntimeException error) {
            closeAll(borrowed);
            throw error;
        }
    }

    private static List<MemorySegment> pointers(List<NativeHandle.Borrow> borrowed) {
        return borrowed.stream().map(NativeHandle.Borrow::pointer).toList();
    }

    private static void closeAll(List<NativeHandle.Borrow> borrowed) {
        for (int index = borrowed.size() - 1; index >= 0; index--) {
            borrowed.get(index).close();
        }
    }
}
