const std = @import("std");
const domain = @import("../domain.zig");

pub fn run(
    allocator: std.mem.Allocator,
    yak_service: *domain.YakService,
    args: []const []const u8,
) !u8 {
    if (args.len < 2) {
        printHelp();
        return 0;
    }

    const command = args[1];

    if (std.mem.eql(u8, command, "--help") or std.mem.eql(u8, command, "help")) {
        printHelp();
        return 0;
    } else if (std.mem.eql(u8, command, "add")) {
        return try cmdAdd(allocator, yak_service, args[2..]);
    } else if (std.mem.eql(u8, command, "list") or std.mem.eql(u8, command, "ls")) {
        return try cmdList(allocator, yak_service, args[2..]);
    } else if (std.mem.eql(u8, command, "done")) {
        return try cmdDone(allocator, yak_service, args[2..]);
    } else if (std.mem.eql(u8, command, "rm")) {
        return try cmdRm(allocator, yak_service, args[2..]);
    } else if (std.mem.eql(u8, command, "prune")) {
        return try cmdPrune(yak_service);
    } else if (std.mem.eql(u8, command, "move") or std.mem.eql(u8, command, "mv")) {
        return try cmdMove(allocator, yak_service, args[2..]);
    } else if (std.mem.eql(u8, command, "context")) {
        return try cmdContext(allocator, yak_service, args[2..]);
    } else if (std.mem.eql(u8, command, "sync")) {
        return try cmdSync(yak_service);
    } else {
        printHelp();
        return 0;
    }
}

fn cmdAdd(
    allocator: std.mem.Allocator,
    yak_service: *domain.YakService,
    args: []const []const u8,
) !u8 {
    if (args.len == 0) {
        // Interactive mode
        const stdout = std.io.getStdOut().writer();
        const stdin = std.io.getStdIn().reader();

        try stdout.writeAll("Enter yaks (empty line to finish):\n");

        var buf: [1024]u8 = undefined;
        while (true) {
            const line = (try stdin.readUntilDelimiterOrEof(&buf, '\n')) orelse break;
            if (line.len == 0) break;

            yak_service.addYak(line) catch |err| {
                const stderr = std.io.getStdErr().writer();
                try stderr.print("Error adding yak: {}\n", .{err});
                return 1;
            };
        }
        return 0;
    }

    // Join all arguments as yak name
    const yak_name = try std.mem.join(allocator, " ", args);
    defer allocator.free(yak_name);

    yak_service.addYak(yak_name) catch |err| {
        const stderr = std.io.getStdErr().writer();
        if (err == error.InvalidYakName) {
            try stderr.writeAll("Invalid yak name: contains forbidden characters (\\ : * ? | < > \")\n");
        } else {
            try stderr.print("Error: {}\n", .{err});
        }
        return 1;
    };

    return 0;
}

fn cmdList(
    allocator: std.mem.Allocator,
    yak_service: *domain.YakService,
    args: []const []const u8,
) !u8 {
    var format: []const u8 = "markdown";
    var only: ?[]const u8 = null;

    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--format") and i + 1 < args.len) {
            format = args[i + 1];
            i += 1;
        } else if (std.mem.eql(u8, args[i], "--only") and i + 1 < args.len) {
            only = args[i + 1];
            i += 1;
        }
    }

    const yaks = try yak_service.listYaks();
    defer {
        for (yaks) |yak| {
            allocator.free(yak.name);
            allocator.free(yak.context);
        }
        allocator.free(yaks);
    }

    // Filter yaks based on --only flag
    var filtered_yaks = std.ArrayList(domain.Yak).init(allocator);
    defer filtered_yaks.deinit();

    for (yaks) |yak| {
        const should_include = if (only) |filter| blk: {
            if (std.mem.eql(u8, filter, "done")) {
                break :blk yak.state == .done;
            } else if (std.mem.eql(u8, filter, "not-done")) {
                break :blk yak.state == .todo;
            } else {
                break :blk true;
            }
        } else true;

        if (should_include) {
            try filtered_yaks.append(yak);
        }
    }

    const stdout = std.io.getStdOut().writer();

    if (filtered_yaks.items.len == 0) {
        if (std.mem.eql(u8, format, "plain") or std.mem.eql(u8, format, "raw")) {
            return 0;
        }
        try stdout.writeAll("You have no yaks. Are you done?\n");
        return 0;
    }

    // Sort yaks (done first, then by name for now - proper mtime sorting would need stat)
    std.mem.sort(domain.Yak, filtered_yaks.items, {}, yakLessThan);

    for (filtered_yaks.items) |yak| {
        if (std.mem.eql(u8, format, "plain") or std.mem.eql(u8, format, "raw")) {
            try stdout.print("{s}\n", .{yak.name});
        } else {
            // Markdown format
            const depth = std.mem.count(u8, yak.name, "/");
            const indent_spaces = depth * 2;

            var indent_buf: [100]u8 = undefined;
            const indent = indent_buf[0..indent_spaces];
            @memset(indent, ' ');

            const display_name = if (std.mem.lastIndexOf(u8, yak.name, "/")) |idx|
                yak.name[idx + 1 ..]
            else
                yak.name;

            if (yak.state == .done) {
                try stdout.print("\x1b[90m{s}- [x] {s}\x1b[0m\n", .{ indent, display_name });
            } else {
                try stdout.print("{s}- [ ] {s}\n", .{ indent, display_name });
            }
        }
    }

    return 0;
}

fn yakLessThan(context: void, a: domain.Yak, b: domain.Yak) bool {
    _ = context;
    // Sort by state (done first), then by name
    if (a.state != b.state) {
        return a.state == .done and b.state == .todo;
    }
    return std.mem.lessThan(u8, a.name, b.name);
}

fn cmdDone(
    allocator: std.mem.Allocator,
    yak_service: *domain.YakService,
    args: []const []const u8,
) !u8 {
    if (args.len == 0) {
        const stderr = std.io.getStdErr().writer();
        try stderr.writeAll("Error: yak name required\n");
        return 1;
    }

    const stderr = std.io.getStdErr().writer();

    if (std.mem.eql(u8, args[0], "--undo")) {
        if (args.len < 2) {
            try stderr.writeAll("Error: yak name required\n");
            return 1;
        }

        const yak_name = try std.mem.join(allocator, " ", args[1..]);
        defer allocator.free(yak_name);

        yak_service.undoDone(yak_name) catch |err| {
            try printYakError(stderr, err, yak_name);
            return 1;
        };
    } else if (std.mem.eql(u8, args[0], "--recursive")) {
        if (args.len < 2) {
            try stderr.writeAll("Error: yak name required\n");
            return 1;
        }

        const yak_name = try std.mem.join(allocator, " ", args[1..]);
        defer allocator.free(yak_name);

        yak_service.markDone(yak_name, true) catch |err| {
            try printYakError(stderr, err, yak_name);
            return 1;
        };
    } else {
        const yak_name = try std.mem.join(allocator, " ", args);
        defer allocator.free(yak_name);

        yak_service.markDone(yak_name, false) catch |err| {
            if (err == error.HasIncompleteChildren) {
                try stderr.print("Error: cannot mark '{s}' as done - it has incomplete children\n", .{yak_name});
            } else {
                try printYakError(stderr, err, yak_name);
            }
            return 1;
        };
    }

    return 0;
}

fn cmdRm(
    allocator: std.mem.Allocator,
    yak_service: *domain.YakService,
    args: []const []const u8,
) !u8 {
    if (args.len == 0) {
        const stderr = std.io.getStdErr().writer();
        try stderr.writeAll("Error: yak name required\n");
        return 1;
    }

    const yak_name = try std.mem.join(allocator, " ", args);
    defer allocator.free(yak_name);

    yak_service.removeYak(yak_name) catch |err| {
        const stderr = std.io.getStdErr().writer();
        try printYakError(stderr, err, yak_name);
        return 1;
    };

    return 0;
}

fn cmdPrune(yak_service: *domain.YakService) !u8 {
    yak_service.pruneYaks() catch |err| {
        const stderr = std.io.getStdErr().writer();
        try stderr.print("Error: {}\n", .{err});
        return 1;
    };
    return 0;
}

fn cmdMove(
    allocator: std.mem.Allocator,
    yak_service: *domain.YakService,
    args: []const []const u8,
) !u8 {
    if (args.len < 2) {
        const stderr = std.io.getStdErr().writer();
        try stderr.writeAll("Error: old and new names required\n");
        return 1;
    }

    const old_name = args[0];
    const new_name = try std.mem.join(allocator, " ", args[1..]);
    defer allocator.free(new_name);

    yak_service.moveYak(old_name, new_name) catch |err| {
        const stderr = std.io.getStdErr().writer();
        if (err == error.InvalidYakName) {
            try stderr.writeAll("Invalid yak name: contains forbidden characters (\\ : * ? | < > \")\n");
        } else {
            try printYakError(stderr, err, old_name);
        }
        return 1;
    };

    return 0;
}

fn cmdContext(
    allocator: std.mem.Allocator,
    yak_service: *domain.YakService,
    args: []const []const u8,
) !u8 {
    if (args.len == 0) {
        const stderr = std.io.getStdErr().writer();
        try stderr.writeAll("Error: yak name required\n");
        return 1;
    }

    const show_mode = std.mem.eql(u8, args[0], "--show");
    const yak_name = if (show_mode)
        try std.mem.join(allocator, " ", args[1..])
    else
        try std.mem.join(allocator, " ", args);
    defer allocator.free(yak_name);

    if (show_mode) {
        const yak = yak_service.getContext(yak_name) catch |err| {
            const stderr = std.io.getStdErr().writer();
            try printYakError(stderr, err, yak_name);
            return 1;
        };
        defer {
            allocator.free(yak.name);
            allocator.free(yak.context);
        }

        const stdout = std.io.getStdOut().writer();
        try stdout.print("{s}\n", .{yak.name});
        if (yak.context.len > 0) {
            try stdout.print("\n{s}\n", .{yak.context});
        }
    } else {
        // Read from stdin
        const stdin = std.io.getStdIn().reader();
        const context = try stdin.readAllAlloc(allocator, 1024 * 1024);
        defer allocator.free(context);

        yak_service.setContext(yak_name, context) catch |err| {
            const stderr = std.io.getStdErr().writer();
            try printYakError(stderr, err, yak_name);
            return 1;
        };
    }

    return 0;
}

fn cmdSync(yak_service: *domain.YakService) !u8 {
    yak_service.syncYaks() catch |err| {
        const stderr = std.io.getStdErr().writer();
        try stderr.print("Error: {}\n", .{err});
        return 1;
    };
    return 0;
}

fn printYakError(stderr: anytype, err: anyerror, yak_name: []const u8) !void {
    if (err == error.YakNotFound) {
        try stderr.print("Error: yak '{s}' not found\n", .{yak_name});
    } else if (err == error.AmbiguousYakName) {
        try stderr.print("Error: yak name '{s}' is ambiguous\n", .{yak_name});
    } else {
        try stderr.print("Error: {}\n", .{err});
    }
}

fn printHelp() void {
    const stdout = std.io.getStdOut().writer();
    stdout.writeAll(
        \\Usage: yx <command> [arguments]
        \\
        \\Commands:
        \\  add <name>                      Add a new yak
        \\  list, ls [--format FMT]         List all yaks
        \\           [--only STATE]
        \\                          --format: Output format
        \\                                    markdown (or md): Checkbox format (default)
        \\                                    plain (or raw): Simple list of names
        \\                          --only: Show only yaks in a specific state
        \\                                  not-done: Show only incomplete yaks
        \\                                  done: Show only completed yaks
        \\  context [--show] <name>         Edit context (uses $EDITOR) or set from stdin
        \\                          --show: Display yak with context
        \\                          --edit: Edit context (default)
        \\  done <name>                     Mark a yak as done
        \\  done --undo <name>              Unmark a yak as done
        \\  rm <name>                       Remove a yak by name
        \\  move <old> <new>                Rename a yak
        \\  mv <old> <new>                  Alias for move
        \\  prune                           Remove all done yaks
        \\  sync                            Push and pull yaks to/from origin via git ref
        \\  --help                          Show this help message
        \\
    ) catch {};
}
