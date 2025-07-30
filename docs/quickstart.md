# Spore Quickstart

## Basic Syntax and Data Types

Spore is a Lisp-like scripting language built on S-expressions (e.g., `(+ 1 2)`). It is dynamically typed and supports several data types:

-   **Numbers**: `42`, `3.14`
-   **Booleans**: `true`, `false`
-   **Strings**: `"Hello"`
-   **Nil**: Represents nothingness (`nil` or `()`).
-   **Symbols**: `x`, `'my-symbol`
-   **Lists**: `(list 1 "two")`
-   **Functions**: Defined with the `function` keyword.

### Comments

Spore supports single-line comments using two semicolons (`;;`). Anything after
`;;` on a line is considered a comment and is ignored by the interpreter.

```lisp
;; This is a comment
(print "Hello") ;; This is also a comment
```



## Variables

Use `def` to define global variables and `let*` for local variables within a
scope. The last expression in a `let*` body is returned.

```lisp
(def global-var 10)

(let* ((local-var 20))
  (+ global-var local-var)) ;; returns 30
```

## Functions

Functions are first-class citizens in Spore. You can create an anonymous
function (also called a lambda) using the `function` keyword.

The syntax is `(function (parameters) body)`.

Here's a function that takes two arguments, `a` and `b`, and returns their sum:

```lisp
(function (a b) (+ a b))
```

To call a function immediately after defining it, you can wrap the definition
and its arguments in another S-expression:

```lisp
;; Defines a function and calls it with 1 and 2, resulting in 3
((function (a b) (+ a b)) 1 2)
```

## Control Flow

Spore provides constructs for controlling the flow of execution.

### If Statements

You can conditionally execute code using `if`. The syntax is `(if condition
then-expression else-expression)`.

If the `condition` evaluates to a truthy value (meaning anything other than
`false` or `nil`), the `then-expression` is executed. Otherwise, the optional
`else-expression` is executed. If the condition is false and no
`else-expression` is provided, the entire expression evaluates to `nil`.

```lisp
;; With an else-expression
(if (> a 0)
  "a is positive"
  "a is not positive")

;; Without an else-expression, this returns nil if a is not positive
(if (> a 0)
  "a is positive")
```

### For Loops

You can iterate over a list using a `for` loop. The syntax is `(for (variable
list-expression) body)`. The `body` is executed for each item in the list. The
`for` loop itself does not return a value.

```lisp
(for (x (list 1 2 3 4))
  ;; This code runs 4 times, with x being 1, 2, 3, and 4
  (let* ((squared (* x x)))
    ;; ... do something with squared ...
    ))
```

## Basic Operations

Spore includes a set of built-in functions for common operations.

-   **Arithmetic**: `+`, `*`, `-`, `mod`
    ```lisp
    (+ 10 20) ;; returns 30
    (* 5 5)   ;; returns 25
    (- 10 4)  ;; returns 6
    (- 5)     ;; returns -5 (negation)
    (mod 10 3) ;; returns 1
    (/ 4 2)    ;; returns 2.0
    (/ 5.0 2.0) ;; returns 2.5
    (/ 2)      ;; returns 0.5 (1.0 / 2)
    ```

-   **Comparison**: `=`
    ```lisp
    (= 5 5)   ;; returns true
    (= 5 6)   ;; returns false
    (= 5 5.0) ;; returns true
    ```

-   **Logical Operators**: `or`, `and`
    Spore provides `or` and `and` for logical disjunction and conjunction, respectively.

    -   `or`: Evaluates arguments from left to right. It returns the first
        argument that evaluates to a "truthy" value. If all arguments are
        "falsey" (`false` and `nil`), it returns the last falsey value. This
        operator is "short-circuiting"; once a truthy value is found, no further
        arguments are evaluated.

    ```lisp
    (or false true)                   ;; returns true
    (or nil 0)                        ;; returns 0 (since 0 is truthy)
    (or (null? (list 1)) "hello")     ;; returns "hello" (since (null? (list 1)) is false)
    (or false nil)                    ;; returns nil (all falsey, returns last falsey value, which is nil in Spore)
    (or (= 1 2) (= 3 3) (not-called)) ;; returns true (short-circuits after (= 3 3))
    ```

    -   `and`: Evaluates arguments from left to right. It returns the first argument that evaluates to a "falsey" value. If all arguments are "truthy", it returns the last argument. This operator is "short-circuiting"; once a falsey value is found, no further arguments are evaluated.

    ```lisp
    (and true false)                   ;; returns false
    (and 10 "hello")                   ;; returns "hello" (since 10 and "hello" are truthy, returns last truthy)
    (and true (null? (list 1)))        ;; returns false (since (null? (list 1)) is false)
    (and 0 nil false)                  ;; returns nil (since 0 is truthy, nil is falsey, returns first falsey)
    (and (= 1 1) (= 2 3) (not-called)) ;; returns false (short-circuits after (= 2 3))
    ```

-   **List Manipulation**: `list`, `cons`, `car`, `cdr`. The `list` function creates a new list from its arguments. For more fundamental control, `cons` adds an element to the front of a list, while `car` and `cdr` access the first element (the "head") and the rest of the list (the "tail"), respectively.
    ```lisp
    (list 1 2 3)        ;; returns a list containing (1 2 3)
    (cons 1 (list 2 3)) ;; returns a new list (1 2 3)
    (car (list 1 2 3))  ;; returns the first element, 1
    (cdr (list 1 2 3))  ;; returns the rest of the list, (2 3)
    ```

-   **Type Predicates**: `number?`, `symbol?`, `null?`, `string?`. These functions check the type of a value, returning `true` or `false`.
    ```lisp
    (number? 123)       ;; returns true
    (string? "hello")   ;; returns true
    (symbol? 'sym)      ;; returns true
    (null? nil)         ;; returns true
    (empty? (list))     ;; returns true
    (empty? (list 1 2)) ;; returns false
    (number? "123")     ;; returns false
    ```

-   **String Operations**: `->string`, `print`. Use `->string` to convert any single value to its string representation.  `print` displays the string representation of each of its arguments. `println` does the same, but adds a newline at the end.
    ```lisp
    (->string (list 1 2)) ;; returns "(1 2)"
    (print "Hello, " 1)   ;; displays "Hello, 1" to the console
    (println "Hello, " 1) ;; displays "Hello, 1\n" to the console
    ```

## Examples

Here is a complete example that uses several of the concepts discussed above. It
calculates the sum of the squares of numbers in a list.

```lisp
;; Initialize a global variable to store the sum
(def squared-sum 0)

;; Iterate through the list of numbers
(for (x (list 1 2 3 4))
  ;; Use a let* block for temporary variables
  (let* ((squared (* x x))
        (new-sum (+ squared squared-sum)))
    ;; Update the global sum
    (def squared-sum new-sum)))
squared-sum
```

The final expression is the value of `squared-sum`, which is `30`.

## Running Spore Programs

To run a Spore program, save your Spore code in a file (e.g., `my_program.spore`) and execute it from your terminal:

```sh
spore my_program.spore
```

## Next Steps

Now that you have a basic understanding of Spore, try experimenting with your
own expressions and functions.

Here are some exercises to get you started:

-   **FizzBuzz**: Write a program that prints the numbers from 1 to 100. For
    multiples of three, print "Fizz" instead of the number. For multiples of
    five, print "Buzz". For numbers which are multiples of both three and five,
    print "FizzBuzz".

-   **Fibonacci**: Write a function that calculates the nth Fibonacci
    number. The Fibonacci sequence is a series of numbers where each number is
    the sum of the two preceding ones, usually starting with 0 and 1.
