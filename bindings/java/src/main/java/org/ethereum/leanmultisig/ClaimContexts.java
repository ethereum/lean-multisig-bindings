package org.ethereum.leanmultisig;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;

/** Private context format shared with the Java-specific native ABI. */
final class ClaimContexts {
    private static final int HEADER_LENGTH = 9;
    private static final int GROUP_FIXED_LENGTH = 40;
    private static final int KEY_LENGTH = 32;

    private ClaimContexts() {}

    static byte[] encode(List<ClaimSigners> groups) {
        if (groups == null) {
            throw new NullPointerException("groups");
        }
        int length = HEADER_LENGTH;
        for (ClaimSigners group : groups) {
            if (group == null) {
                throw new NullPointerException("group");
            }
            length = Math.addExact(length, Math.addExact(GROUP_FIXED_LENGTH, group.flattenedSigners().length));
        }
        ByteBuffer output = ByteBuffer.allocate(length).order(ByteOrder.LITTLE_ENDIAN);
        output.put((byte) 'L').put((byte) 'M').put((byte) 'C').put((byte) 'G').put((byte) 1);
        output.putInt(groups.size());
        for (ClaimSigners group : groups) {
            output.put(group.claim().messageInternal());
            output.putInt(group.claim().slotInternal());
            byte[] signers = group.flattenedSigners();
            output.putInt(signers.length / KEY_LENGTH);
            output.put(signers);
        }
        return output.array();
    }

    static List<ClaimSigners> decode(byte[] encoded) {
        if (encoded.length < HEADER_LENGTH) {
            throw new LeanMultisigException("malformed native claim context");
        }
        ByteBuffer input = ByteBuffer.wrap(encoded).order(ByteOrder.LITTLE_ENDIAN);
        if (input.get() != 'L' || input.get() != 'M' || input.get() != 'C' || input.get() != 'G' || input.get() != 1) {
            throw new LeanMultisigException("malformed native claim context");
        }
        long count = Integer.toUnsignedLong(input.getInt());
        if (count > Integer.MAX_VALUE) {
            throw new LeanMultisigException("too many native claim groups");
        }
        List<ClaimSigners> groups = new ArrayList<>((int) count);
        try {
            for (int group = 0; group < count; group++) {
                byte[] message = new byte[KEY_LENGTH];
                input.get(message);
                long slot = Integer.toUnsignedLong(input.getInt());
                long signerCount = Integer.toUnsignedLong(input.getInt());
                int signerLength = Math.toIntExact(Math.multiplyExact(signerCount, KEY_LENGTH));
                if (signerLength > input.remaining()) {
                    throw new LeanMultisigException("truncated native claim context");
                }
                List<byte[]> signers = new ArrayList<>((int) signerCount);
                for (int signer = 0; signer < signerCount; signer++) {
                    byte[] key = new byte[KEY_LENGTH];
                    input.get(key);
                    signers.add(key);
                }
                groups.add(new ClaimSigners(new Claim(message, slot), signers));
            }
        } catch (java.nio.BufferUnderflowException | ArithmeticException error) {
            throw new LeanMultisigException("malformed native claim context");
        }
        if (input.hasRemaining()) {
            throw new LeanMultisigException("trailing bytes in native claim context");
        }
        return List.copyOf(groups);
    }
}
