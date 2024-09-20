# Spore


Spore is in early development. It is:

- A text editor
- powered by a simple Lisp language.

It is inspired by Emacs. The editor is meant to be mostly implemented in Spore
Lisp. Rust is used to implement the VM itself, along with a core set of Spore
Lisp functionality like `Buffer` and `Event` objects.

## Crates

- [Code Coverage](https://wmedrano.github.io/spore/llvm-cov)

### Spore Editor

The main text editor. This is not ready for use yet.

### Spore VM

The Virtual Machine that implements Spore lispy language.

- [Rust Docs](https://wmedrano.github.io/spore/doc/spore_vm)
