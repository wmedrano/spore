---
layout: default
title: Installation
nav_enabled: true
nav_order: 1
---

# Overview

This guide outlines the process of installing Spore. This:

- Builds Spore.
- Installs Spore at `$HOME/.spore/bin`.
- Adds `$HOME/.spore/bin` to `$PATH` so that it can be run anywhere.

## Prerequisites

- Rust and Cargo (Rust's package manager) installed on your system. If you don't have Rust installed, you can get it from [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).
- Git (optional, but recommended for cloning the repository).

## Installation Steps

1. Clone the Spore repository (or download the source code):

   ```
   git clone https://github.com/wmedrano/spore.git
   cd spore
   ```

1. Run the installation script:

   ```sh
   sh install.sh
   ```

   The script will:
   - Build Spore using Cargo.
   - Create a `.spore/bin` directory in your home folder.
   - Copy the Spore executable to `$HOME/.spore/bin/spore`.

1. Add Spore to your PATH:

   To use Spore from any location, add the following line to your shell configuration file (e.g., `~/.bashrc`, `~/.zshrc`, or `~/.profile`):

   ```
   export PATH="$HOME/.spore/bin:$PATH"
   ```

   After modifying the file, reload your shell configuration:

   ```
   source ~/.bashrc # or the appropriate file for your shell.
   ```

1. Verify the installation:

   ```
   spore --version
   ```

   This should display the version of Spore you've installed.
