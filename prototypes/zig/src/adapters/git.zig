const std = @import("std");
const domain = @import("../domain.zig");

pub const GitAdapter = struct {
    allocator: std.mem.Allocator,
    git_work_tree: []const u8,
    git_ops: domain.GitPort,

    pub fn init(allocator: std.mem.Allocator, git_work_tree: []const u8) GitAdapter {
        var self = GitAdapter{
            .allocator = allocator,
            .git_work_tree = git_work_tree,
            .git_ops = undefined,
        };

        self.git_ops = domain.GitPort{
            .ptr = &self,
            .vtable = &.{
                .logCommand = logCommand,
                .isRepository = isRepository,
                .sync = sync,
            },
        };

        return self;
    }

    fn logCommand(ptr: *anyopaque, command: []const u8) !void {
        const self: *GitAdapter = @ptrCast(@alignCast(ptr));

        if (!isRepository(self)) return;

        const yaks_path = try std.fmt.allocPrint(self.allocator, "{s}/.yaks", .{self.git_work_tree});
        defer self.allocator.free(yaks_path);

        // Check if .yaks directory exists
        std.fs.cwd().access(yaks_path, .{}) catch return;

        // Create temporary index
        const temp_index = try std.fmt.allocPrint(
            self.allocator,
            "/tmp/yaks-index-{d}",
            .{std.time.milliTimestamp()},
        );
        defer self.allocator.free(temp_index);
        defer std.fs.cwd().deleteFile(temp_index) catch {};

        // Read tree empty
        _ = try runGitCommand(self.allocator, self.git_work_tree, &.{
            "git",
            "-C",
            self.git_work_tree,
            "read-tree",
            "--empty",
        }, temp_index, yaks_path);

        // Add all files
        _ = try runGitCommand(self.allocator, self.git_work_tree, &.{
            "git",
            "-C",
            self.git_work_tree,
            "add",
            ".",
        }, temp_index, yaks_path);

        // Write tree
        const tree_output = try runGitCommand(self.allocator, self.git_work_tree, &.{
            "git",
            "-C",
            self.git_work_tree,
            "write-tree",
        }, temp_index, yaks_path);
        defer self.allocator.free(tree_output);

        const tree = std.mem.trim(u8, tree_output, &std.ascii.whitespace);

        // Get parent if exists
        const parent_ref = runGitCommand(self.allocator, self.git_work_tree, &.{
            "git",
            "-C",
            self.git_work_tree,
            "rev-parse",
            "refs/notes/yaks",
        }, null, null) catch null;
        defer if (parent_ref) |p| self.allocator.free(p);

        // Commit tree
        var commit_args: std.ArrayList([]const u8) = .init(self.allocator);
        defer commit_args.deinit(self.allocator);

        try commit_args.appendSlice(&.{ "git", "-C", self.git_work_tree, "commit-tree", tree });

        if (parent_ref) |p| {
            const trimmed_parent = std.mem.trim(u8, p, &std.ascii.whitespace);
            try commit_args.append("-p");
            try commit_args.append(trimmed_parent);
        }

        try commit_args.appendSlice(&.{ "-m", command });

        const commit_output = try runGitCommand(
            self.allocator,
            self.git_work_tree,
            commit_args.items,
            null,
            null,
        );
        defer self.allocator.free(commit_output);

        const commit = std.mem.trim(u8, commit_output, &std.ascii.whitespace);

        // Update ref
        _ = try runGitCommand(self.allocator, self.git_work_tree, &.{
            "git",
            "-C",
            self.git_work_tree,
            "update-ref",
            "refs/notes/yaks",
            commit,
        }, null, null);
    }

    fn isRepository(ptr: *anyopaque) bool {
        const self: *GitAdapter = @ptrCast(@alignCast(ptr));

        const result = runGitCommand(self.allocator, self.git_work_tree, &.{
            "git",
            "-C",
            self.git_work_tree,
            "rev-parse",
            "--git-dir",
        }, null, null) catch return false;

        self.allocator.free(result);
        return true;
    }

    fn sync(ptr: *anyopaque) !void {
        const self: *GitAdapter = @ptrCast(@alignCast(ptr));

        // Fetch remote yaks ref
        _ = runGitCommand(self.allocator, self.git_work_tree, &.{
            "git",
            "-C",
            self.git_work_tree,
            "fetch",
            "origin",
            "refs/notes/yaks:refs/remotes/origin/yaks",
        }, null, null) catch {};

        // For now, simplified sync - just push local changes
        _ = runGitCommand(self.allocator, self.git_work_tree, &.{
            "git",
            "-C",
            self.git_work_tree,
            "push",
            "origin",
            "refs/notes/yaks:refs/notes/yaks",
        }, null, null) catch {};
    }

    fn runGitCommand(
        allocator: std.mem.Allocator,
        cwd: []const u8,
        argv: []const []const u8,
        git_index_file: ?[]const u8,
        git_work_tree: ?[]const u8,
    ) ![]u8 {
        var child = std.process.Child.init(argv, allocator);
        child.cwd = cwd;
        child.stdout_behavior = .Pipe;
        child.stderr_behavior = .Pipe;

        if (git_index_file) |index| {
            var env_map = try std.process.getEnvMap(allocator);
            defer env_map.deinit();
            try env_map.put("GIT_INDEX_FILE", index);
            if (git_work_tree) |wt| {
                try env_map.put("GIT_WORK_TREE", wt);
            }
            child.env_map = &env_map;
        }

        try child.spawn();

        const stdout = try child.stdout.?.readToEndAlloc(allocator, 10 * 1024 * 1024);
        errdefer allocator.free(stdout);

        const stderr = try child.stderr.?.readToEndAlloc(allocator, 10 * 1024 * 1024);
        defer allocator.free(stderr);

        const term = try child.wait();

        if (term != .Exited or term.Exited != 0) {
            allocator.free(stdout);
            return error.GitCommandFailed;
        }

        return stdout;
    }
};
