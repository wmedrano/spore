---
layout: home
title: Quick Start
nav_enabled: true
nav_order: 0
---

# Spore

Spore is an interpreted (toy) programming language.

## Quick Start

### Running

The REPL (Read-Evaluate-Print-Loop) is used to run and debug Spore
code.

{% mermaid %}
graph LR;
    ReadInput --> EvaluateInput;
    EvaluateInput --> PrintResult;
    PrintResult --> ReadInput;
{% endmermaid %}

The REPL can be started by running:

```shell
cargo run
```

### Expressions

Expressions can be evaluated interactively. Expressions have the form
of a constant (like `1`, `2.0`, `"hello world"`) or a procedure
evaluation in the form of `(<procedure> <operands...>)`:

```lisp
>> (+ 1 2)
$1 = 3
>> (- 3 4)
$2 = -1
>> 5
$3 = 5
>> (* $1 $2 $3)
-15
```

### Defining Values

Values can be defined and referenced:

```lisp
>> (define pi 3.14)
>> (- pi 3)
$1 = 0.14
```

### Defining Procedures

Procedures can be defined and called:

```lisp
>> (define pi 3.14)
>> (define (circle-area radius)
..   (* radius pi pi))
>> (circle-area 2)
$1 = 19.7192
```

## FAQ

**Q: Is Spore usable?**

> No, this is a toy project.

**Q: Why all the parentheses?**

> Spore is a Lisp which means it uses parentheses. While the syntax
> may not be elegant, it is simple to understand. The simple syntax
> also allows me to focus more on building the VM and less on language
> design.
