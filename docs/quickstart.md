# Spore Quickstart

## Running Spore Programs

Spore programs are executed by reading input from standard input. There are two
ways to run a program:

1.  **From a file:**
    ```sh
    spore < my_program.spore
    ```

2.  **Interactively:** Run `spore`, type the program into the terminal, and
    send an end-of-file character (typically `Ctrl+d`).

### Emacs Org Mode

Alternatively, programs may be edited and executed within Emacs Org mode.

1. Load `tools/spore-mode.el`.  (For example, add `(load-file
   "/path/to/spore/tools/spore-mode.el")` to your Emacs configuration.)
2. Create an org block and execute it (typically with `C-c C-c`, to execute the source block).

```org
#+BEGIN_SRC spore
(defun foo (a b) (+ a b))
(println (foo 1 2))
#+END_SRC

#+RESULTS:
: 3
```

## Basic Syntax and Data Types

Spore is a Lisp-like scripting language built on S-expressions (e.g., `(+ 1 2)`). It is dynamically typed and supports several data types:

-   **Numbers**: `42`, `3.14`
-   **Booleans**: `true`, `false`
-   **Strings**: `"Hello"`
-   **Nil**: Represents nothingness (`nil`).
-   **Symbols**: `x`.
-   **Lists**: `(list 1 "two")` or `(quote (1 "two"))`.
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

Functions are first-class citizens in Spore.

Spore provides the `defun` macro for defining named functions.

The syntax is `(defun name (parameters) body)`.

For example:
```lisp
(defun add (a b)
  (+ a b))

(add 5 3) ;; returns 8
```

### Lambdas

You can also create an anonymous function (also called a lambda) using the
`function` keyword. `defun` is syntactic sugar that allows you to define a
function and assign it to a symbol in one step, equivalent to using `def` with a
`function` expression.

The syntax for an anonymous function is `(function (parameters) body)`.

Here's an anonymous function that takes two arguments, `a` and `b`, and returns their sum:
```lisp
(function (a b) (+ a b))
```

To call a function immediately after defining it, you can wrap the definition and its arguments in another S-expression:

```lisp
;; Defines a function and calls it with 1 and 2, resulting in 3
((function (a b) (+ a b)) 1 2)
```

### Early Returns with `return`

Spore supports early returns from within functions using the `return` form. When
`(return <expression>)` is evaluated, the enclosing function immediately stops
execution and returns the value of `<expression>`.  This is useful for exiting
early from loops or handling base cases in recursion.

For example:
```lisp
(defun find-first-positive (numbers)
  (for (n numbers)
    (if (> n 0)
      (return n))) ;; Immediately exits the function and returns n
  nil) ;; Returned if no positive number is found

(find-first-positive (list -1 -5 3 8)) ;; returns 3
(find-first-positive (list -2 -1))     ;; returns nil
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

For loops also allow iterating over a half-open integer range using a `cons`
pair. The loop will include the `start` number and go up to, but not include,
the `end` number.

```lisp
;; Iterates with x as 0, 1, 2, 3
(for (x (cons 0 4))
  (print x))
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
    ;; The division operator `/` can take one or two arguments. When given two
    ;; arguments, it returns their quotient. When given a single argument, it
    ;; returns the reciprocal (1.0 divided by the argument). Division in Spore
    ;; always results in a floating-point number.
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
        "falsy" (`false` and `nil`), it returns the last falsy value. This
        operator is "short-circuiting"; once a truthy value is found, no further
        arguments are evaluated.

    ```lisp
    (or false true)                   ;; returns true
    (or nil 0)                        ;; returns 0 (since 0 is truthy)
    (or (null? (list 1)) "hello")     ;; returns "hello" (since (null? (list 1)) is false)
    (or false nil)                    ;; returns nil (all falsy, returns last falsy value, which is nil in Spore)
    (or (= 1 2) (= 3 3) (not-called)) ;; returns true (short-circuits after (= 3 3))
    ```

    -   `and`: Evaluates arguments from left to right. It returns the first argument that evaluates to a "falsy" value. If all arguments are "truthy", it returns the last argument. This operator is "short-circuiting"; once a falsy value is found, no further arguments are evaluated.

    ```lisp
    (and true false)                   ;; returns false
    (and 10 "hello")                   ;; returns "hello" (since 10 and "hello" are truthy, returns last truthy)
    (and true (null? (list 1)))        ;; returns false (since (null? (list 1)) is false)
    (and 0 nil false)                  ;; returns nil (since 0 is truthy, nil is falsy, returns first falsy)
    (and (= 1 1) (= 2 3) (not-called)) ;; returns false (short-circuits after (= 2 3))
    ```

-   **Quoting**: `quote`
    The `quote` form prevents the evaluation of its argument. Instead of executing the expression, `quote` returns the expression itself as a literal value. This is useful for treating code or data structures literally. `quote` expects exactly one argument.
    ```lisp
    (quote (+ 1 2))     ;; returns the list (+ 1 2), not 3
    (quote my-symbol)   ;; returns the symbol my-symbol, not its assigned value
    (quote "hello")     ;; returns the string "hello"
    (quote 42)          ;; returns the number 42
    ```

-   **List Manipulation**: `list`, `cons`, `car`, `cdr`. The `list` function creates a new list from its arguments. For more fundamental control, `cons` adds an element to the front of a list, while `car` accesses the first element (the "head" of the list) and `cdr` accesses the rest of the list (the "tail" of the list), respectively.
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

-   **String Operations**: `->string`, `print`, `println`

    Use `->string` to convert any single value to its string representation.
    `print` displays the string representation of each of its arguments.
    `println` does the same, but adds a newline at the end.
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
