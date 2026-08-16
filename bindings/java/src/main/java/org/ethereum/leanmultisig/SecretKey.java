package org.ethereum.leanmultisig;

import java.lang.foreign.MemorySegment;
import java.util.Objects;

/** An XMSS secret key. Close it promptly to release its native allocation. */
public final class SecretKey implements AutoCloseable {
    private final NativeHandle handle;

    private SecretKey(MemorySegment pointer) {
        handle = new NativeHandle(pointer, Native::destroySecretKey);
    }

    public static SecretKey generate(long slotStart, long slotEnd) {
        return new SecretKey(Native.generateSecretKey(slotStart, slotEnd));
    }

    public static SecretKey fromSeed(byte[] seed, long slotStart, long slotEnd) {
        Objects.requireNonNull(seed, "seed");
        if (seed.length != 32) {
            throw new IllegalArgumentException("seed must be exactly 32 bytes, got " + seed.length);
        }
        return new SecretKey(Native.secretKeyFromSeed(seed.clone(), slotStart, slotEnd));
    }

    public static SecretKey fromBytes(byte[] bytes) {
        return new SecretKey(Native.secretKeyFromBytes(Objects.requireNonNull(bytes, "bytes").clone()));
    }

    public byte[] toBytes() {
        try (NativeHandle.Borrow borrowed = handle.borrow()) {
            return Native.secretKeyBytes(borrowed.pointer());
        }
    }

    public byte[] publicKey() {
        try (NativeHandle.Borrow borrowed = handle.borrow()) {
            return Native.publicKey(borrowed.pointer());
        }
    }

    public long slotStart() {
        try (NativeHandle.Borrow borrowed = handle.borrow()) {
            return Native.slots(borrowed.pointer())[0];
        }
    }

    public long slotEnd() {
        try (NativeHandle.Borrow borrowed = handle.borrow()) {
            return Native.slots(borrowed.pointer())[1];
        }
    }

    public void prepare(long slot) {
        try (NativeHandle.Borrow borrowed = handle.borrow()) {
            Native.prepare(borrowed.pointer(), slot);
        }
    }

    public Signature sign(Claim claim) {
        Objects.requireNonNull(claim, "claim");
        try (NativeHandle.Borrow borrowed = handle.borrow()) {
            return new Signature(Native.sign(borrowed.pointer(), claim));
        }
    }

    @Override
    public void close() {
        handle.close();
    }
}
