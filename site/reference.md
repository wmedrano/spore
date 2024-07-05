---
layout: default
title: Reference
nav_enabled: true
nav_order: 3
---

# Reference

## Control Flow

### if
Evalutes a predicate to determine which branch to run and return. Must
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

## Arithmetic Operations

### +
Adds two or more numbers.

```lisp
>> (+ 1 2 3)
$1 = 6
>> (+)
$2 = 1
```

### -
Subtracts numbers from the first argument.

```lisp
>> (- 2 3)
$1 = -1
>> (- 10)
$2 = -10
```

### *
Multiplies two or more numbers.

```lisp
>> (* 1 2 3)
$1 = 6
>> (*)
$2 = 1
```

### /
Divides the first number by the subsequent numbers.

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
Checks if two values are equal.

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
Provides information about a specific module.

```lisp
>> (println (module-info "%global%"))
Module: %global%
  'first => <proc first>
  '* => <proc *>
  'rest => <proc rest>
  '/ => <proc />
  'substring => <proc substring>
  'list? => <proc list?>
  'string-concat => <proc string-concat>
  '- => <proc ->
  'module-info => <proc module-info>
  '< => <proc <>
  '> => <proc >>
  '>= => <proc >=>
  'equal? => <proc equal?>
  'nth => <proc nth>
  'modules => <proc modules>
  'list => <proc list>
  'len => <proc len>
  'do => <proc do>
  'println => <proc println>
  '<= => <proc <=>
  '+ => <proc +>
```
