//! Safe Zig bindings for leanMultisig's XMSS signatures and recursive aggregation.
//!
//! Native objects own opaque Rust handles. Call `deinit` exactly once on a
//! `SecretKey`, `Signature`, or `MultiClaimProof`; call `deinit` on a
//! `ClaimGroups` value returned by `MultiClaimProof.verifiedClaims`.
const std = @import("std");

const c = @cImport({
    @cInclude("lean_multisig_native.h");
});

pub const PublicKey = [32]u8;

pub const Claim = struct {
    message: [32]u8,
    slot: u32,
};

pub const ClaimSigners = struct {
    claim: Claim,
    signers: []const PublicKey,
};

/// Owns groups returned by `MultiClaimProof.verifiedClaims`.
pub const ClaimGroups = struct {
    items: []ClaimSigners,

    pub fn deinit(self: *ClaimGroups, allocator: std.mem.Allocator) void {
        for (self.items) |group| allocator.free(group.signers);
        allocator.free(self.items);
        self.* = undefined;
    }
};

/// Initializes process-wide recursive-proof resources. It is safe to call repeatedly.
pub fn setup() !void {
    try checkStatus(c.lms_setup());
}

/// Copies the native error text for the calling thread, if one is available.
pub fn lastError(allocator: std.mem.Allocator) ![]u8 {
    var output: c.lms_buffer = .{ .data = null, .len = 0 };
    try checkStatus(c.lms_last_error(&output));
    return takeBuffer(allocator, output);
}

pub const SecretKey = struct {
    handle: ?*c.lms_secret_key,

    pub fn generate(slot_start: u32, slot_end: u32) !SecretKey {
        var output: ?*c.lms_secret_key = null;
        try checkStatus(c.lms_secret_key_generate(slot_start, slot_end, &output));
        return .{ .handle = output orelse return error.NativeFailure };
    }

    pub fn fromSeed(seed: [32]u8, slot_start: u32, slot_end: u32) !SecretKey {
        var output: ?*c.lms_secret_key = null;
        try checkStatus(c.lms_secret_key_from_seed(&seed, seed.len, slot_start, slot_end, &output));
        return .{ .handle = output orelse return error.NativeFailure };
    }

    pub fn fromBytes(data: []const u8) !SecretKey {
        var output: ?*c.lms_secret_key = null;
        try checkStatus(c.lms_secret_key_from_bytes(bytesPtr(data), data.len, &output));
        return .{ .handle = output orelse return error.NativeFailure };
    }

    pub fn deinit(self: *SecretKey) void {
        if (self.handle) |handle| c.lms_secret_key_destroy(handle);
        self.handle = null;
    }

    pub fn toBytes(self: *const SecretKey, allocator: std.mem.Allocator) ![]u8 {
        var output: c.lms_buffer = .{ .data = null, .len = 0 };
        try checkStatus(c.lms_secret_key_to_bytes(try self.require(), &output));
        return takeBuffer(allocator, output);
    }

    pub fn publicKey(self: *const SecretKey, allocator: std.mem.Allocator) !PublicKey {
        var output: c.lms_buffer = .{ .data = null, .len = 0 };
        try checkStatus(c.lms_secret_key_public_key(try self.require(), &output));
        const data = try takeBuffer(allocator, output);
        defer allocator.free(data);
        if (data.len != 32) return error.InvalidNativeEncoding;
        var key: PublicKey = undefined;
        @memcpy(&key, data);
        return key;
    }

    pub fn slots(self: *const SecretKey) !struct { start: u32, end: u32 } {
        var start: u32 = undefined;
        var end: u32 = undefined;
        try checkStatus(c.lms_secret_key_slots(try self.require(), &start, &end));
        return .{ .start = start, .end = end };
    }

    pub fn prepare(self: *const SecretKey, slot: u32) !void {
        try checkStatus(c.lms_secret_key_prepare(try self.require(), slot));
    }

    pub fn sign(self: *const SecretKey, claim: Claim) !Signature {
        var output: ?*c.lms_signature = null;
        try checkStatus(c.lms_secret_key_sign(
            try self.require(),
            &claim.message,
            claim.message.len,
            claim.slot,
            &output,
        ));
        return .{ .handle = output orelse return error.NativeFailure };
    }

    fn require(self: *const SecretKey) !*c.lms_secret_key {
        return self.handle orelse error.Closed;
    }

};

pub const Signature = struct {
    handle: ?*c.lms_signature,

    pub fn fromBytes(data: []const u8, claim: Claim, signers: []const PublicKey) !Signature {
        var output: ?*c.lms_signature = null;
        try checkStatus(c.lms_signature_from_bytes(
            bytesPtr(data),
            data.len,
            &claim.message,
            claim.message.len,
            claim.slot,
            keysPtr(signers),
            signers.len,
            &output,
        ));
        return .{ .handle = output orelse return error.NativeFailure };
    }

    pub fn deinit(self: *Signature) void {
        if (self.handle) |handle| c.lms_signature_destroy(handle);
        self.handle = null;
    }

    /// Returns the cryptographic encoding, which excludes claim/signer context.
    pub fn toBytes(self: *const Signature, allocator: std.mem.Allocator) ![]u8 {
        var output: c.lms_buffer = .{ .data = null, .len = 0 };
        try checkStatus(c.lms_signature_to_bytes(try self.require(), &output));
        return takeBuffer(allocator, output);
    }

    pub fn verifiedSigners(self: *const Signature, allocator: std.mem.Allocator, claim: Claim) ![]PublicKey {
        var output: c.lms_buffer = .{ .data = null, .len = 0 };
        try checkStatus(c.lms_signature_verified_signers(
            try self.require(),
            &claim.message,
            claim.message.len,
            claim.slot,
            &output,
        ));
        const encoded = try takeBuffer(allocator, output);
        defer allocator.free(encoded);
        return splitKeys(allocator, encoded);
    }

    /// A false result is a normal verification failure. Other native failures are errors.
    pub fn verify(self: *const Signature, signers: []const PublicKey, claim: Claim) !bool {
        return verificationStatus(c.lms_signature_verify(
            try self.require(),
            keysPtr(signers),
            signers.len,
            &claim.message,
            claim.message.len,
            claim.slot,
        ));
    }

    fn require(self: *const Signature) !*c.lms_signature {
        return self.handle orelse error.Closed;
    }
};

/// Combines raw or aggregated signatures for one claim. Call `setup` first.
pub fn aggregate(allocator: std.mem.Allocator, signatures: []const *const Signature, claim: Claim) !Signature {
    if (signatures.len == 0) return error.EmptySignatures;
    const handles = try allocator.alloc(?*c.lms_signature, signatures.len);
    defer allocator.free(handles);
    for (signatures, 0..) |signature, index| handles[index] = try signature.require();

    var output: ?*c.lms_signature = null;
    try checkStatus(c.lms_signature_aggregate(
        handles.ptr,
        handles.len,
        &claim.message,
        claim.message.len,
        claim.slot,
        &output,
    ));
    return .{ .handle = output orelse return error.NativeFailure };
}

pub const MultiClaimProof = struct {
    handle: ?*c.lms_multi_claim_proof,

    pub fn fromBytes(allocator: std.mem.Allocator, data: []const u8, groups: []const ClaimSigners) !MultiClaimProof {
        const context = try encodeGroups(allocator, groups);
        defer allocator.free(context);
        var output: ?*c.lms_multi_claim_proof = null;
        try checkStatus(c.lms_multi_claim_proof_from_bytes(bytesPtr(data), data.len, bytesPtr(context), context.len, &output));
        return .{ .handle = output orelse return error.NativeFailure };
    }

    pub fn deinit(self: *MultiClaimProof) void {
        if (self.handle) |handle| c.lms_multi_claim_proof_destroy(handle);
        self.handle = null;
    }

    pub fn toBytes(self: *const MultiClaimProof, allocator: std.mem.Allocator) ![]u8 {
        var output: c.lms_buffer = .{ .data = null, .len = 0 };
        try checkStatus(c.lms_multi_claim_proof_to_bytes(try self.require(), &output));
        return takeBuffer(allocator, output);
    }

    pub fn verifiedClaims(self: *const MultiClaimProof, allocator: std.mem.Allocator) !ClaimGroups {
        var output: c.lms_buffer = .{ .data = null, .len = 0 };
        try checkStatus(c.lms_multi_claim_proof_verified_claims(try self.require(), &output));
        const encoded = try takeBuffer(allocator, output);
        defer allocator.free(encoded);
        return decodeGroups(allocator, encoded);
    }

    /// A false result is a normal verification failure. Other native failures are errors.
    pub fn verifyClaims(self: *const MultiClaimProof, allocator: std.mem.Allocator, groups: []const ClaimSigners) !bool {
        const context = try encodeGroups(allocator, groups);
        defer allocator.free(context);
        return verificationStatus(c.lms_multi_claim_proof_verify(try self.require(), bytesPtr(context), context.len));
    }

    fn require(self: *const MultiClaimProof) !*c.lms_multi_claim_proof {
        return self.handle orelse error.Closed;
    }
};

/// Creates one proof covering the supplied signatures. Call `setup` first.
pub fn mergeClaims(allocator: std.mem.Allocator, signatures: []const *const Signature) !MultiClaimProof {
    if (signatures.len == 0) return error.EmptySignatures;
    const handles = try allocator.alloc(?*c.lms_signature, signatures.len);
    defer allocator.free(handles);
    for (signatures, 0..) |signature, index| handles[index] = try signature.require();

    var output: ?*c.lms_multi_claim_proof = null;
    try checkStatus(c.lms_multi_claim_proof_merge(handles.ptr, handles.len, &output));
    return .{ .handle = output orelse return error.NativeFailure };
}

fn checkStatus(status: c.int32_t) !void {
    return switch (status) {
        0 => {},
        1 => error.NativeFailure,
        2 => error.NativePanic,
        else => error.InvalidNativeStatus,
    };
}

fn verificationStatus(status: c.int32_t) !bool {
    return switch (status) {
        0 => true,
        1 => false,
        2 => error.NativePanic,
        else => error.InvalidNativeStatus,
    };
}

fn takeBuffer(allocator: std.mem.Allocator, buffer: c.lms_buffer) ![]u8 {
    const data = buffer.data orelse {
        if (buffer.len == 0) return allocator.alloc(u8, 0);
        return error.InvalidNativeEncoding;
    };
    defer c.lms_buffer_free(data, buffer.len);
    return allocator.dupe(u8, data[0..buffer.len]);
}

fn bytesPtr(data: []const u8) ?[*c]const u8 {
    return if (data.len == 0) null else data.ptr;
}

fn keysPtr(keys: []const PublicKey) ?[*c]const u8 {
    return if (keys.len == 0) null else @ptrCast(keys.ptr);
}

fn splitKeys(allocator: std.mem.Allocator, data: []const u8) ![]PublicKey {
    if (data.len % 32 != 0) return error.InvalidNativeEncoding;
    const keys = try allocator.alloc(PublicKey, data.len / 32);
    for (keys, 0..) |*key, index| @memcpy(key, data[index * 32 ..][0..32]);
    return keys;
}

fn encodeGroups(allocator: std.mem.Allocator, groups: []const ClaimSigners) ![]u8 {
    if (groups.len > std.math.maxInt(u32)) return error.ContextTooLarge;
    var len: usize = 9;
    for (groups) |group| {
        if (group.signers.len > std.math.maxInt(u32)) return error.ContextTooLarge;
        const signer_len = std.math.mul(usize, group.signers.len, 32) catch return error.ContextTooLarge;
        len = std.math.add(usize, len, 40 + signer_len) catch return error.ContextTooLarge;
    }

    const result = try allocator.alloc(u8, len);
    errdefer allocator.free(result);
    @memcpy(result[0..4], "LMCG");
    result[4] = 1;
    std.mem.writeInt(u32, result[5..9], @intCast(groups.len), .little);
    var cursor: usize = 9;
    for (groups) |group| {
        @memcpy(result[cursor .. cursor + 32], &group.claim.message);
        cursor += 32;
        std.mem.writeInt(u32, result[cursor .. cursor + 4], group.claim.slot, .little);
        cursor += 4;
        std.mem.writeInt(u32, result[cursor .. cursor + 4], @intCast(group.signers.len), .little);
        cursor += 4;
        for (group.signers) |signer| {
            @memcpy(result[cursor .. cursor + 32], &signer);
            cursor += 32;
        }
    }
    return result;
}

fn decodeGroups(allocator: std.mem.Allocator, data: []const u8) !ClaimGroups {
    if (data.len < 9 or !std.mem.eql(u8, data[0..4], "LMCG") or data[4] != 1) return error.InvalidNativeEncoding;
    var cursor: usize = 5;
    const count = try takeU32(data, &cursor);
    const groups = try allocator.alloc(ClaimSigners, count);
    var initialized: usize = 0;
    errdefer {
        for (groups[0..initialized]) |group| allocator.free(group.signers);
        allocator.free(groups);
    }
    for (groups) |*group| {
        const message = try take32(data, &cursor);
        const slot = try takeU32(data, &cursor);
        const signer_count = try takeU32(data, &cursor);
        const signer_bytes = std.math.mul(usize, signer_count, 32) catch return error.InvalidNativeEncoding;
        if (cursor > data.len or signer_bytes > data.len - cursor) return error.InvalidNativeEncoding;
        const signers = try allocator.alloc(PublicKey, signer_count);
        errdefer allocator.free(signers);
        for (signers) |*signer| {
            @memcpy(signer, data[cursor .. cursor + 32]);
            cursor += 32;
        }
        group.* = .{ .claim = .{ .message = message, .slot = slot }, .signers = signers };
        initialized += 1;
    }
    if (cursor != data.len) return error.InvalidNativeEncoding;
    return .{ .items = groups };
}

fn takeU32(data: []const u8, cursor: *usize) !u32 {
    if (cursor.* > data.len or data.len - cursor.* < 4) return error.InvalidNativeEncoding;
    const value = std.mem.readInt(u32, data[cursor.* .. cursor.* + 4], .little);
    cursor.* += 4;
    return value;
}

fn take32(data: []const u8, cursor: *usize) ![32]u8 {
    if (cursor.* > data.len or data.len - cursor.* < 32) return error.InvalidNativeEncoding;
    var result: [32]u8 = undefined;
    @memcpy(&result, data[cursor.* .. cursor.* + 32]);
    cursor.* += 32;
    return result;
}
