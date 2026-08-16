package org.ethereum.leanmultisig;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;

/** The exact public-key set authorized for one claim. */
public final class ClaimSigners {
    private static final int PUBLIC_KEY_LENGTH = 32;

    private final Claim claim;
    private final List<byte[]> signers;

    public ClaimSigners(Claim claim, List<byte[]> signers) {
        this.claim = Objects.requireNonNull(claim, "claim");
        Objects.requireNonNull(signers, "signers");
        List<byte[]> copied = new ArrayList<>(signers.size());
        for (byte[] signer : signers) {
            Objects.requireNonNull(signer, "signer");
            if (signer.length != PUBLIC_KEY_LENGTH) {
                throw new IllegalArgumentException("public key must be exactly 32 bytes, got " + signer.length);
            }
            copied.add(signer.clone());
        }
        this.signers = List.copyOf(copied);
    }

    public Claim claim() {
        return claim;
    }

    /** Returns defensive copies of the public keys. */
    public List<byte[]> signers() {
        return signers.stream().map(byte[]::clone).toList();
    }

    byte[] flattenedSigners() {
        byte[] result = new byte[Math.multiplyExact(signers.size(), PUBLIC_KEY_LENGTH)];
        for (int index = 0; index < signers.size(); index++) {
            System.arraycopy(signers.get(index), 0, result, index * PUBLIC_KEY_LENGTH, PUBLIC_KEY_LENGTH);
        }
        return result;
    }

    @Override
    public boolean equals(Object other) {
        if (!(other instanceof ClaimSigners group) || !claim.equals(group.claim) || signers.size() != group.signers.size()) {
            return false;
        }
        for (int index = 0; index < signers.size(); index++) {
            if (!Arrays.equals(signers.get(index), group.signers.get(index))) {
                return false;
            }
        }
        return true;
    }

    @Override
    public int hashCode() {
        int result = claim.hashCode();
        for (byte[] signer : signers) {
            result = 31 * result + Arrays.hashCode(signer);
        }
        return result;
    }
}
