# Spore - Lisp-like Language in Zig

Spore is a dynamically-typed Lisp-like scripting language implemented in Zig. It features S-expression syntax, garbage collection, and a REPL.

## Project Structure

- `src/` - Core language implementation
  - `main.zig` - CLI entry point and REPL
  - `root.zig` - Library root
  - `Vm.zig` - Virtual machine execution engine
  - `Compiler.zig` - Bytecode compiler
  - `Reader.zig` - S-expression parser
  - `Tokenizer.zig` - Lexical analysis
  - `Val.zig` - Value representation system
  - `Heap.zig` - Memory management
  - `GarbageCollector.zig` - Automatic memory cleanup
  - `ExecutionContext.zig` - Runtime execution state
  - `BytecodeFunction.zig` - Compiled function representation
  - `instruction.zig` - Bytecode instruction set
  - `LexicalScope.zig` - Variable scoping
  - `StringInterner.zig` - String deduplication
  - `Symbol.zig`, `String.zig`, `Pair.zig` - Core data types
  - `NativeFunction.zig` - Built-in function wrapper
  - `Builder.zig` - Bytecode generation helper
  - `Inspector.zig` - Runtime introspection
  - `PrettyPrinter.zig` - Value formatting
  - `errors.zig` - Error type definitions
  - `object_pool.zig` - Object pooling utilities
  - `builtins.zig` - Built-in function registry
  - `builtins/` - Built-in function implementations
    - `arithmetic.zig` - Mathematical operations
    - `control_flow.zig` - Flow control constructs
    - `conversion.zig` - Type conversion functions
    - `data_structures.zig` - List and data manipulation
    - `io.zig` - Input/output operations
    - `type_predicates.zig` - Type checking functions
    - `utility.zig` - Utility functions
  - `terminal/` - Terminal interface utilities
    - `Color.zig` - Color output support
    - `Readline.zig` - Line editing functionality
- `examples/` - Example Spore programs (`.sp` files)
- `tools/spore-mode.el` - Emacs integration

## Build System

- **Build**: `zig build` - Compiles the executable
- **Test**: `zig build test --summary all` - Runs unit tests
- **Coverage**: `zig build coverage` - Generates test coverage with kcov
- **Docs**: `zig build doc` - Generates API documentation

## Language Characteristics

- **Syntax**: Lisp S-expressions `(+ 1 2)`
- **Types**: Numbers, booleans, strings, nil, symbols, lists, functions
- **Comments**: `;;` for single-line comments
- **Variables**: `def` (global), `let*` (local)
- **Functions**: `defun` macro or `function` keyword for lambdas
- **Control Flow**: `if`, `for` loops, `return` for early exit
- **Built-ins**: Arithmetic, comparison, logical ops, list manipulation, type predicates

## Code Conventions

- Use existing error types from `errors.zig`
- Follow Zig naming conventions (snake_case for functions, PascalCase for types)
- Add unit tests for new functionality in same file as implementation
- Test behaviors, not methods - format: "action expectation"
- Always verify with `zig build` and `zig build test --summary all`

## Testing Guidelines

- Unit tests should test behaviors, not methods
- Format: "action expectation" (e.g., "compile_simple_expression returns_bytecode")
- One unit test per behavior
- Always use `zig build test --summary all` instead of `zig test`

## File Extensions

- `.sp` - Spore source files
- `.zig` - Zig implementation files

## Running Spore

```sh
# From file
spore < program.sp

# Interactive REPL
spore
# (type program and Ctrl+D)

# With Emacs org-mode
# Load tools/spore-mode.el and use #+BEGIN_SRC spore blocks
```
