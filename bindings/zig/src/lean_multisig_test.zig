const std = @import("std");
const lms = @import("lean_multisig.zig");

test "signs, aggregates, restores, and verifies" {
    const allocator = std.testing.allocator;
    try lms.setup();
    const claim = lms.Claim{ .message = [_]u8{42} ** 32, .slot = 100 };

    var alice = try lms.SecretKey.fromSeed([_]u8{1} ** 32, 100, 115);
    defer alice.deinit();
    var bob = try lms.SecretKey.fromSeed([_]u8{2} ** 32, 100, 115);
    defer bob.deinit();

    var alice_signature = try alice.sign(claim);
    defer alice_signature.deinit();
    var bob_signature = try bob.sign(claim);
    defer bob_signature.deinit();
    const signatures = [_]*const lms.Signature{ &alice_signature, &bob_signature };
    var combined = try lms.aggregate(allocator, &signatures, claim);
    defer combined.deinit();

    const signers = [_]lms.PublicKey{
        try alice.publicKey(allocator),
        try bob.publicKey(allocator),
    };
    try std.testing.expect(try combined.verify(&signers, claim));

    const encoded = try combined.toBytes(allocator);
    defer allocator.free(encoded);
    var restored = try lms.Signature.fromBytes(encoded, claim, &signers);
    defer restored.deinit();
    try std.testing.expect(try restored.verify(&signers, claim));
}

test "merges claims and restores the proof" {
    const allocator = std.testing.allocator;
    try lms.setup();
    const first = lms.Claim{ .message = [_]u8{3} ** 32, .slot = 100 };
    const second = lms.Claim{ .message = [_]u8{4} ** 32, .slot = 101 };
    var key = try lms.SecretKey.fromSeed([_]u8{5} ** 32, 100, 115);
    defer key.deinit();
    var first_signature = try key.sign(first);
    defer first_signature.deinit();
    var second_signature = try key.sign(second);
    defer second_signature.deinit();
    const signatures = [_]*const lms.Signature{ &first_signature, &second_signature };
    var proof = try lms.mergeClaims(allocator, &signatures);
    defer proof.deinit();

    const public_key = try key.publicKey(allocator);
    const signer_set = [_]lms.PublicKey{public_key};
    const groups = [_]lms.ClaimSigners{
        .{ .claim = first, .signers = &signer_set },
        .{ .claim = second, .signers = &signer_set },
    };
    try std.testing.expect(try proof.verifyClaims(allocator, &groups));

    const encoded = try proof.toBytes(allocator);
    defer allocator.free(encoded);
    var restored = try lms.MultiClaimProof.fromBytes(allocator, encoded, &groups);
    defer restored.deinit();
    var actual = try restored.verifiedClaims(allocator);
    defer actual.deinit(allocator);
    try std.testing.expectEqual(@as(usize, 2), actual.items.len);
    try std.testing.expect(try restored.verifyClaims(allocator, actual.items));
}
