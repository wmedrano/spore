# Zig API Documentation

This document explains how to embed and use Spore in your Zig applications. The Spore library provides a simple API for evaluating Spore code and converting between Zig and Spore values.

## Quick Start

Here's a minimal example of using Spore in a Zig application:

```zig
const std = @import("std");
const spore = @import("spore_lib");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    
    // Initialize the virtual machine
    var vm = try spore.Vm.init(gpa.allocator());
    defer vm.deinit();
    
    // Evaluate Spore code
    const result = try vm.evalStr("(+ 1 2 3)");
    
    // Convert result to Zig type
    const number = try result.to(i64);
    std.debug.print("Result: {}\n", .{number}); // Prints: Result: 6
}
```

## Core Types

### Vm (Virtual Machine)

The `Vm` is the main interface for executing Spore code. It manages memory, execution state, and built-in functions.

```zig
const Vm = @import("spore").Vm;
```

#### Creating and Destroying

```zig
// Create a new VM instance
var vm = try Vm.init(allocator);
defer vm.deinit(); // Always call deinit to free resources
```

#### Evaluating Code

```zig
// Evaluate a string of Spore source code
const result = try vm.evalStr("(+ 1 2 3)");

// Multiple expressions - returns the last result
const last = try vm.evalStr("(def x 10) (def y 20) (+ x y)");
```

#### Error Handling

```zig
const result = vm.evalStr("(+ 1 \"hello\")") catch |err| switch (err) {
    error.TypeError => {
        std.debug.print("Type error: {}\n", .{vm.inspector().errorReport()});
        return;
    },
    error.ParseError => {
        std.debug.print("Parse error: {}\n", .{vm.inspector().errorReport()});
        return;
    },
    else => return err,
};
```

### Val (Value)

`Val` is Spore's universal value type that can hold any Spore data.

```zig
const Val = @import("spore").Val;
```

#### Creating Values

```zig
// Create from Zig primitives
const number = Val.init(42);
const boolean = Val.init(true);
const nil_val = Val.init({});

// Create using the VM builder for complex types
const string_val = try vm.initVal("Hello, World!");
const list_val = try vm.initVal(&[_]Val{ Val.init(1), Val.init(2), Val.init(3) });
```

#### Converting to Zig Types

```zig
const result = try vm.evalStr("42");

// Convert to specific types
const as_int = try result.to(i64);           // 42
const as_float = try result.to(f64);         // 42.0

// Handle conversion errors
const maybe_string = result.to([]const u8) catch |err| switch (err) {
    error.WrongType => {
        std.debug.print("Expected string, got number\n", .{});
        return;
    },
    else => return err,
};
```

#### Supported Conversion Types

| Spore Type | Zig Types | Example |
|------------|-----------|---------|
| Integer | `i64`, `f64` | `42` → `@as(i64, 42)` |
| Float | `f64`, `i64` | `3.14` → `3.14` |
| Boolean | `bool` | `true` → `true` |
| String | `[]const u8` | `"hello"` → `"hello"` |
| Symbol | `Symbol.Interned` | `'my-symbol` → `Symbol.Interned{...}` |
| Nil | `void` | `nil` → `{}` |

## Building Values

The `Builder` provides a convenient way to create Spore values from Zig data.

### Accessing the Builder

```zig
const builder = vm.builder();
```

### Creating Values

```zig
// Primitives
const num = try builder.init(42);
const str = try builder.init("Hello");
const arr = try builder.init(&[_]i64{ 1, 2, 3 });

// Complex structures
const pair_val = try builder.init(spore.Pair.init(Val.init(1), Val.init(2)));
```

## Inspecting Values

The `Inspector` helps with debugging and value introspection.

### Getting an Inspector

```zig
const inspector = vm.inspector();
```

### Pretty Printing

```zig
const result = try vm.evalStr("(list 1 2 3)");
std.debug.print("Result: {}\n", .{inspector.pretty(result)});
// Output: Result: (1 2 3)

// Print multiple values
const args = &[_]Val{ Val.init(1), Val.init(2), Val.init(3) };
std.debug.print("Args: {}\n", .{inspector.prettySlice(args)});
// Output: Args: 1 2 3
```

### Error Reporting

```zig
const result = vm.evalStr("(undefined-function)") catch {
    std.debug.print("Error: {}\n", .{inspector.errorReport()});
    return;
};
```

### Type Conversion with Inspector

The inspector can convert values that require VM access:

```zig
// Convert a list to an iterator
const list_result = try vm.evalStr("(list 1 2 3 4)");
var list_iter = try inspector.to(spore.Pair.ListIter, list_result);

while (try list_iter.next(&vm)) |item| {
    const num = try item.to(i64);
    std.debug.print("Item: {}\n", .{num});
}
```

## Working with Specific Types

### Strings

```zig
// Create string
const str_val = try vm.initVal("Hello, Spore!");

// Get string content
const content = try str_val.to([]const u8); // Requires VM's heap to be valid
```

### Lists and Pairs

```zig
// Create a list
const list_val = try vm.evalStr("(list 1 2 3)");

// Iterate over list
var iter = try vm.inspector().to(spore.Pair.ListIter, list_val);
while (try iter.next(&vm)) |item| {
    const num = try item.to(i64);
    std.debug.print("Item: {}\n", .{num});
}

// Create a pair
const pair_val = try vm.evalStr("(pair 10 20)");
const first = try vm.evalStr("(first my-pair)");  // After setting my-pair
const second = try vm.evalStr("(second my-pair)");
```

### Functions

```zig
// Define a function in Spore
_ = try vm.evalStr("(defun double (x) (* x 2))");

// Call the function
const result = try vm.evalStr("(double 21)");
const answer = try result.to(i64); // 42
```

## Memory Management

Spore manages memory automatically through garbage collection, but you should be aware of object lifetimes.

### Automatic Garbage Collection

```zig
// Objects are automatically collected when unreachable
_ = try vm.evalStr("(def temp-list (list 1 2 3))");
_ = try vm.evalStr("(def temp-list nil)"); // Original list becomes unreachable

// Manually trigger garbage collection (optional)
try vm.garbageCollect();
```

### Object Lifetime

```zig
// Values are valid as long as the VM exists
var vm = try Vm.init(allocator);
defer vm.deinit();

const val = try vm.evalStr("\"Hello\"");
const str = try val.to([]const u8);
// str is valid until vm.deinit() is called

// Don't do this:
// vm.deinit();
// const invalid = str; // str may now point to freed memory
```

## Advanced Usage

### Custom Native Functions

You can register custom Zig functions to be callable from Spore:

```zig
const spore = @import("spore");

fn addOneImpl(vm: *spore.Vm) spore.errors.Error!spore.Val {
    const args = vm.execution_context.localStack();
    if (args.len != 1) {
        return vm.builder().addError(spore.errors.DetailedError{
            .wrong_arity = .{ .function = "add-one", .want = 1, .got = @intCast(args.len) }
        });
    }
    
    const num = args[0].to(i64) catch {
        return vm.builder().addError(spore.errors.DetailedError{
            .wrong_type = .{ .want = "integer", .got = args[0] }
        });
    };
    
    return spore.Val.init(num + 1);
}

const add_one = spore.NativeFunction{
    .name = "add-one",
    .docstring = "Adds 1 to the given number",
    .ptr = addOneImpl,
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    
    var vm = try spore.Vm.init(gpa.allocator());
    defer vm.deinit();
    
    // Register the custom function
    try add_one.register(&vm);
    
    // Use it in Spore code
    const result = try vm.evalStr("(add-one 41)");
    const answer = try result.to(i64); // 42
}
```

### Error Handling Patterns

```zig
fn safeEval(vm: *spore.Vm, source: []const u8) !?i64 {
    const result = vm.evalStr(source) catch |err| switch (err) {
        error.ParseError, error.RuntimeError => {
            std.log.warn("Spore evaluation failed: {}", .{vm.inspector().errorReport()});
            return null;
        },
        else => return err,
    };
    
    return result.to(i64) catch |err| switch (err) {
        error.WrongType => {
            std.log.warn("Expected integer result, got: {}", .{vm.inspector().pretty(result)});
            return null;
        },
        else => return err,
    };
}
```

### Performance Considerations

```zig
// Reuse VM instances when possible
var vm = try spore.Vm.init(allocator);
defer vm.deinit();

for (expressions) |expr| {
    // Reusing the same VM is more efficient than creating new ones
    const result = try vm.evalStr(expr);
    // Process result...
}

// Reset call stack between unrelated evaluations if needed
vm.execution_context.resetCalls();
```

## Complete Example

```zig
const std = @import("std");
const spore = @import("spore");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    
    var vm = try spore.Vm.init(gpa.allocator());
    defer vm.deinit();
    
    // Define some variables and functions
    _ = try vm.evalStr(
        \\(def numbers (list 1 2 3 4 5))
        \\(defun sum-list (lst)
        \\  (if (empty? lst)
        \\      0
        \\      (+ (first lst) (sum-list (second lst)))))
    );
    
    // Calculate the sum
    const result = try vm.evalStr("(sum-list numbers)");
    const sum = try result.to(i64);
    
    std.debug.print("Sum of [1,2,3,4,5] = {}\n", .{sum}); // Sum = 15
    
    // Error handling example
    const safe_result = vm.evalStr("(/ 10 0)") catch |err| switch (err) {
        error.RuntimeError => {
            std.debug.print("Runtime error caught: {}\n", .{vm.inspector().errorReport()});
            return;
        },
        else => return err,
    };
    
    std.debug.print("Safe result: {}\n", .{vm.inspector().pretty(safe_result)});
}
```

## API Reference Summary

### Core Functions

- `Vm.init(allocator)` - Create VM instance
- `Vm.deinit()` - Destroy VM and free resources
- `vm.evalStr(source)` - Evaluate Spore source code
- `vm.initVal(zig_value)` - Convert Zig value to Spore Val
- `vm.builder()` - Get builder for creating values
- `vm.inspector()` - Get inspector for debugging

### Value Operations

- `Val.init(primitive)` - Create Val from Zig primitive
- `val.to(Type)` - Convert Val to Zig type
- `inspector.pretty(val)` - Pretty print a value
- `inspector.prettySlice(vals)` - Pretty print multiple values

### Error Handling

- `inspector.errorReport()` - Get detailed error information
- `inspector.stackTrace()` - Get execution stack trace

### Memory Management

- `vm.garbageCollect()` - Manually trigger garbage collection
- Objects are automatically freed when unreachable