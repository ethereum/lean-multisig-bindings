const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const native_dir = b.option([]const u8, "native-dir", "Directory containing liblean_multisig_native.a") orelse
        @panic("pass -Dnative-dir=PATH_TO_NATIVE_ARCHIVE");

    const tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/lean_multisig_test.zig"),
            .target = target,
            .optimize = optimize,
            .link_libc = true,
        }),
    });
    tests.root_module.addIncludePath(b.path("include"));
    tests.root_module.addLibraryPath(b.path(native_dir));
    tests.root_module.linkSystemLibrary("lean_multisig_native", .{ .preferred_link_mode = .static });
    tests.root_module.linkSystemLibrary("m", .{});
    if (target.result.os.tag == .linux) {
        tests.root_module.linkSystemLibrary("dl", .{});
        tests.root_module.linkSystemLibrary("pthread", .{});
        tests.root_module.linkSystemLibrary("unwind", .{});
    }
    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run Zig binding tests");
    test_step.dependOn(&run_tests.step);
}
