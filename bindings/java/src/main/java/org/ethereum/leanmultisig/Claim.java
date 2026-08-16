package org.ethereum.leanmultisig;

import java.util.Arrays;
import java.util.Objects;

/** A 32-byte message and its unsigned 32-bit slot. */
public final class Claim {
    private static final int MESSAGE_LENGTH = 32;

    private final byte[] message;
    private final long slot;

    public Claim(byte[] message, long slot) {
        Objects.requireNonNull(message, "message");
        if (message.length != MESSAGE_LENGTH) {
            throw new IllegalArgumentException("message must be exactly 32 bytes, got " + message.length);
        }
        if (slot < 0 || slot > 0xffff_ffffL) {
            throw new IllegalArgumentException("slot must fit an unsigned 32-bit integer");
        }
        this.message = message.clone();
        this.slot = slot;
    }

    /** Returns a defensive copy of the 32-byte message. */
    public byte[] message() {
        return message.clone();
    }

    /** Returns the slot as a non-negative Java {@code long}. */
    public long slot() {
        return slot;
    }

    byte[] messageInternal() {
        return message;
    }

    int slotInternal() {
        return (int) slot;
    }

    @Override
    public boolean equals(Object other) {
        return other instanceof Claim claim && slot == claim.slot && Arrays.equals(message, claim.message);
    }

    @Override
    public int hashCode() {
        return 31 * Long.hashCode(slot) + Arrays.hashCode(message);
    }

    @Override
    public String toString() {
        return "Claim[slot=" + slot + "]";
    }
}
