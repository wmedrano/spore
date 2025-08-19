//! Terminal color utilities using ANSI escape codes.
const std = @import("std");

/// ANSI color codes for terminal output
pub const ANSI = struct {
    pub const RESET = "\x1b[0m";
    pub const BOLD = "\x1b[1m";

    // Foreground colors
    pub const FG_BLACK = "\x1b[30m";
    pub const FG_RED = "\x1b[31m";
    pub const FG_GREEN = "\x1b[32m";
    pub const FG_YELLOW = "\x1b[33m";
    pub const FG_BLUE = "\x1b[34m";
    pub const FG_MAGENTA = "\x1b[35m";
    pub const FG_CYAN = "\x1b[36m";
    pub const FG_WHITE = "\x1b[37m";

    // Bright foreground colors
    pub const FG_BRIGHT_BLACK = "\x1b[90m";
    pub const FG_BRIGHT_RED = "\x1b[91m";
    pub const FG_BRIGHT_GREEN = "\x1b[92m";
    pub const FG_BRIGHT_YELLOW = "\x1b[93m";
    pub const FG_BRIGHT_BLUE = "\x1b[94m";
    pub const FG_BRIGHT_MAGENTA = "\x1b[95m";
    pub const FG_BRIGHT_CYAN = "\x1b[96m";
    pub const FG_BRIGHT_WHITE = "\x1b[97m";
};

/// Color themes for different output categories
pub const Theme = struct {
    prompt: []const u8,
    success: []const u8,
    err: []const u8,
    info: []const u8,
    special: []const u8,
    reset: []const u8,

    pub const default = Theme{
        .prompt = ANSI.FG_BLUE,
        .success = ANSI.FG_GREEN,
        .err = ANSI.FG_RED,
        .info = ANSI.FG_YELLOW,
        .special = ANSI.FG_CYAN,
        .reset = ANSI.RESET,
    };
};

/// Checks if the terminal supports color output
pub fn supportsColor() bool {
    if (!std.posix.isatty(std.posix.STDOUT_FILENO)) return false;

    const term = std.posix.getenv("TERM") orelse return false;
    const no_color = std.posix.getenv("NO_COLOR");
    if (no_color != null) return false;

    return std.mem.indexOf(u8, term, "color") != null or
        std.mem.eql(u8, term, "xterm") or
        std.mem.startsWith(u8, term, "xterm-") or
        std.mem.startsWith(u8, term, "screen") or
        std.mem.startsWith(u8, term, "tmux");
}

/// Formats colored output directly to a writer
pub fn printColored(writer: anytype, text: []const u8, color: []const u8) !void {
    if (supportsColor()) {
        try writer.print("{s}{s}{s}", .{ color, text, ANSI.RESET });
    } else {
        try writer.print("{s}", .{text});
    }
}
