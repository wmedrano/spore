# Spore Quickstart

## Introduction to Spore

Spore is a concise, Lisp-like scripting language designed to be embedded in other applications. It is dynamically typed and garbage-collected, which allows for rapid development and iteration. Its syntax is built on S-expressions, making the code structure simple and consistent.

## Basic Syntax and Data Types

The fundamental building block of Spore code is the S-expression (Symbolic Expression), which is a list of atoms (like numbers or symbols) enclosed in parentheses. The first element in an S-expression is typically a function or operator, and the rest are its arguments.

For example, `(+ 1 2)` is an S-expression that calls the `+` function with `1` and `2` as arguments, evaluating to `3`.

Spore supports basic data types like numbers and lists:
- **Numbers**: e.g., `10`, `30`
- **Lists**: A sequence of values, created with the `list` function. e.g., `(list 1 2 3 4)`

The empty list, `()`, is also used to represent a `nil` or null value.

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

-   **Arithmetic**: `+`, `*`
    ```spore
    (+ 10 20) ; returns 30
    (* 5 5)   ; returns 25
    ```
-   **List Creation**: `list`
    ```spore
    (list 1 2 3 4) ; returns a list containing 1, 2, 3, 4
    ```

## Memory Management

Spore manages memory automatically using a garbage collector. You do not need to manually allocate or deallocate objects. The garbage collector periodically runs to clean up objects that are no longer in use, simplifying memory management for the developer.

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
