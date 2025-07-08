const std = @import("std");

/// This function is called by the Zig compiler when we run `zig build`.
pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const lib_mod = b.createModule(.{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    const lib = b.addLibrary(.{
        .linkage = .static,
        .name = "spore",
        .root_module = lib_mod,
    });
    b.installArtifact(lib);
    const lib_unit_tests = b.addTest(.{
        .root_module = lib_mod,
    });

    // Tests
    const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_lib_unit_tests.step);

    // Docs
    const install_docs = b.addInstallDirectory(.{
        .source_dir = lib.getEmittedDocs(),
        .install_dir = .{ .custom = "docs" },
        .install_subdir = ".",
    });
    const docs_step = b.step("doc", "Install docs into zig-out/docs");
    docs_step.dependOn(&install_docs.step);
}
