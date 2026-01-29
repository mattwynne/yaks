const std = @import("std");
const domain = @import("domain.zig");
const fs_adapter = @import("adapters/filesystem.zig");
const git_adapter = @import("adapters/git.zig");
const cli_adapter = @import("adapters/cli.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    // Get current working directory
    var cwd_buf: [std.fs.max_path_bytes]u8 = undefined;
    const cwd = try std.process.getCwd(&cwd_buf);

    // Determine git work tree (can be overridden by GIT_WORK_TREE env var)
    const git_work_tree = std.process.getEnvVarOwned(
        allocator,
        "GIT_WORK_TREE",
    ) catch try allocator.dupe(u8, cwd);
    defer allocator.free(git_work_tree);

    // Initialize adapters
    var fs = fs_adapter.FilesystemAdapter.init(allocator, git_work_tree);
    var git = git_adapter.GitAdapter.init(allocator, git_work_tree);

    // Initialize domain service with adapters
    var yak_service = domain.YakService.init(
        allocator,
        fs.storage,
        git.git_ops,
    );

    // Run CLI
    const exit_code = try cli_adapter.run(allocator, &yak_service, args);
    std.process.exit(exit_code);
}
