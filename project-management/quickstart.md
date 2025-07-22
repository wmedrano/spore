# Spore Quickstart

## Introduction to Spore

Spore is a concise, Lisp-like scripting language designed to be embedded in other applications. It is dynamically typed and garbage-collected, which allows for rapid development and iteration. Its syntax is built on S-expressions, making the code structure simple and consistent.

## Basic Syntax and Data Types

The fundamental building block of Spore code is the S-expression (Symbolic Expression), which is a list of atoms (like numbers or symbols) enclosed in parentheses. The first element in an S-expression is typically a function or operator, and the rest are its arguments.

For example, `(+ 1 2)` is an S-expression that calls the `+` function with `1` and `2` as arguments, evaluating to `3`.

Spore is dynamically typed, meaning you don't need to declare the type of a variable. The language supports several fundamental data types:

-   **Numbers**: This includes both integers like `42` and floating-point numbers like `3.14`.
-   **Strings**: A sequence of characters inside double quotes, such as `"Hello, Spore!"`.
-   **Symbols**: Identifiers used to name variables and functions, like `x` or `squared-sum`. When quoted, they evaluate to themselves (e.g., `'my-symbol`).
-   **Lists**: The core data structure, created with the `list` function or as S-expressions. A list is a sequence of other values, e.g., `(list 1 "two" 'three)`.
-   **Nil**: A special value representing nothingness or falsehood. It can be written as `nil` or as an empty list `()`.
-   **Functions**: Procedures that can be called with arguments, defined with the `function` keyword.

### Truthiness and `nil`

Spore does not have distinct `true` and `false` boolean types. Instead, it uses a simple rule for conditional logic (like in an `if` expression):

-   **`nil` (or an empty list `()`) is the only "falsey" value.**
-   **Every other value is "truthy".** This includes numbers (even `0`), non-empty lists, strings (even `""`), and symbols.

For example, a built-in function might return the symbol `true` on success, but this is only for convention. In a conditional check, the symbol `true` is truthy simply because it is not `nil`.

```spore
;; `if` checks if a value is nil or not-nil
(if nil
    "this will not execute"
    "this will execute") ;; returns "this will execute"

(if 0
    "0 is truthy"
    "0 is falsey") ;; returns "0 is truthy"

(if 'true
    "'true is a symbol, and not nil, so it's truthy"
    "this will not execute") ;; returns "'true is a symbol..."
```

## Variables

You can define variables to store and reuse values.

### Global Variables

Use `def` to create a global variable. This is useful for defining state that can be accessed from anywhere in the program.

```spore
(def squared-sum 0)
```

### Local Variables

Use `let` to create temporary, local variables that are only accessible within a specific scope. This is the preferred way to manage state within a function or loop.

```spore
(let ((squared (* x x))
      (new-sum (+ squared squared-sum)))
  (def squared-sum new-sum))
```
In this example, `squared` and `new-sum` exist only within the `let` block.

## Functions

Functions are first-class citizens in Spore. You can create an anonymous function (also called a lambda) using the `function` keyword.

The syntax is `(function (parameters) body)`.

Here's a function that takes two arguments, `a` and `b`, and returns their sum:

```spore
(function (a b) (+ a b))
```

To call a function immediately after defining it, you can wrap the definition and its arguments in another S-expression:

```spore
;; Defines a function and calls it with 1 and 2, resulting in 3
((function (a b) (+ a b)) 1 2)
```

## Control Flow

Spore provides constructs for controlling the flow of execution.

### If Statements

You can conditionally execute code using `if`. The syntax is `(if condition then-expression else-expression)`.

If the `condition` evaluates to a non-nil value (meaning anything other than `()` or `nil`), the `then-expression` is executed. Otherwise, the optional `else-expression` is executed. If the condition is false and no `else-expression` is provided, the entire expression evaluates to `nil`.

```spore
;; With an else-expression
(if (> a 0)
  "a is positive"
  "a is not positive")

;; Without an else-expression, this returns nil if a is not positive
(if (> a 0)
  "a is positive")
```

### For Loops

You can iterate over a list using a `for` loop. The syntax is `(for (variable list-expression) body)`. The `body` is executed for each item in the list.

```spore
(for (x (list 1 2 3 4))
  ;; This code runs 4 times, with x being 1, 2, 3, and 4
  (let ((squared (* x x)))
    ;; ... do something with squared ...
    ))
```

## Basic Operations

Spore includes a set of built-in functions for common operations.

-   **Arithmetic**: `+`, `*`, `-`, `mod`
    ```spore
    (+ 10 20) ;; returns 30
    (* 5 5)   ;; returns 25
    (- 10 4)  ;; returns 6
    (- 5)     ;; returns -5 (negation)
    (mod 10 3) ;; returns 1
    ```

-   **Comparison**: `=`
    ```spore
    (= 5 5)   ;; returns true
    (= 5 6)   ;; returns nil
    (= 5 5.0) ;; returns true
    ```

-   **List Manipulation**: `list`, `cons`, `car`, `cdr`. The `list` function creates a new list from its arguments. For more fundamental control, `cons` adds an element to the front of a list, while `car` and `cdr` access the first element (the "head") and the rest of the list (the "tail"), respectively.
    ```spore
    (list 1 2 3)        ;; returns a list containing (1 2 3)
    (cons 1 (list 2 3)) ;; returns a new list (1 2 3)
    (car (list 1 2 3))  ;; returns the first element, 1
    (cdr (list 1 2 3))  ;; returns the rest of the list, (2 3)
    ```

-   **Type Predicates**: `number?`, `symbol?`, `null?`, `string?`. These functions check the type of a value, returning `true` or `false`.
    ```spore
    (number? 123)     ;; returns true
    (string? "hello") ;; returns true
    (symbol? 'sym)    ;; returns true
    (null? nil)       ;; returns true
    (number? "123")   ;; returns false
    ```

-   **String Operations**: `->string`, `print`. Use `->string` to convert any single value to its string representation. Use `print` to concatenate the string representations of multiple values.
    ```spore
    (->string (list 1 2)) ;; returns "(1 2)"
    (print "Hello, " 1)   ;; returns "Hello, 1"
    ```

## Examples

Here is a complete example that uses several of the concepts discussed above. It calculates the sum of the squares of numbers in a list.

```spore
;; Initialize a global variable to store the sum
(def squared-sum 0)

;; Iterate through the list of numbers
(for (x (list 1 2 3 4))
  ;; Use a let block for temporary variables
  (let ((squared (* x x))
        (new-sum (+ squared squared-sum)))
    ;; Update the global sum
    (def squared-sum new-sum)))

;; The final expression is the value of squared-sum, which is 30
squared-sum
```

## Next Steps

Now that you have a basic understanding of Spore, try experimenting with your own expressions and functions.

Here are some exercises to get you started:

-   **FizzBuzz**: Write a program that prints the numbers from 1 to 100. For multiples of three, print "Fizz" instead of the number. For multiples of five, print "Buzz". For numbers which are multiples of both three and five, print "FizzBuzz".

-   **Fibonacci**: Write a function that calculates the nth Fibonacci number. The Fibonacci sequence is a series of numbers where each number is the sum of the two preceding ones, usually starting with 0 and 1.
