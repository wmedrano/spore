# Spore Documentation

Welcome to the Spore documentation! Spore is a dynamically-typed Lisp-like scripting language implemented in Zig.

## Documentation Structure

### For Users

- **[Getting Started](getting-started.md)** - Learn how to install, run, and write your first Spore programs
- **[Language Reference](language-reference.md)** - Complete reference for Spore syntax, built-in functions, and language features

### For Developers

- **[Zig API Documentation](zig-api.md)** - How to embed Spore in your Zig applications, work with the VM, and convert between Zig and Spore values

## Quick Links

### Installation

```bash
git clone <repository-url>
cd spore
zig build
```

### Hello World

```lisp
(println "Hello, World!")
```

### REPL Usage

```bash
spore
spore> (+ 1 2 3)
=> 6
spore> exit
Goodbye!
```

## Language Features

- **S-expression syntax** - Lisp-style parenthesized expressions
- **Dynamic typing** - Numbers, booleans, strings, symbols, lists, functions
- **Garbage collection** - Automatic memory management  
- **Interactive REPL** - Real-time code evaluation and testing
- **Zig embedding** - Easy integration into Zig applications

## Example Programs

### Factorial Function
```lisp
(defun factorial (n)
  (if (= n 0)
      1
      (* n (factorial (- n 1)))))

(factorial 5)  ;; => 120
```

### List Processing
```lisp
(def numbers (list 1 2 3 4 5))
(defun double (x) (* x 2))

(for (n numbers)
  (println (double n)))
```

### Fibonacci Sequence
```lisp
(defun fib (n)
  (if (< n 2)
      n
      (+ (fib (- n 1)) (fib (- n 2)))))

(for (i (range 0 10))
  (println (fib i)))
```

## Architecture

Spore consists of several key components:

- **Reader** - Parses S-expressions into AST
- **Compiler** - Transforms AST into bytecode  
- **VM** - Executes bytecode with stack-based virtual machine
- **Heap** - Manages dynamic objects with garbage collection
- **Built-ins** - Provides arithmetic, I/O, and data structure operations

## Contributing

The codebase follows these conventions:

- Use existing error types from `errors.zig`
- Follow Zig naming conventions (snake_case for functions, PascalCase for types)  
- Add unit tests for new functionality
- Test behaviors, not methods - format: "action expectation"
- Always verify with `zig build` and `zig build test --summary all`

## File Extensions

- `.sp` - Spore source files
- `.zig` - Zig implementation files

## Getting Help

- Check the documentation in this directory
- Run `(help)` in the REPL for basic commands
- Explore the `examples/` directory for sample programs
- Use `tools/spore-mode.el` for Emacs integration