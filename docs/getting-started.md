# Getting Started with Spore

Spore is a dynamically-typed Lisp-like scripting language implemented in Zig. It features S-expression syntax, garbage collection, and an interactive REPL.

## Installation

To build Spore from source, you'll need Zig installed on your system.

```bash
# Clone the repository
git clone <repository-url>
cd spore

# Build the executable
zig build

# The executable will be available as zig-out/bin/spore
```

## Running Spore

### Interactive REPL

Start the interactive REPL by running the executable without arguments:

```bash
spore
```

You'll see a prompt where you can enter Spore expressions:

```
Spore REPL - Enter expressions to evaluate ((help) for commands)
spore> (+ 1 2 3)
=> 6
spore> (println "Hello, World!")
Hello, World!
=> nil
spore> exit
Goodbye!
```

### Running Files

Execute a Spore program from a file:

```bash
spore < program.sp
```

## Basic Syntax

Spore uses S-expression (symbolic expression) syntax, where everything is represented as nested lists in parentheses.

### Comments

Use `;;` for single-line comments:

```lisp
;; This is a comment
(+ 1 2) ;; Add 1 and 2
```

### Basic Data Types

```lisp
;; Numbers
42           ;; Integer
3.14         ;; Float

;; Booleans
true
false

;; Strings
"Hello, World!"

;; Nil (empty/null value)
nil

;; Symbols
my-variable
+
println
```

### Lists and Pairs

```lisp
;; Create a list
(list 1 2 3 4)

;; Create a pair (two-element structure)
(pair 1 2)

;; Access pair elements
(first (pair 1 2))  ;; => 1
(second (pair 1 2)) ;; => 2

;; Check if list is empty
(empty? (list))     ;; => true
(empty? (list 1))   ;; => false
```

## Basic Operations

### Arithmetic

```lisp
(+ 1 2 3)           ;; Addition: 6
(- 10 3)            ;; Subtraction: 7
(* 2 3 4)           ;; Multiplication: 24
(/ 10 2)            ;; Division: 5
(mod 10 3)          ;; Modulo: 1
```

### Comparison

```lisp
(= 1 1)             ;; Equality: true
(= 1 2)             ;; Equality: false
```

### Input/Output

```lisp
(print "Hello")     ;; Print without newline
(println "World!")  ;; Print with newline
```

## Variables and Functions

### Global Variables

Use `def` to define global variables:

```lisp
(def my-number 42)
(def greeting "Hello, Spore!")
(println my-number)  ;; => 42
```

### Local Variables

Use `let*` for local variable bindings:

```lisp
(let* ((x 10)
       (y 20)
       (sum (+ x y)))
  (println sum))  ;; => 30
```

### Functions

Define functions using `defun`:

```lisp
(defun square (x)
  (* x x))

(square 5)  ;; => 25
```

You can also create anonymous functions with `function`:

```lisp
((function (x y) (+ x y)) 3 4)  ;; => 7
```

## Control Flow

### Conditionals

Use `if` for conditional execution:

```lisp
(if (= 1 1)
    (println "True!")
    (println "False!"))  ;; Prints "True!"
```

### Logical Operations

```lisp
(and true false)    ;; => false
(or false true)     ;; => true
```

### Loops

Use `for` to iterate over lists or ranges:

```lisp
;; Iterate over a list
(for (x (list 1 2 3))
  (println x))

;; Iterate over a range
(for (i (range 1 5))  ;; Numbers 1 through 4
  (println i))
```

## Type Predicates

Check the type of values:

```lisp
(integer? 42)       ;; => true
(float? 3.14)       ;; => true
(boolean? true)     ;; => true
(string? "hello")   ;; => true
(nil? nil)          ;; => true
(symbol? 'my-sym)   ;; => true
(pair? (pair 1 2))  ;; => true
(function? square)  ;; => true
```

## Example Programs

### Hello World

```lisp
(println "Hello World!")
(println "Bye World!")
```

### Calculate Sum of Squares

```lisp
;; Define a global variable
(def squared-sum 0)

;; Define a function
(defun square (number) 
  (* number number))

;; Iterate over a list and accumulate
(for (x (list 1 2 3 4))
  (let* ((squared (square x))
         (new-sum (+ squared squared-sum)))
    (def squared-sum new-sum)))

(println squared-sum)  ;; => 30
```

### Fibonacci Sequence

```lisp
(defun fibonacci (n)
  (if (= n 0)
      0
      (if (= n 1)
          1
          (+ (fibonacci (- n 1)) 
             (fibonacci (- n 2))))))

(for (i (range 0 10))
  (println (fibonacci i)))
```

## Error Handling

When an error occurs, Spore will display an error message. In the REPL, you can continue after an error:

```
spore> (+ 1 "hello")
Error: Type error: Expected number, got string
spore> (+ 1 2)
=> 3
```

## Next Steps

- Read the [Language Reference](language-reference.md) for detailed information about all language features
- Explore the [Zig API Documentation](zig-api.md) to learn how to embed Spore in your Zig applications
- Check out the examples in the `examples/` directory