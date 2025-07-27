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
        .name = "spore",
        .linkage = .static,
        .root_module = lib_mod,
    });

    const exe_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    exe_mod.addImport("spore_lib", lib_mod);
    const exe = b.addExecutable(.{
        .name = "spore",
        .root_module = exe_mod,
    });

    b.installArtifact(exe);

    // Tests
    const lib_unit_tests = b.addTest(.{ .root_module = lib_mod });
    const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_lib_unit_tests.step);

    // Test Coverage
    const run_coverage = b.addSystemCommand(&.{
        "kcov",
        "--clean",
        "--include-pattern=src/",
        b.pathJoin(&.{ b.install_path, "coverage" }),
    });
    run_coverage.addArtifactArg(lib_unit_tests);
    const coverage_step = b.step("coverage", "Generate test coverage report");
    coverage_step.dependOn(&run_coverage.step);

    // Docs
    const install_docs = b.addInstallDirectory(.{
        .source_dir = lib.getEmittedDocs(),
        .install_dir = .{ .custom = "docs" },
        .install_subdir = ".",
    });
    const docs_step = b.step("doc", "Install docs into zig-out/docs");
    docs_step.dependOn(&install_docs.step);
}
