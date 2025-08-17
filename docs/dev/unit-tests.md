# Unit Testing Guide

This guide explains how to write unit tests for the Spore project, which uses
Zig's built-in testing framework.

## Running Tests

```bash
# Run all unit tests
zig build test

# Generate test coverage report
zig build coverage
```

## Basic Test Structure

Zig tests are written using the `test` keyword followed by a descriptive string:

```zig
const std = @import("std");
const testing = std.testing;

test "descriptive test name" {
    // Test implementation
}
```

## Test Setup and Teardown

### Memory Management

Always use `testing.allocator` for tests and properly clean up resources:

```zig
test "resource management example" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit(); // Always defer cleanup

    // Test implementation using vm
}
```

Use `testing.FailingAllocator` to simulate out-of-memory conditions:

```zig
test "handles allocation failure" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    var failing_allocator = testing.FailingAllocator.init(testing.allocator, .{ .fail_index = 0 });
    vm.heap.allocator = failing_allocator.allocator();

    try testing.expectError(
        errors.Error.OutOfMemory,
        vm.builder().someOperation(),
    );
}
```

## Common Testing Patterns

### 1. Simple Value Testing

Use `testing.expectEqual` for basic value comparisons:

```zig
test "Val.to i64" {
    const val = Val.init(42);
    try testing.expectEqual(@as(i64, 42), val.to(i64));
}
```

### 2. Deep Structure Testing

Use `testing.expectEqualDeep` for complex structures:

```zig
test "empty string has no tokens" {
    var tokenizer = Tokenizer.init("");
    try testing.expectEqualDeep(null, tokenizer.next());
}
```

### 3. String Comparison

Use `testing.expectEqualStrings` for string comparisons:

```zig
test "complex s-expression returns each token" {
    var tokenizer = Tokenizer.init("(plus one two)");
    try testing.expectEqualStrings("(", tokenizer.nextStr().?);
    try testing.expectEqualStrings("plus", tokenizer.nextStr().?);
}
```

### 4. Error Testing

Use `testing.expectError` to verify error conditions:

```zig
test "Builder.init: handles out of memory when converting Val slice" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    // Set up a failing allocator to trigger OOM
    var failing_allocator = testing.FailingAllocator.init(testing.allocator, .{ .fail_index = 0 });
    vm.heap.allocator = failing_allocator.allocator();

    const vals = [_]Val{ Val.init(1), Val.init(2) };
    try testing.expectError(
        errors.Error.OutOfMemory,
        vm.builder().init(@as([]const Val, &vals)),
    );
}
```

### 5. Custom Output Formatting

Use `testing.expectFmt` as a last resort when other methods fail. It's primarily
used for testing formatted output of complex values:

```zig
test "Builder.init: converts nil" {
    var vm = try Vm.init(testing.allocator);
    defer vm.deinit();

    const result = try vm.builder().init({});
    try testing.expectFmt(
        "nil",
        "{}",
        .{vm.inspector().pretty(result)},
    );
}
```

Note: Prefer `expectEqual`, `expectEqualDeep`, or `expectEqualStrings` when possible, as they provide clearer test failures and are more direct.

## Best Practices

1. **One concept per test**: Each test should verify one specific behavior or condition.

2. **Descriptive names**: Test names should clearly describe what is being tested.

3. **Arrange-Act-Assert**: Structure tests with clear setup, action, and verification phases.

4. **Error path testing**: Always test both success and failure scenarios.

5. **Resource cleanup**: Use `defer` for cleanup to ensure resources are freed even if tests fail.

6. **Edge cases**: Test boundary conditions, empty inputs, and error states.

7. **Prefer realistic tests**: Use real components over mocks and fakes for more realistic testing.

## Common Testing Utilities

The project uses these standard testing utilities:

- `testing.expectEqual()` - Compare primitive values
- `testing.expectEqualDeep()` - Compare complex structures
- `testing.expectEqualStrings()` - Compare string values
- `testing.expectError()` - Verify error conditions
- `testing.expectFmt()` - Test formatted output
- `testing.allocator` - Standard test allocator
- `testing.FailingAllocator` - Simulate allocation failures



## Test Naming Conventions

Tests should focus on behaviors using the pattern "when x then y":

- Example: `"when tokenizing empty string then no tokens returned"`
- Example: `"when converting nil value then result is nil"`
- Example: `"when allocator fails then out of memory error returned"`
- Example: `"when parsing quoted string then single string token created"`
