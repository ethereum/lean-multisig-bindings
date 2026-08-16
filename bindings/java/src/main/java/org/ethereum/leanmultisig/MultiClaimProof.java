package org.ethereum.leanmultisig;

import java.lang.foreign.MemorySegment;
import java.util.List;
import java.util.Objects;

/** A proof binding multiple claims to their signer sets. */
public final class MultiClaimProof implements AutoCloseable {
    private final NativeHandle handle;

    MultiClaimProof(MemorySegment pointer) {
        handle = new NativeHandle(pointer, Native::destroyProof);
    }

    public static MultiClaimProof fromBytes(byte[] bytes, List<ClaimSigners> expected) {
        Objects.requireNonNull(bytes, "bytes");
        return new MultiClaimProof(Native.proofFromBytes(bytes.clone(), ClaimContexts.encode(expected)));
    }

    public byte[] toBytes() {
        try (NativeHandle.Borrow borrowed = handle.borrow()) {
            return Native.proofBytes(borrowed.pointer());
        }
    }

    NativeHandle.Borrow borrow() {
        return handle.borrow();
    }

    @Override
    public void close() {
        handle.close();
    }
}
