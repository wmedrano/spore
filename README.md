# Spore

An interpretted programming language used for Rust.

[Documentation](https://wmedrano.github.io/spore)

## Installation

[Detailed Installation Documentation](https://wmedrano.github.io/spore/installation.html)

Before installing Spore, ensure you have the following prerequisites:

- Rust and Cargo (Rust's package manager) installed on your system. If
  you don't have Rust installed, you can get it from
  [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).
- Git (optional, but recommended for cloning the repository)

```sh
# Download
git clone https://github.com/wmedrano/spore.git
cd spore
# Run install script
sh install.sh
# Add to path for current session.
# Extra work is needed to make always have spore in $PATH.
export PATH="$HOME/.spore/bin:$PATH"
# Run Spore
spore
```

## Syntax

Spore uses a lispy syntax. All expressions are surrounded by
parentheses with the first identifier as the operator/procedure and
subsequent identifiers as the arguments.

Expressions:

- `1`
- `"Hello World!"`
- `1.4`
- `1.4e4`
- `(+ 1 2 3)`


### Defining Variables

```lisp
(define pi 3.14)
```

### Defining Procedures

```lisp
(define (circle-area radius)
  (* radius radius pi))
```

### Calling Procedures

```lisp
>> (println (circle-area 2))
12.56
```

## FAQ


**Q: Is this usable?**

> No, this is a toy project.

**Q: Why all the parentheses?**

> Spore is a Lisp which means it uses parentheses. While the syntax
> may not be elegant, it is simple to understand. The simple syntax
> also allows me to focus more on building the VM and less on language
> design.
