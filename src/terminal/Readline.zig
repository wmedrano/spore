//! A simple readline implementation for text input editing.
const std = @import("std");

const Color = @import("Color.zig");

/// Terminal control constants
const TerminalCodes = struct {
    const CTRL_C = 3;
    const CTRL_D = 4;
    const CTRL_A = 1;
    const CTRL_E = 5;
    const ESC = 27;
    const BACKSPACE = 127;
    const DELETE = 8;
    const CARRIAGE_RETURN = '\r';
    const NEWLINE = '\n';

    // ANSI escape sequences
    const CLEAR_LINE = "\r\x1b[K";
    const CURSOR_LEFT_FMT = "\x1b[{d}D";
};

/// ASCII character range constants
const ASCII = struct {
    const PRINTABLE_START = 32;
    const PRINTABLE_END = 127;
};

const Readline = @This();

/// The text buffer storing the current line being edited.
buffer: std.ArrayList(u8),
/// The current cursor position within the buffer.
cursor: usize = 0,

/// Initializes a new `Readline` instance with the given allocator.
pub fn init(allocator: std.mem.Allocator) Readline {
    return Readline{
        .buffer = std.ArrayList(u8).init(allocator),
        .cursor = 0,
    };
}

/// Deinitializes the `Readline`, freeing its allocated memory.
pub fn deinit(self: *Readline) void {
    self.buffer.deinit();
}

/// Clears the buffer and resets the cursor to the beginning.
pub fn clear(self: *Readline) void {
    self.buffer.clearRetainingCapacity();
    self.cursor = 0;
}

/// Inserts a character at the current cursor position and advances the cursor.
pub fn insertChar(self: *Readline, ch: u8) !void {
    try self.buffer.insert(self.cursor, ch);
    self.cursor += 1;
}

/// Deletes the character before the cursor (backspace).
/// Returns `true` if a character was deleted, `false` otherwise.
pub fn deleteChar(self: *Readline) !bool {
    if (self.cursor == 0 or self.buffer.items.len == 0) return false;

    _ = self.buffer.orderedRemove(self.cursor - 1);
    self.cursor -= 1;
    return true;
}

/// Moves the cursor one position to the left.
/// Returns `true` if the cursor moved, `false` if already at the beginning.
pub fn moveCursorLeft(self: *Readline) bool {
    if (self.cursor > 0) {
        self.cursor -= 1;
        return true;
    }
    return false;
}

/// Moves the cursor one position to the right.
/// Returns `true` if the cursor moved, `false` if already at the end.
pub fn moveCursorRight(self: *Readline) bool {
    if (self.cursor < self.buffer.items.len) {
        self.cursor += 1;
        return true;
    }
    return false;
}

/// Moves the cursor to the beginning of the line.
/// Returns `true` if the cursor moved, `false` if already at the beginning.
pub fn moveCursorToStart(self: *Readline) bool {
    if (self.cursor > 0) {
        self.cursor = 0;
        return true;
    }
    return false;
}

/// Moves the cursor to the end of the line.
/// Returns `true` if the cursor moved, `false` if already at the end.
pub fn moveCursorToEnd(self: *Readline) bool {
    if (self.cursor < self.buffer.items.len) {
        self.cursor = self.buffer.items.len;
        return true;
    }
    return false;
}

/// Returns the current line as a read-only slice.
pub fn getLine(self: *Readline) []const u8 {
    return self.buffer.items;
}

/// Sets the terminal to raw mode or restores normal mode.
fn setRawMode(enable: bool) !void {
    const stdin_fd = std.posix.STDIN_FILENO;
    var termios = try std.posix.tcgetattr(stdin_fd);

    termios.lflag.ICANON = !enable;
    termios.lflag.ECHO = !enable;
    termios.lflag.ISIG = !enable;
    termios.iflag.IXON = !enable;
    termios.iflag.ICRNL = !enable;

    try std.posix.tcsetattr(stdin_fd, std.posix.TCSA.FLUSH, termios);
}

/// Enables raw mode for terminal input.
pub fn enableRawMode() !void {
    try setRawMode(true);
}

/// Disables raw mode, restoring normal terminal behavior.
pub fn disableRawMode() !void {
    try setRawMode(false);
}

/// Refreshes the display line with the current buffer contents.
fn refreshLine(stdout: anytype, prompt: []const u8, editor: *Readline) !void {
    refreshLineColored(stdout, prompt, Color.Theme.default.prompt, editor) catch |err| return err;
}

/// Refreshes the display line with colored prompt support.
fn refreshLineColored(stdout: anytype, prompt: []const u8, prompt_color: []const u8, editor: *Readline) !void {
    _ = try stdout.write(TerminalCodes.CLEAR_LINE);
    try Color.printColored(stdout, prompt, prompt_color);
    try stdout.print("{s}", .{editor.getLine()});
    if (editor.cursor < editor.buffer.items.len) {
        const moves = editor.buffer.items.len - editor.cursor;
        try stdout.print(TerminalCodes.CURSOR_LEFT_FMT, .{moves});
    }
}

/// Reads a line of input with editing capabilities.
/// Returns the entered line or null if EOF/Ctrl+C/Ctrl+D.
pub fn readLine(self: *Readline, prompt: []const u8) !?[]u8 {
    return self.readLineColored(prompt, Color.Theme.default.prompt);
}

/// Reads a line of input with colored prompt support.
/// Returns the entered line or null if EOF/Ctrl+C/Ctrl+D.
pub fn readLineColored(self: *Readline, prompt: []const u8, prompt_color: []const u8) !?[]u8 {
    const stdin = std.io.getStdIn().reader();
    const stdout = std.io.getStdOut().writer();
    const allocator = self.buffer.allocator;

    try enableRawMode();
    defer disableRawMode() catch |err| {
        std.log.warn("Failed to disable raw mode: {}", .{err});
    };

    self.clear();
    try Color.printColored(stdout, prompt, prompt_color);

    while (true) {
        const ch = stdin.readByte() catch |err| switch (err) {
            error.EndOfStream => return null,
            else => return err,
        };

        switch (ch) {
            TerminalCodes.CARRIAGE_RETURN, TerminalCodes.NEWLINE => {
                _ = try stdout.write("\r\n");
                const line = try allocator.dupe(u8, self.getLine());
                return line;
            },
            TerminalCodes.CTRL_C => {
                _ = try stdout.write("^C\r\n");
                return null;
            },
            TerminalCodes.CTRL_D => {
                if (self.buffer.items.len == 0) {
                    _ = try stdout.write("\r\n");
                    return null;
                }
            },
            TerminalCodes.CTRL_A => {
                if (self.moveCursorToStart()) {
                    try refreshLineColored(stdout, prompt, prompt_color, self);
                }
            },
            TerminalCodes.CTRL_E => {
                if (self.moveCursorToEnd()) {
                    try refreshLineColored(stdout, prompt, prompt_color, self);
                }
            },
            TerminalCodes.BACKSPACE, TerminalCodes.DELETE => {
                if (try self.deleteChar()) {
                    try refreshLineColored(stdout, prompt, prompt_color, self);
                }
            },
            TerminalCodes.ESC => {
                const seq1 = stdin.readByte() catch |err| switch (err) {
                    error.EndOfStream => continue,
                    else => return err,
                };
                if (seq1 == '[') {
                    const seq2 = stdin.readByte() catch |err| switch (err) {
                        error.EndOfStream => continue,
                        else => return err,
                    };
                    switch (seq2) {
                        'D' => {
                            if (self.moveCursorLeft()) {
                                try refreshLineColored(stdout, prompt, prompt_color, self);
                            }
                        },
                        'C' => {
                            if (self.moveCursorRight()) {
                                try refreshLineColored(stdout, prompt, prompt_color, self);
                            }
                        },
                        else => {},
                    }
                }
            },
            else => {
                if (ch >= ASCII.PRINTABLE_START and ch < ASCII.PRINTABLE_END) {
                    self.insertChar(ch) catch |err| switch (err) {
                        error.OutOfMemory => return err,
                    };
                    try refreshLineColored(stdout, prompt, prompt_color, self);
                }
            },
        }
    }
}

pub fn printSuccess(writer: anytype, text: []const u8) !void {
    try Color.printColored(writer, text, Color.Theme.default.success);
}

pub fn printError(writer: anytype, text: []const u8) !void {
    try Color.printColored(writer, text, Color.Theme.default.err);
}

pub fn printInfo(writer: anytype, text: []const u8) !void {
    try Color.printColored(writer, text, Color.Theme.default.info);
}

pub fn printSpecial(writer: anytype, text: []const u8) !void {
    try Color.printColored(writer, text, Color.Theme.default.special);
}
