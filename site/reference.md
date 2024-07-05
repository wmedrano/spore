---
layout: default
title: Reference
nav_enabled: true
nav_order: 3
---

# Reference

## Control Flow

### if
Evaluates a predicate to determine which branch to run and return. Must
be of the form `(if <pred> <true-branch>)` or `(if <pred> <true-branch> <false-branch>)`

```lisp
>> (if true 1 2)
$1 = 1
>> (if false 1 2)
$2 = 2
>> (if (< 1 2) "1 < 2" "1 is not < than 2")
$3 = "1 < 2"
>> (if true "it was true")
$4 = "it was true"
>> (if false "it was true")
>> (if true (do 1 2 3 4))
$5 = 4
```

### do
Evaluates multiple expressions in sequence, returning the value of the
last expression.

```lisp
>> (do)
>> (do 1 2 3)
$1 = 3
```

### apply
Applies a procedure to arguments provided as a list.

```lisp
>> (apply + (list 1 2 3 4))
$1 = 10
>> (+ 1 2 3 4)
$2 = 10
>> (apply string-concat (list "hello" " " "world" "!"))
$3 = "hello world!"
```

## Lists

### list
Creates a new list containing the given arguments.

```lisp
>> (list 1 2 3 "go")
$1 = (1 2 3 "go")
```

### list?
Checks if the given argument is a list.  Returns `true` if `arg` is a
list, `false` otherwise.

```lisp
>> (list? (list))
$1 = true
>> (list? "list")
$2 = false
```


### first
Returns the first element of a list.  Throws an error if the list is
empty.

```lisp
>> (first (list 10 20 30))
$1 = 10
```

### rest
Returns a new list containing all elements of the input list except
the first.  Returns an empty list if the input list has only one
element. Throws an error if argument is not a list or an empty list.

```lisp
>> (rest (list 1 2 3))
$1 = (2 3)
>> (rest (list 1))
$2 = ()
```


### nth
Returns the nth element of a list (0-indexed). Throws an error if the
index is out of bounds.

```lisp
>> (nth (list 0 1 2 3 4) 2)
$1 = 2
```

### len
Returns the length of a list or string.

```lisp
>> (len (list 1 2 3))
$1 = 3
>> (len ("12345"))
$2 = 5
```

## Strings

### substring
Extracts a portion of a string.  Returns a new string containing
characters from index `start` (inclusive) to `end` (exclusive).

```lisp
>> (substring "012345" 2 4)
$1 = "23"
```


### string-concat
Concatenates two or more strings.

```lisp
>> (string-concat "hello" " " "world")
$1 = "hello world"
```

### ->string
Converts all arguments into a string by concatenating their string
representation.

```lisp
>> (->string "The answer is: " (- 100 50 8))
$1 = "The answer is: 42"
```

## Arithmetic Operations

### +
Adds 0 or more numbers. If 0 numbers are provided, then `0` is
returned.

```lisp
>> (+ 1 2 3)
$1 = 6
>> (+)
$2 = 0
```

### -
Subtracts numbers from the first argument. If only one argument is
provided, then it is negated.

```lisp
>> (- 2 3)
$1 = -1
>> (- 10)
$2 = -10
```

### *
Multiplies 0 or more numbers. If 0 numbers are provided, then `1` is
returned.

```lisp
>> (* 1 2 3)
$1 = 6
>> (*)
$2 = 1
```

### /
Divides the first number by the subsequent numbers. If only one number
is provided, then the reciprocal is returned.

```lisp
>> (/ 3 4)
$1 = 0.75
>> (/ 4)
$2 = 0.25
>> (/ 1 0)
$3 = inf
```

## Comparison Operations

### <
Checks if the first argument is less than the second argument.

```lisp
>> (< 1 2)
$1 = true
>> (< 2 1)
$2 = false
>> (< 1 1)
$3 = false
```

### <=
Checks if the first argument is less than or equal to the second
argument.

```lisp
>> (<= 1 2)
$1 = true
>> (<= 2 1)
$2 = false
>> (<= 1 1)
$3 = true
```

### >
Checks if the first argument is greater than the second.

```lisp
>> (> 1 2)
$1 = false
>> (> 2 1)
$2 = true
>> (> 1 1)
$3 = false
```

### >=
Checks if the first argument is greater than or equal to the second
argument.

```lisp
>> (>= 1 2)
$1 = false
>> (>= 2 1)
$2 = true
>> (>= 1 1)
$3 = true
```

### equal?
Returns `true` if 2 values are equal. The 2 arguments may be of any
type.

```lisp
>> (equal? 1 1)
$1 = true
>> (equal? (list 1 2 3) (list 1 2 3))
$2 = true
>> (equal? 1 "one")
$3 = false
>> (equal? 1 2)
$4 = false
```

## I/O Operations

### println
Prints its arguments to the console, followed by a newline.

```lisp
>> (println "The answer is " 42)
The answer is 42
```

## Module Operations

### modules
Lists all available modules.

```lisp
>> (modules)
$1 = ("%global%" "%virtual%/%repl%")
```

### module-info
Prints information about a specific module.

```lisp
>> (module-info "%global%")
Module: %global%
  'module-info => <proc module-info>
  'list? => <proc list?>
  '< => <proc <>
  'substring => <proc substring>
  '+ => <proc +>
  'string-concat => <proc string-concat>
  '* => <proc *>
  '>= => <proc >=>
  'equal? => <proc equal?>
  'first => <proc first>
  'do => <proc do>
  'len => <proc len>
  'modules => <proc modules>
  'rest => <proc rest>
  'println => <proc println>
  'apply => <proc apply>
  '- => <proc ->
  '->string => <proc ->string>
  'nth => <proc nth>
  '<= => <proc <=>
  '> => <proc >>
  '/ => <proc />
  'list => <proc list>
```
