---
layout: default
title: REPL Commands
nav_enabled: true
nav_order: 1
---

# Overview

The REPL can be used to evaluate expressions. Additionally, the REPL
provides access to several special commands.

## Debugging

### Trace

The `,trace` command prints the input and output of every function
call.

```lisp
>> ,trace (if (< 1 2) (+ 1 2) (* 1 2))
(<proc repl-proc-1>) => 3
  (<proc <> 1 2) => true
  (<proc +> 1 2) => 3
$1 = 3
```

### Time

The `,time` command prints out the duration for each expression.

```lisp
>> ,time (+ 1 2 3 4) (* 1 2 3 4) (/ 1 2 3 4)
Time: 6.572µs
$1 = 10
Time: 1.943µs
$2 = 24
Time: 2.305µs
$3 = 0.041666666666666664
```

## Interpreter Internals

### Tokens

The `,tokens` command displays the lexical tokens of an expression.

```lisp
>> ,tokens (+ 1 2)
Token { item: LeftParen, range: 0..1 }
Token { item: Identifier("+"), range: 1..2 }
Token { item: Int(1), range: 3..4 }
Token { item: Int(2), range: 5..6 }
Token { item: RightParen, range: 6..7 }
```

### Abstract Syntax Tree (AST)

The `,ast` command shows the abstract syntax tree of an expression.

```lisp
>> ,ast (+ 1 2)
<identifier +>
  <int 1>
  <int 2>
```

### Intermediate Representation (IR)

The `,ir` command displays the intermediate representation of an
expression.

```lisp
>> ,ir (+ 1 2)
CodeBlock {
    name: Some(
        "0",
    ),
    arg_to_idx: {},
    instructions: [
        CallProc {
            proc: DerefIdentifier {
                symbol: "+",
            },
            args: [
                PushConst(
                    Int(
                        1,
                    ),
                ),
                PushConst(
                    Int(
                        2,
                    ),
                ),
            ],
        },
    ],
}
```

### Bytecode

The `,bytecode` command shows the bytecode generated for an expression.

```lisp
>> ,bytecode (+ 1 2)
  01 - get value for %virtual%/%repl%/+
  02 - push value 1
  03 - push value 2
  04 - evaluate last 3
```

If invoked on a `procedure`, then the bytecode for that procedure will
be printed out.

```lisp
>> (define (make-pair a b) (list a b))
>> ,bytecode (make-pair 1 2)
  01 - get value for %virtual%/%repl%/make-pair
  02 - push value 1
  03 - push value 2
  04 - evaluate last 3
>> ,bytecode make-pair
  01 - get value for %virtual%/%repl%/list
  02 - get arg 0
  03 - get arg 1
  04 - evaluate last 3
```
