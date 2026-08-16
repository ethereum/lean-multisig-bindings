package org.ethereum.leanmultisig;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemoryLayout;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/** All FFM interaction is confined here; public binding classes never expose native addresses. */
final class Native {
    private static final int OK = 0;
    private static final long U32_MAX = 0xffff_ffffL;
    private static final MemoryLayout BUFFER = MemoryLayout.structLayout(ValueLayout.ADDRESS, ValueLayout.JAVA_LONG);
    private static final Linker LINKER = Linker.nativeLinker();
    private static final Arena LIBRARY_ARENA = Arena.ofShared();
    private static final SymbolLookup LIBRARY = SymbolLookup.libraryLookup(nativePath(), LIBRARY_ARENA);

    private static final MethodHandle SETUP = downcall("lms_setup", FunctionDescriptor.of(ValueLayout.JAVA_INT));
    private static final MethodHandle LAST_ERROR = downcall("lms_last_error", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS));
    private static final MethodHandle BUFFER_FREE = downcall("lms_buffer_free", FunctionDescriptor.ofVoid(ValueLayout.ADDRESS, ValueLayout.JAVA_LONG));

    private static final MethodHandle KEY_GENERATE = downcall("lms_secret_key_generate", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.JAVA_INT, ValueLayout.JAVA_INT, ValueLayout.ADDRESS));
    private static final MethodHandle KEY_FROM_SEED = downcall("lms_secret_key_from_seed", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.JAVA_INT, ValueLayout.JAVA_INT, ValueLayout.ADDRESS));
    private static final MethodHandle KEY_FROM_BYTES = downcall("lms_secret_key_from_bytes", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS));
    private static final MethodHandle KEY_TO_BYTES = downcall("lms_secret_key_to_bytes", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
    private static final MethodHandle KEY_PUBLIC_KEY = downcall("lms_secret_key_public_key", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
    private static final MethodHandle KEY_SLOTS = downcall("lms_secret_key_slots", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
    private static final MethodHandle KEY_PREPARE = downcall("lms_secret_key_prepare", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.JAVA_INT));
    private static final MethodHandle KEY_SIGN = downcall("lms_secret_key_sign", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.JAVA_INT, ValueLayout.ADDRESS));
    private static final MethodHandle KEY_DESTROY = downcall("lms_secret_key_destroy", FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));

    private static final MethodHandle SIGNATURE_FROM_BYTES = downcall("lms_signature_from_bytes", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS));
    private static final MethodHandle SIGNATURE_TO_BYTES = downcall("lms_signature_to_bytes", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
    private static final MethodHandle SIGNATURE_AGGREGATE = downcall("lms_signature_aggregate", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.JAVA_INT, ValueLayout.ADDRESS));
    private static final MethodHandle SIGNATURE_VERIFIED_SIGNERS = downcall("lms_signature_verified_signers", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.JAVA_INT, ValueLayout.ADDRESS));
    private static final MethodHandle SIGNATURE_VERIFY = downcall("lms_signature_verify", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.JAVA_INT));
    private static final MethodHandle SIGNATURE_DESTROY = downcall("lms_signature_destroy", FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));

    private static final MethodHandle PROOF_MERGE = downcall("lms_multi_claim_proof_merge", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS));
    private static final MethodHandle PROOF_FROM_BYTES = downcall("lms_multi_claim_proof_from_bytes", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS));
    private static final MethodHandle PROOF_TO_BYTES = downcall("lms_multi_claim_proof_to_bytes", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
    private static final MethodHandle PROOF_VERIFIED_CLAIMS = downcall("lms_multi_claim_proof_verified_claims", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
    private static final MethodHandle PROOF_VERIFY = downcall("lms_multi_claim_proof_verify", FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG));
    private static final MethodHandle PROOF_DESTROY = downcall("lms_multi_claim_proof_destroy", FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));

    private Native() {}

    static void setup() {
        check(status(SETUP));
    }

    static MemorySegment generateSecretKey(long slotStart, long slotEnd) {
        return handle(KEY_GENERATE, u32(slotStart, "slotStart"), u32(slotEnd, "slotEnd"));
    }

    static MemorySegment secretKeyFromSeed(byte[] seed, long slotStart, long slotEnd) {
        try (Arena arena = Arena.ofConfined()) {
            return handle(KEY_FROM_SEED, copy(arena, requireBytes(seed, "seed")), (long) seed.length, u32(slotStart, "slotStart"), u32(slotEnd, "slotEnd"));
        }
    }

    static MemorySegment secretKeyFromBytes(byte[] bytes) {
        try (Arena arena = Arena.ofConfined()) {
            return handle(KEY_FROM_BYTES, copy(arena, requireBytes(bytes, "secret key")), (long) bytes.length);
        }
    }

    static byte[] secretKeyBytes(MemorySegment key) {
        return buffer(KEY_TO_BYTES, key);
    }

    static byte[] publicKey(MemorySegment key) {
        return buffer(KEY_PUBLIC_KEY, key);
    }

    static long[] slots(MemorySegment key) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment start = arena.allocate(ValueLayout.JAVA_INT);
            MemorySegment end = arena.allocate(ValueLayout.JAVA_INT);
            check(status(KEY_SLOTS, key, start, end));
            return new long[] {Integer.toUnsignedLong(start.get(ValueLayout.JAVA_INT, 0)), Integer.toUnsignedLong(end.get(ValueLayout.JAVA_INT, 0))};
        }
    }

    static void prepare(MemorySegment key, long slot) {
        check(status(KEY_PREPARE, key, u32(slot, "slot")));
    }

    static MemorySegment sign(MemorySegment key, Claim claim) {
        try (Arena arena = Arena.ofConfined()) {
            byte[] message = claim.messageInternal();
            return handle(KEY_SIGN, key, copy(arena, message), (long) message.length, claim.slotInternal());
        }
    }

    static MemorySegment signatureFromBytes(byte[] bytes, Claim claim, List<byte[]> signers) {
        try (Arena arena = Arena.ofConfined()) {
            byte[] message = claim.messageInternal();
            byte[] flattened = flattenKeys(signers);
            return handle(SIGNATURE_FROM_BYTES,
                copy(arena, requireBytes(bytes, "signature")), (long) bytes.length,
                copy(arena, message), (long) message.length, claim.slotInternal(),
                copy(arena, flattened), (long) signers.size());
        }
    }

    static byte[] signatureBytes(MemorySegment signature) {
        return buffer(SIGNATURE_TO_BYTES, signature);
    }

    static MemorySegment aggregate(List<MemorySegment> signatures, Claim claim) {
        try (Arena arena = Arena.ofConfined()) {
            byte[] message = claim.messageInternal();
            return handle(SIGNATURE_AGGREGATE, pointerArray(arena, signatures), (long) signatures.size(), copy(arena, message), (long) message.length, claim.slotInternal());
        }
    }

    static List<byte[]> verifiedSigners(MemorySegment signature, Claim claim) {
        try (Arena arena = Arena.ofConfined()) {
            byte[] message = claim.messageInternal();
            return splitKeys(buffer(SIGNATURE_VERIFIED_SIGNERS, signature, copy(arena, message), (long) message.length, claim.slotInternal()));
        }
    }

    static void verify(MemorySegment signature, List<byte[]> signers, Claim claim) {
        try (Arena arena = Arena.ofConfined()) {
            byte[] message = claim.messageInternal();
            byte[] flattened = flattenKeys(signers);
            check(status(SIGNATURE_VERIFY, signature, copy(arena, flattened), (long) signers.size(), copy(arena, message), (long) message.length, claim.slotInternal()));
        }
    }

    static MemorySegment mergeClaims(List<MemorySegment> signatures) {
        try (Arena arena = Arena.ofConfined()) {
            return handle(PROOF_MERGE, pointerArray(arena, signatures), (long) signatures.size());
        }
    }

    static MemorySegment proofFromBytes(byte[] bytes, byte[] context) {
        try (Arena arena = Arena.ofConfined()) {
            return handle(PROOF_FROM_BYTES, copy(arena, requireBytes(bytes, "multi-claim proof")), (long) bytes.length, copy(arena, requireBytes(context, "claim context")), (long) context.length);
        }
    }

    static byte[] proofBytes(MemorySegment proof) {
        return buffer(PROOF_TO_BYTES, proof);
    }

    static byte[] verifiedClaims(MemorySegment proof) {
        return buffer(PROOF_VERIFIED_CLAIMS, proof);
    }

    static void verifyClaims(MemorySegment proof, byte[] context) {
        try (Arena arena = Arena.ofConfined()) {
            check(status(PROOF_VERIFY, proof, copy(arena, requireBytes(context, "claim context")), (long) context.length));
        }
    }

    static void destroySecretKey(MemorySegment pointer) {
        invoke(KEY_DESTROY, pointer);
    }

    static void destroySignature(MemorySegment pointer) {
        invoke(SIGNATURE_DESTROY, pointer);
    }

    static void destroyProof(MemorySegment pointer) {
        invoke(PROOF_DESTROY, pointer);
    }

    private static Path nativePath() {
        String path = System.getProperty("lean.multisig.native.path");
        if (path == null || path.isBlank()) {
            throw new IllegalStateException("set -Dlean.multisig.native.path to the leanMultisig native library");
        }
        return Path.of(path).toAbsolutePath();
    }

    private static MethodHandle downcall(String name, FunctionDescriptor descriptor) {
        MemorySegment symbol = LIBRARY.find(name).orElseThrow(() -> new IllegalStateException("missing native symbol " + name));
        return LINKER.downcallHandle(symbol, descriptor);
    }

    private static MemorySegment handle(MethodHandle function, Object... arguments) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment out = arena.allocate(ValueLayout.ADDRESS);
            Object[] all = Arrays.copyOf(arguments, arguments.length + 1);
            all[arguments.length] = out;
            check(status(function, all));
            MemorySegment result = out.get(ValueLayout.ADDRESS, 0);
            if (result.address() == 0) {
                throw new LeanMultisigException("native operation returned a null handle");
            }
            return MemorySegment.ofAddress(result.address());
        }
    }

    private static byte[] buffer(MethodHandle function, Object... arguments) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment out = arena.allocate(BUFFER);
            Object[] all = Arrays.copyOf(arguments, arguments.length + 1);
            all[arguments.length] = out;
            check(status(function, all));
            MemorySegment data = out.get(ValueLayout.ADDRESS, 0);
            long length = out.get(ValueLayout.JAVA_LONG, ValueLayout.ADDRESS.byteSize());
            try {
                if (length < 0 || length > Integer.MAX_VALUE) {
                    throw new LeanMultisigException("native buffer length is invalid: " + length);
                }
                return length == 0 ? new byte[0] : data.reinterpret(length).toArray(ValueLayout.JAVA_BYTE);
            } finally {
                invoke(BUFFER_FREE, data, length);
            }
        }
    }

    private static int status(MethodHandle function, Object... arguments) {
        return (int) invoke(function, arguments);
    }

    private static Object invoke(MethodHandle function, Object... arguments) {
        try {
            return function.invokeWithArguments(arguments);
        } catch (RuntimeException error) {
            throw error;
        } catch (Throwable error) {
            throw new IllegalStateException("FFM invocation failed", error);
        }
    }

    private static void check(int status) {
        if (status != OK) {
            throw new LeanMultisigException(errorMessage());
        }
    }

    private static String errorMessage() {
        byte[] bytes = buffer(LAST_ERROR);
        String message = new String(bytes, java.nio.charset.StandardCharsets.UTF_8);
        return message.isBlank() ? "native lean-multisig operation failed" : message;
    }

    private static MemorySegment copy(Arena arena, byte[] bytes) {
        if (bytes.length == 0) {
            return MemorySegment.NULL;
        }
        MemorySegment copy = arena.allocate(bytes.length, 1);
        copy.asByteBuffer().put(bytes);
        return copy;
    }

    private static MemorySegment pointerArray(Arena arena, List<MemorySegment> pointers) {
        if (pointers.isEmpty()) {
            return MemorySegment.NULL;
        }
        MemorySegment result = arena.allocate(Math.multiplyExact(pointers.size(), ValueLayout.ADDRESS.byteSize()), ValueLayout.ADDRESS.byteAlignment());
        for (int index = 0; index < pointers.size(); index++) {
            result.set(ValueLayout.ADDRESS, index * ValueLayout.ADDRESS.byteSize(), pointers.get(index));
        }
        return result;
    }

    private static int u32(long value, String name) {
        if (value < 0 || value > U32_MAX) {
            throw new IllegalArgumentException(name + " must fit an unsigned 32-bit integer");
        }
        return (int) value;
    }

    private static byte[] requireBytes(byte[] bytes, String name) {
        if (bytes == null) {
            throw new NullPointerException(name);
        }
        return bytes;
    }

    private static byte[] flattenKeys(List<byte[]> keys) {
        if (keys == null) {
            throw new NullPointerException("signers");
        }
        byte[] result = new byte[Math.multiplyExact(keys.size(), 32)];
        for (int index = 0; index < keys.size(); index++) {
            byte[] key = keys.get(index);
            if (key == null || key.length != 32) {
                throw new IllegalArgumentException("every public key must be exactly 32 bytes");
            }
            System.arraycopy(key, 0, result, index * 32, 32);
        }
        return result;
    }

    private static List<byte[]> splitKeys(byte[] flattened) {
        if (flattened.length % 32 != 0) {
            throw new LeanMultisigException("native signer data is not a multiple of 32 bytes");
        }
        List<byte[]> result = new ArrayList<>(flattened.length / 32);
        for (int offset = 0; offset < flattened.length; offset += 32) {
            result.add(Arrays.copyOfRange(flattened, offset, offset + 32));
        }
        return List.copyOf(result);
    }
}
