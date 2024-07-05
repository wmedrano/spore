---
layout: default
title: Modules
nav_enabled: true
nav_order: 2
---

# Modules

## Overview

In Spore, modules are a fundamental unit of code organization. They
provide a way to encapsulate related functionality and manage
namespaces.

## Module Definition

Modules in Spore are defined by files. Each `.spore` file is
considered a separate module.

## Importing Modules

To use functionality from another module, you need to import it using
the `import` function. The syntax is as follows:

```lisp
(import "path/to/module.spore")
```

The path should be relative to the working directory.

## Accessing Module Contents

After importing a module, you can access its contents using the module
name (derived from the file name) followed by a forward slash and the
value name:

```lisp
module/value-name
```

For example, if you have a module `math.spore` with a function `add`, you would use it like this:

```lisp
(import "math.spore")
(math/add 2 3)
```

## Example

Here's a complete example demonstrating module usage:

File: `math.spore`
```lisp
(define (add a b)
  (+ a b))

(define pi 3.14159)
```

File: `main.spore`
```lisp
(import "math.spore")

(println (math/add 5 7))  ; Output: 12
(println math/pi)         ; Output: 3.14159
```
