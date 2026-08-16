package org.ethereum.leanmultisig;

import java.lang.foreign.MemorySegment;
import java.util.List;
import java.util.Objects;

/** A raw or recursively aggregated signature. */
public final class Signature implements AutoCloseable {
    private final NativeHandle handle;

    Signature(MemorySegment pointer) {
        handle = new NativeHandle(pointer, Native::destroySignature);
    }

    public static Signature fromBytes(byte[] bytes, Claim claim, List<byte[]> signers) {
        Objects.requireNonNull(bytes, "bytes");
        Objects.requireNonNull(claim, "claim");
        return new Signature(Native.signatureFromBytes(bytes.clone(), claim, signers));
    }

    public byte[] toBytes() {
        try (NativeHandle.Borrow borrowed = handle.borrow()) {
            return Native.signatureBytes(borrowed.pointer());
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
