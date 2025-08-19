# Spore Lisp Interpreter

## Project Overview

Spore is a Lisp-like interpreter built in Zig. It supports S-expressions, dynamic typing, garbage collection, and standard Lisp operations. The interpreter includes a virtual machine with bytecode compilation, memory management, and a comprehensive set of built-in functions.

## Build & Test Commands

```bash
# Build the project (always use this instead of zig test)
zig build

# Run all unit tests with summary
zig build test --summary all

# Generate test coverage report (requires kcov)
zig build coverage

# Generate documentation
zig build doc

# Run the interpreter
./zig-out/bin/spore < examples/hello-world.sp
```

## Project Structure

```
src/
├── main.zig              # CLI entry point
├── root.zig              # Library root
├── Vm.zig                # Virtual machine core
├── Val.zig               # Dynamic value system
├── Compiler.zig          # Bytecode compiler
├── Reader.zig            # S-expression parser
├── Tokenizer.zig         # Lexical analyzer
├── BytecodeFunction.zig  # Compiled functions
├── ExecutionContext.zig  # Runtime context
├── Heap.zig              # Memory management
├── GarbageCollector.zig  # GC implementation
├── StringInterner.zig    # String deduplication
├── builtins/             # Built-in functions
│   ├── arithmetic.zig
│   ├── control_flow.zig
│   ├── data_structures.zig
│   ├── io.zig
│   ├── type_predicates.zig
│   └── utility.zig
├── errors.zig            # Error definitions
└── terminal/             # Terminal interface
    ├── Readline.zig
    └── Color.zig
```

## Code Quality Standards

### Zig Code Style

- Use PascalCase for types: `Val`, `BytecodeFunction`, `ExecutionContext`
- Use snake_case for functions and variables: `init()`, `deinit()`, `next_token`
- Use SCREAMING_SNAKE_CASE for constants: `MAX_STACK_SIZE`
- Prefer explicit types over `var` when clarity benefits: `const result: Error!Val`
- Use `@This()` pattern for self-referential types
- Always provide `init()` and `deinit()` methods for resource-managing types
- Use `defer` for cleanup to ensure resources are freed
- Prefer stack allocation over heap when possible
- Use handles (`Handle(T)`) for heap objects to enable garbage collection

### Memory Management

- Always use `testing.allocator` in tests
- Implement proper `deinit()` methods that free all allocated resources
- Use `defer vm.deinit()` pattern in tests and functions
- Leverage the garbage collector for Val objects rather than manual memory management
- Use object pools for frequently allocated/deallocated objects
- Test allocation failure scenarios with `testing.FailingAllocator`

### Error Handling

- Use Zig's error unions: `Error!Val`, `DetailedError!void`
- Provide detailed error information via `DetailedError` union
- Always handle or propagate errors - no silent failures
- Use `try` for error propagation, `catch` only when handling locally
- Test both success and error paths in all functions
- Include context in error messages (function name, expected vs actual values)

### Testing Philosophy

- **Test behaviors, not implementation details**
- **One test per behavior** - use pattern: "when X then Y"
- Use descriptive test names: `"when tokenizing empty string then no tokens returned"`
- Structure tests as Arrange-Act-Assert
- Always test error conditions alongside success paths
- Use realistic integration over mocking when possible
- Prefer `testing.expectEqual()` > `testing.expectEqualDeep()` > `testing.expectFmt()`

### Documentation

- Use `//!` for module documentation at the top of files
- Use `///` for public function documentation
- Document complex algorithms and data structures
- Include examples in documentation for non-trivial APIs
- Keep comments concise and focused on "why" not "what"

## Language Implementation Details

### Value System (`Val.zig`)
- Tagged union with optimized representation
- Supports: boolean, nil, int, float, symbol, pair, string, functions
- Uses interned symbols and handles for memory efficiency
- Implements garbage collection integration

### Virtual Machine (`Vm.zig`)
- Stack-based execution model
- Bytecode compilation and interpretation
- Integrated garbage collector
- Support for native and user-defined functions

### Built-in Functions
- Organized by category in `builtins/` directory
- Each built-in validates arity and argument types
- Consistent error handling across all built-ins
- Performance-critical operations implemented natively

## Spore Language Features

The interpreter supports:
- S-expression syntax: `(+ 1 2 3)`
- Dynamic typing with type predicates
- First-class functions and lambdas
- Lexical scoping with `let*`
- Control flow: `if`, `for`, `return`
- List manipulation: `list`, `pair`, `first`, `second`
- I/O operations: `print`, `println`
- Standard arithmetic and comparison operators

## Development Workflow

1. Write failing tests first when adding features
2. Implement the minimal code to make tests pass
3. Refactor for clarity and performance
4. Ensure all tests pass with `zig build test --summary all`
5. Check test coverage with `zig build coverage`
6. Generate and review documentation with `zig build doc`

## Performance Considerations

- The VM uses bytecode compilation for performance
- Garbage collection runs automatically when heap pressure increases
- String interning reduces memory usage for symbols
- Object pools minimize allocation overhead
- Stack-based execution avoids excessive heap allocation

## Integration with Emacs

The project includes Emacs integration via `tools/spore-mode.el` for org-mode source blocks, enabling interactive development and documentation.