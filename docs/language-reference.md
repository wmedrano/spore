# Spore Language Reference

This document provides a comprehensive reference for the Spore programming language, including all built-in functions, syntax rules, and language features.

## Syntax

Spore uses S-expression (symbolic expression) syntax, where programs are written as nested lists.

### Basic Structure

```lisp
(function-name arg1 arg2 arg3)
```

### Comments

Single-line comments start with `;;`:

```lisp
;; This is a comment
(+ 1 2) ;; This is also a comment
```

## Data Types

### Numbers

Spore supports both integers and floating-point numbers:

```lisp
42        ;; Integer
-17       ;; Negative integer
3.14      ;; Float
-2.5      ;; Negative float
```

### Booleans

```lisp
true      ;; True value
false     ;; False value
```

### Strings

String literals are enclosed in double quotes:

```lisp
"Hello, World!"
"Multi-line strings
are supported"
""        ;; Empty string
```

### Nil

The `nil` value represents empty/null:

```lisp
nil
```

### Symbols

Symbols are identifiers used for variables and function names:

```lisp
my-variable
+
println
some-function-name
```

### Lists and Pairs

Lists are collections of values, and pairs are two-element structures:

```lisp
(list 1 2 3 4)        ;; List with 4 elements
(pair 10 20)          ;; Pair with 2 elements
nil                   ;; Empty list (same as nil)
```

## Variables

### Global Variables

Use `def` to define global variables:

```lisp
(def variable-name value)

;; Examples:
(def pi 3.14159)
(def greeting "Hello")
(def my-list (list 1 2 3))
```

### Local Variables

Use `let*` for local variable bindings within a scope:

```lisp
(let* ((var1 value1)
       (var2 value2)
       (var3 (+ var1 var2)))
  ;; Body - use variables here
  (println var3))
```

The `*` in `let*` indicates sequential binding, where later bindings can reference earlier ones.

## Functions

### Defining Functions

Use `defun` to define named functions:

```lisp
(defun function-name (param1 param2)
  body)

;; Examples:
(defun square (x)
  (* x x))

(defun add-three (a b c)
  (+ a b c))
```

### Anonymous Functions

Use `function` to create anonymous functions:

```lisp
(function (param1 param2) body)

;; Example:
((function (x y) (+ x y)) 5 10)  ;; => 15
```

### Function Calls

```lisp
(function-name arg1 arg2 ...)

;; Examples:
(square 5)          ;; => 25
(+ 1 2 3 4)         ;; => 10
(println "Hello")   ;; Prints "Hello"
```

## Control Flow

### Conditionals

Use `if` for conditional execution:

```lisp
(if condition true-expr false-expr)

;; Examples:
(if (= x 0)
    "Zero"
    "Non-zero")

(if (> age 18)
    (println "Adult")
    (println "Minor"))
```

### Logical Operations

#### `and`

Evaluates arguments left to right, returning the first falsy value or the last value:

```lisp
(and expr1 expr2 ...)

;; Examples:
(and true true)          ;; => true
(and true false)         ;; => false
(and 1 2 3)             ;; => 3
(and false "ignored")    ;; => false (short-circuits)
```

#### `or`

Evaluates arguments left to right, returning the first truthy value or the last value:

```lisp
(or expr1 expr2 ...)

;; Examples:
(or false true)          ;; => true
(or false false)         ;; => false
(or)                     ;; => nil
(or 1 "ignored")         ;; => 1 (short-circuits)
```

#### `not`

Returns the logical negation:

```lisp
(not expr)

;; Examples:
(not true)              ;; => false
(not false)             ;; => true
(not nil)               ;; => true
(not 42)                ;; => false
```

### Loops

Use `for` to iterate over collections:

```lisp
(for (variable collection)
  body)

;; Examples:
(for (x (list 1 2 3))
  (println x))

(for (i (range 0 5))     ;; Iterate from 0 to 4
  (println i))
```

### Early Return

Use `return` to exit early from a function:

```lisp
(defun find-positive (numbers)
  (for (n numbers)
    (if (> n 0)
        (return n)))
  nil)
```

## Built-in Functions

### Arithmetic Operations

#### Addition (`+`)
```lisp
(+ number1 number2 ...)
(+ 1 2 3)               ;; => 6
(+ 1.5 2.5)             ;; => 4.0
(+)                     ;; => 0
```

#### Subtraction (`-`)
```lisp
(- number1 number2 ...)
(- 10 3)                ;; => 7
(- 10 2 1)              ;; => 7
```

#### Multiplication (`*`)
```lisp
(* number1 number2 ...)
(* 2 3 4)               ;; => 24
(* 2.5 4)               ;; => 10.0
(*)                     ;; => 1
```

#### Division (`/`)
```lisp
(/ number1 number2 ...)
(/ 12 3)                ;; => 4
(/ 10 2 2)              ;; => 2.5
```

#### Modulo (`mod`)
```lisp
(mod dividend divisor)
(mod 10 3)              ;; => 1
(mod 15 4)              ;; => 3
```

#### Equality (`=`)
```lisp
(= value1 value2 ...)
(= 1 1)                 ;; => true
(= 1 2)                 ;; => false
(= 1 1 1)               ;; => true
```

### Data Structure Operations

#### Creating Lists
```lisp
(list item1 item2 ...)
(list 1 2 3)            ;; => (1 2 3)
(list)                  ;; => nil (empty list)
```

#### Creating Pairs
```lisp
(pair first second)
(pair 10 20)            ;; => (10 . 20)
```

#### Accessing Pair Elements
```lisp
(first pair)
(second pair)

(first (pair 1 2))      ;; => 1
(second (pair 1 2))     ;; => 2
```

#### Checking if Empty
```lisp
(empty? collection)
(empty? (list))         ;; => true
(empty? (list 1))       ;; => false
(empty? nil)            ;; => true
```

### Type Predicates

#### Number Predicate
```lisp
(number? value)
(number? 42)            ;; => true
(number? "hello")       ;; => false
```

#### String Predicate
```lisp
(string? value)
(string? "hello")       ;; => true
(string? 42)            ;; => false
```

#### Symbol Predicate
```lisp
(symbol? value)
(symbol? 'my-symbol)    ;; => true
(symbol? "string")      ;; => false
```

#### Nil Predicate
```lisp
(null? value)
(null? nil)             ;; => true
(null? 0)               ;; => false
```

#### Pair Predicate
```lisp
(pair? value)
(pair? (pair 1 2))      ;; => true
(pair? (list 1 2))      ;; => false
```

### Input/Output Operations

#### Print
```lisp
(print value1 value2 ...)
(print "Hello" " " "World")  ;; Outputs: Hello World
```

#### Print Line
```lisp
(println value1 value2 ...)
(println "Hello World")      ;; Outputs: Hello World\n
```

### Utility Functions

#### Apply Function
```lisp
(apply function argument-list)
(apply + (list 1 2 3))      ;; => 6
(apply * (list 2 3 4))      ;; => 24
```

#### Range Creation
```lisp
(range start end)
(range 0 5)                 ;; Creates pair representing [0, 5)
```

#### Help
```lisp
(help)                      ;; Shows REPL help information
```

## Advanced Features

### Function Composition

Functions can be passed as arguments and returned as values:

```lisp
(defun twice (f x)
  (f (f x)))

(defun add-one (x)
  (+ x 1))

(twice add-one 5)          ;; => 7
```

### Higher-Order Functions

```lisp
(defun map-list (f lst)
  (if (empty? lst)
      nil
      (pair (f (first lst))
            (map-list f (second lst)))))

(map-list square (list 1 2 3))  ;; => (1 4 9)
```

### Closures

Functions can capture variables from their enclosing scope:

```lisp
(defun make-counter (start)
  (let* ((count start))
    (function ()
      (def count (+ count 1))
      count)))

(def counter (make-counter 10))
(counter)                  ;; => 11
(counter)                  ;; => 12
```

## Error Handling

Spore provides error reporting for common issues:

### Type Errors
```lisp
(+ 1 "hello")             ;; Error: Type mismatch
```

### Arity Errors
```lisp
(+ 1)                     ;; Error: Wrong number of arguments
```

### Undefined Variables
```lisp
undefined-var             ;; Error: Variable not found
```

### Parse Errors
```lisp
(+ 1 2                    ;; Error: Unmatched parenthesis
```

## Memory Management

Spore features automatic garbage collection. Objects are automatically freed when they are no longer reachable from the program's root set (global variables, local variables, and call stack).

## Best Practices

1. **Use descriptive names**: Choose clear, meaningful names for variables and functions
2. **Keep functions small**: Break complex logic into smaller, reusable functions  
3. **Use local variables**: Prefer `let*` for temporary values instead of global `def`
4. **Comment complex logic**: Use `;;` comments to explain non-obvious code
5. **Handle edge cases**: Consider empty lists, nil values, and error conditions
6. **Use type predicates**: Check types when necessary to avoid runtime errors

## Examples

### Factorial Function
```lisp
(defun factorial (n)
  (if (= n 0)
      1
      (* n (factorial (- n 1)))))

(factorial 5)              ;; => 120
```

### List Sum
```lisp
(defun sum-list (lst)
  (if (empty? lst)
      0
      (+ (first lst) (sum-list (second lst)))))

(sum-list (list 1 2 3 4))  ;; => 10
```

### FizzBuzz
```lisp
(defun fizzbuzz (n)
  (for (i (range 1 (+ n 1)))
    (let* ((div3 (= (mod i 3) 0))
           (div5 (= (mod i 5) 0)))
      (if (and div3 div5)
          (println "FizzBuzz")
          (if div3
              (println "Fizz")
              (if div5
                  (println "Buzz")
                  (println i)))))))

(fizzbuzz 15)
```