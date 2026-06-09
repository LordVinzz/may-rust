# Proofs

This directory contains the Rocq proof for the parser grammar and AST invariants.

## How to run

From the project root, run:

```sh
make -C proofs
```

To clean generated proof files and run everything again:

```sh
make -C proofs recheck
```

## Purpose

The proof checks, in simple terms, that when a valid token stream is parsed successfully, the produced AST has the expected structure.

It proves that parsed programs follow the grammar rules: imports come before the namespace, a namespace contains a component, a component contains valid requirements, provided ports, parts, and binds, and the generated AST satisfies the invariants defined in the proof.

The proof starts from tokens. It does not prove that the lexer correctly turns source text into tokens, and it does not prove the compiled Rust binary itself.
