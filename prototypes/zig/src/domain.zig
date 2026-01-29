const std = @import("std");

// Yak represents a task in the system
pub const Yak = struct {
    name: []const u8,
    state: YakState,
    context: []const u8,

    pub const YakState = enum {
        todo,
        done,
    };
};

// Port: Storage interface (implemented by filesystem adapter)
pub const StoragePort = struct {
    ptr: *anyopaque,
    vtable: *const VTable,

    pub const VTable = struct {
        createYak: *const fn (ptr: *anyopaque, name: []const u8) anyerror!void,
        readYak: *const fn (ptr: *anyopaque, allocator: std.mem.Allocator, name: []const u8) anyerror!Yak,
        updateYakState: *const fn (ptr: *anyopaque, name: []const u8, state: Yak.YakState) anyerror!void,
        updateYakContext: *const fn (ptr: *anyopaque, name: []const u8, context: []const u8) anyerror!void,
        deleteYak: *const fn (ptr: *anyopaque, name: []const u8) anyerror!void,
        listYaks: *const fn (ptr: *anyopaque, allocator: std.mem.Allocator) anyerror![]Yak,
        findYak: *const fn (ptr: *anyopaque, allocator: std.mem.Allocator, search_term: []const u8) anyerror!?[]const u8,
        hasIncompleteChildren: *const fn (ptr: *anyopaque, name: []const u8) anyerror!bool,
    };

    pub fn createYak(self: StoragePort, name: []const u8) !void {
        return self.vtable.createYak(self.ptr, name);
    }

    pub fn readYak(self: StoragePort, allocator: std.mem.Allocator, name: []const u8) !Yak {
        return self.vtable.readYak(self.ptr, allocator, name);
    }

    pub fn updateYakState(self: StoragePort, name: []const u8, state: Yak.YakState) !void {
        return self.vtable.updateYakState(self.ptr, name, state);
    }

    pub fn updateYakContext(self: StoragePort, name: []const u8, context: []const u8) !void {
        return self.vtable.updateYakContext(self.ptr, name, context);
    }

    pub fn deleteYak(self: StoragePort, name: []const u8) !void {
        return self.vtable.deleteYak(self.ptr, name);
    }

    pub fn listYaks(self: StoragePort, allocator: std.mem.Allocator) ![]Yak {
        return self.vtable.listYaks(self.ptr, allocator);
    }

    pub fn findYak(self: StoragePort, allocator: std.mem.Allocator, search_term: []const u8) !?[]const u8 {
        return self.vtable.findYak(self.ptr, allocator, search_term);
    }

    pub fn hasIncompleteChildren(self: StoragePort, name: []const u8) !bool {
        return self.vtable.hasIncompleteChildren(self.ptr, name);
    }
};

// Port: Git operations interface (implemented by git adapter)
pub const GitPort = struct {
    ptr: *anyopaque,
    vtable: *const VTable,

    pub const VTable = struct {
        logCommand: *const fn (ptr: *anyopaque, command: []const u8) anyerror!void,
        isRepository: *const fn (ptr: *anyopaque) bool,
        sync: *const fn (ptr: *anyopaque) anyerror!void,
    };

    pub fn logCommand(self: GitPort, command: []const u8) !void {
        return self.vtable.logCommand(self.ptr, command);
    }

    pub fn isRepository(self: GitPort) bool {
        return self.vtable.isRepository(self.ptr);
    }

    pub fn sync(self: GitPort) !void {
        return self.vtable.sync(self.ptr);
    }
};

// Domain Service: Core business logic
pub const YakService = struct {
    allocator: std.mem.Allocator,
    storage: StoragePort,
    git: GitPort,

    pub fn init(
        allocator: std.mem.Allocator,
        storage: StoragePort,
        git: GitPort,
    ) YakService {
        return .{
            .allocator = allocator,
            .storage = storage,
            .git = git,
        };
    }

    pub fn addYak(self: *YakService, name: []const u8) !void {
        try validateYakName(name);
        try self.storage.createYak(name);
        const log_msg = try std.fmt.allocPrint(self.allocator, "add {s}", .{name});
        defer self.allocator.free(log_msg);
        if (self.git.isRepository()) {
            try self.git.logCommand(log_msg);
        }
    }

    pub fn listYaks(self: *YakService) ![]Yak {
        return self.storage.listYaks(self.allocator);
    }

    pub fn markDone(self: *YakService, name: []const u8, recursive: bool) !void {
        const resolved_name = try self.requireYak(name);
        defer self.allocator.free(resolved_name);

        if (!recursive) {
            if (try self.storage.hasIncompleteChildren(resolved_name)) {
                return error.HasIncompleteChildren;
            }
        }

        if (recursive) {
            try self.markDoneRecursive(resolved_name);
        } else {
            try self.storage.updateYakState(resolved_name, .done);
        }

        const log_msg = if (recursive)
            try std.fmt.allocPrint(self.allocator, "done --recursive {s}", .{resolved_name})
        else
            try std.fmt.allocPrint(self.allocator, "done {s}", .{resolved_name});
        defer self.allocator.free(log_msg);

        if (self.git.isRepository()) {
            try self.git.logCommand(log_msg);
        }
    }

    fn markDoneRecursive(self: *YakService, name: []const u8) !void {
        try self.storage.updateYakState(name, .done);
        // TODO: Implement recursive marking of children
    }

    pub fn undoDone(self: *YakService, name: []const u8) !void {
        const resolved_name = try self.requireYak(name);
        defer self.allocator.free(resolved_name);

        try self.storage.updateYakState(resolved_name, .todo);

        const log_msg = try std.fmt.allocPrint(self.allocator, "done --undo {s}", .{resolved_name});
        defer self.allocator.free(log_msg);

        if (self.git.isRepository()) {
            try self.git.logCommand(log_msg);
        }
    }

    pub fn removeYak(self: *YakService, name: []const u8) !void {
        const resolved_name = try self.requireYak(name);
        defer self.allocator.free(resolved_name);

        try self.storage.deleteYak(resolved_name);

        const log_msg = try std.fmt.allocPrint(self.allocator, "rm {s}", .{resolved_name});
        defer self.allocator.free(log_msg);

        if (self.git.isRepository()) {
            try self.git.logCommand(log_msg);
        }
    }

    pub fn pruneYaks(self: *YakService) !void {
        const yaks = try self.storage.listYaks(self.allocator);
        defer {
            for (yaks) |yak| {
                self.allocator.free(yak.name);
                self.allocator.free(yak.context);
            }
            self.allocator.free(yaks);
        }

        for (yaks) |yak| {
            if (yak.state == .done) {
                try self.storage.deleteYak(yak.name);
            }
        }
    }

    pub fn moveYak(self: *YakService, old_name: []const u8, new_name: []const u8) !void {
        const resolved_old = try self.requireYak(old_name);
        defer self.allocator.free(resolved_old);

        try validateYakName(new_name);

        // Read old yak
        const yak = try self.storage.readYak(self.allocator, resolved_old);
        defer {
            self.allocator.free(yak.name);
            self.allocator.free(yak.context);
        }

        // Create new yak with same data
        try self.storage.createYak(new_name);
        try self.storage.updateYakState(new_name, yak.state);
        try self.storage.updateYakContext(new_name, yak.context);

        // Delete old yak
        try self.storage.deleteYak(resolved_old);

        const log_msg = try std.fmt.allocPrint(self.allocator, "move {s} {s}", .{ resolved_old, new_name });
        defer self.allocator.free(log_msg);

        if (self.git.isRepository()) {
            try self.git.logCommand(log_msg);
        }
    }

    pub fn setContext(self: *YakService, name: []const u8, context: []const u8) !void {
        const resolved_name = try self.requireYak(name);
        defer self.allocator.free(resolved_name);

        try self.storage.updateYakContext(resolved_name, context);

        const log_msg = try std.fmt.allocPrint(self.allocator, "context {s}", .{resolved_name});
        defer self.allocator.free(log_msg);

        if (self.git.isRepository()) {
            try self.git.logCommand(log_msg);
        }
    }

    pub fn getContext(self: *YakService, name: []const u8) !Yak {
        const resolved_name = try self.requireYak(name);
        defer self.allocator.free(resolved_name);

        return self.storage.readYak(self.allocator, resolved_name);
    }

    pub fn syncYaks(self: *YakService) !void {
        try self.git.sync();
    }

    // Helper: Require yak to exist and resolve fuzzy match
    fn requireYak(self: *YakService, name: []const u8) ![]const u8 {
        if (try self.storage.findYak(self.allocator, name)) |resolved| {
            return resolved;
        }
        return error.YakNotFound;
    }
};

// Validation function for yak names
fn validateYakName(name: []const u8) !void {
    const forbidden = "\\:*?|<>\"";
    for (name) |c| {
        for (forbidden) |f| {
            if (c == f) {
                return error.InvalidYakName;
            }
        }
    }
}

test "validateYakName accepts valid names" {
    try validateYakName("valid-name");
    try validateYakName("valid name");
    try validateYakName("parent/child");
}

test "validateYakName rejects invalid names" {
    try std.testing.expectError(error.InvalidYakName, validateYakName("invalid\\name"));
    try std.testing.expectError(error.InvalidYakName, validateYakName("invalid:name"));
    try std.testing.expectError(error.InvalidYakName, validateYakName("invalid*name"));
}
