const std = @import("std");
const domain = @import("../domain.zig");

pub const FilesystemAdapter = struct {
    allocator: std.mem.Allocator,
    yaks_path: []const u8,
    storage: domain.StoragePort,

    pub fn init(allocator: std.mem.Allocator, git_work_tree: []const u8) FilesystemAdapter {
        const yaks_path = std.fmt.allocPrint(
            allocator,
            "{s}/.yaks",
            .{git_work_tree},
        ) catch unreachable;

        var self = FilesystemAdapter{
            .allocator = allocator,
            .yaks_path = yaks_path,
            .storage = undefined,
        };

        self.storage = domain.StoragePort{
            .ptr = &self,
            .vtable = &.{
                .createYak = createYak,
                .readYak = readYak,
                .updateYakState = updateYakState,
                .updateYakContext = updateYakContext,
                .deleteYak = deleteYak,
                .listYaks = listYaks,
                .findYak = findYak,
                .hasIncompleteChildren = hasIncompleteChildren,
            },
        };

        return self;
    }

    fn createYak(ptr: *anyopaque, name: []const u8) !void {
        const self: *FilesystemAdapter = @ptrCast(@alignCast(ptr));

        const yak_dir = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ self.yaks_path, name });
        defer self.allocator.free(yak_dir);

        // Create parent directory if needed
        if (std.fs.path.dirname(yak_dir)) |parent_dir| {
            try std.fs.cwd().makePath(parent_dir);
        }

        // Create yak directory
        try std.fs.cwd().makePath(yak_dir);

        // Write state file
        const state_path = try std.fmt.allocPrint(self.allocator, "{s}/state", .{yak_dir});
        defer self.allocator.free(state_path);

        const state_file = try std.fs.cwd().createFile(state_path, .{});
        defer state_file.close();
        try state_file.writeAll("todo");

        // Create empty context.md
        const context_path = try std.fmt.allocPrint(self.allocator, "{s}/context.md", .{yak_dir});
        defer self.allocator.free(context_path);

        const context_file = try std.fs.cwd().createFile(context_path, .{});
        defer context_file.close();
    }

    fn readYak(ptr: *anyopaque, allocator: std.mem.Allocator, name: []const u8) !domain.Yak {
        const self: *FilesystemAdapter = @ptrCast(@alignCast(ptr));

        const yak_dir = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ self.yaks_path, name });
        defer self.allocator.free(yak_dir);

        // Read state
        const state_path = try std.fmt.allocPrint(self.allocator, "{s}/state", .{yak_dir});
        defer self.allocator.free(state_path);

        const state_file = try std.fs.cwd().openFile(state_path, .{});
        defer state_file.close();

        var state_buf: [10]u8 = undefined;
        const state_len = try state_file.readAll(&state_buf);
        const state_str = state_buf[0..state_len];

        const state: domain.Yak.YakState = if (std.mem.eql(u8, state_str, "done"))
            .done
        else
            .todo;

        // Read context
        const context_path = try std.fmt.allocPrint(self.allocator, "{s}/context.md", .{yak_dir});
        defer self.allocator.free(context_path);

        const context_file = std.fs.cwd().openFile(context_path, .{}) catch |err| {
            if (err == error.FileNotFound) {
                return domain.Yak{
                    .name = try allocator.dupe(u8, name),
                    .state = state,
                    .context = try allocator.dupe(u8, ""),
                };
            }
            return err;
        };
        defer context_file.close();

        const context = try context_file.readToEndAlloc(allocator, 1024 * 1024);

        return domain.Yak{
            .name = try allocator.dupe(u8, name),
            .state = state,
            .context = context,
        };
    }

    fn updateYakState(ptr: *anyopaque, name: []const u8, state: domain.Yak.YakState) !void {
        const self: *FilesystemAdapter = @ptrCast(@alignCast(ptr));

        const yak_dir = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ self.yaks_path, name });
        defer self.allocator.free(yak_dir);

        const state_path = try std.fmt.allocPrint(self.allocator, "{s}/state", .{yak_dir});
        defer self.allocator.free(state_path);

        const state_file = try std.fs.cwd().createFile(state_path, .{});
        defer state_file.close();

        const state_str = if (state == .done) "done" else "todo";
        try state_file.writeAll(state_str);
    }

    fn updateYakContext(ptr: *anyopaque, name: []const u8, context: []const u8) !void {
        const self: *FilesystemAdapter = @ptrCast(@alignCast(ptr));

        const yak_dir = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ self.yaks_path, name });
        defer self.allocator.free(yak_dir);

        const context_path = try std.fmt.allocPrint(self.allocator, "{s}/context.md", .{yak_dir});
        defer self.allocator.free(context_path);

        const context_file = try std.fs.cwd().createFile(context_path, .{});
        defer context_file.close();
        try context_file.writeAll(context);
    }

    fn deleteYak(ptr: *anyopaque, name: []const u8) !void {
        const self: *FilesystemAdapter = @ptrCast(@alignCast(ptr));

        const yak_dir = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ self.yaks_path, name });
        defer self.allocator.free(yak_dir);

        try std.fs.cwd().deleteTree(yak_dir);
    }

    fn listYaks(ptr: *anyopaque, allocator: std.mem.Allocator) ![]domain.Yak {
        const self: *FilesystemAdapter = @ptrCast(@alignCast(ptr));

        var yaks: std.ArrayList(domain.Yak) = .init(allocator);
        errdefer yaks.deinit(allocator);

        // Check if .yaks directory exists
        var yaks_dir = std.fs.cwd().openDir(self.yaks_path, .{ .iterate = true }) catch |err| {
            if (err == error.FileNotFound) {
                return yaks.toOwnedSlice(allocator);
            }
            return err;
        };
        defer yaks_dir.close();

        try walkYakDir(self, allocator, &yaks, yaks_dir, "");

        return yaks.toOwnedSlice(allocator);
    }

    fn walkYakDir(
        self: *FilesystemAdapter,
        allocator: std.mem.Allocator,
        yaks: *std.ArrayList(domain.Yak),
        dir: std.fs.Dir,
        prefix: []const u8,
    ) !void {
        var iter = dir.iterate();
        while (try iter.next()) |entry| {
            if (entry.kind != .directory) continue;

            const yak_name = if (prefix.len == 0)
                try allocator.dupe(u8, entry.name)
            else
                try std.fmt.allocPrint(allocator, "{s}/{s}", .{ prefix, entry.name });

            // Try to read the yak (it might be a directory but not a yak)
            const yak = readYak(self, allocator, yak_name) catch {
                allocator.free(yak_name);
                continue;
            };

            try yaks.append(yak);

            // Recurse into subdirectories
            var subdir = try dir.openDir(entry.name, .{ .iterate = true });
            defer subdir.close();
            try walkYakDir(self, allocator, yaks, subdir, yak_name);
        }
    }

    fn findYak(ptr: *anyopaque, allocator: std.mem.Allocator, search_term: []const u8) !?[]const u8 {
        const self: *FilesystemAdapter = @ptrCast(@alignCast(ptr));

        // Try exact match first
        const exact_path = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ self.yaks_path, search_term });
        defer self.allocator.free(exact_path);

        std.fs.cwd().access(exact_path, .{}) catch |err| {
            if (err == error.FileNotFound) {
                // Try fuzzy match
                return try fuzzyFindYak(self, allocator, search_term);
            }
            return err;
        };

        return try allocator.dupe(u8, search_term);
    }

    fn fuzzyFindYak(self: *FilesystemAdapter, allocator: std.mem.Allocator, search_term: []const u8) !?[]const u8 {
        const all_yaks = try listYaks(self, allocator);
        defer {
            for (all_yaks) |yak| {
                allocator.free(yak.name);
                allocator.free(yak.context);
            }
            allocator.free(all_yaks);
        }

        var matches = std.ArrayList([]const u8).init(allocator);
        defer matches.deinit();

        for (all_yaks) |yak| {
            if (std.mem.indexOf(u8, yak.name, search_term) != null) {
                try matches.append(yak.name);
            }
        }

        if (matches.items.len == 0) {
            return null;
        } else if (matches.items.len == 1) {
            return try allocator.dupe(u8, matches.items[0]);
        } else {
            // Ambiguous match
            return error.AmbiguousYakName;
        }
    }

    fn hasIncompleteChildren(ptr: *anyopaque, name: []const u8) !bool {
        const self: *FilesystemAdapter = @ptrCast(@alignCast(ptr));

        const yak_dir_path = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ self.yaks_path, name });
        defer self.allocator.free(yak_dir_path);

        var yak_dir = std.fs.cwd().openDir(yak_dir_path, .{ .iterate = true }) catch |err| {
            if (err == error.FileNotFound) return false;
            return err;
        };
        defer yak_dir.close();

        var iter = yak_dir.iterate();
        while (try iter.next()) |entry| {
            if (entry.kind != .directory) continue;

            const child_name = try std.fmt.allocPrint(self.allocator, "{s}/{s}", .{ name, entry.name });
            defer self.allocator.free(child_name);

            const child = try readYak(self, self.allocator, child_name);
            defer {
                self.allocator.free(child.name);
                self.allocator.free(child.context);
            }

            if (child.state == .todo) {
                return true;
            }
        }

        return false;
    }
};
