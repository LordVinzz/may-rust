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

The proof mirrors the SPEADL parser and AST in
`may_rust/src/modules/speadl`. It checks, in simple terms, that when a valid
token stream is parsed successfully, the produced AST has the expected
structure.

It proves that parsed programs follow the grammar rules: imports come before the namespace, a namespace contains a component, a component contains valid requirements, provided ports, parts, and binds, and the generated AST satisfies the invariants defined in the proof.

The formal AST includes the Rust metadata introduced by import resolution:
an import may contain a resolved AST, a specialization contains its parent and
optional parent AST, and a provided service is either local or delegated. File
lookup, I/O, and recursive parsing are represented by an abstract resolver;
the proof models the parser's first matching import lookup and the attachment
of the resolved parent to that import.

Concrete examples in `Grammar.v` establish that the namespace and part parser
relations are inhabited, including generic parts and one- or two-segment bind
targets. This prevents the soundness theorems from succeeding only because a
parser relation is accidentally impossible to construct.

## Proof files

- `Grammar.v` defines tokens, the current AST, guarded parser relations, import
  attachment, well-formedness, and the main parser soundness theorem.
- `LexerCorrectness.v` defines an executable reference model of the SPEADL
  lexer for ASCII inputs, proves its token result is unique, and states the
  exact bridge obligation for the native Rust lexer. It includes computed
  success and invalid-character examples.
- `ParserEquivalence.v` states the bidirectional equivalence obligation between
  an observation of `Parser::namespace` and the Coq relation. From that
  obligation it proves AST soundness and uniqueness for the Rust result.
- `ParserCompleteness.v` defines completeness against the declarative grammar
  and proves that equivalence implies acceptance of every valid token stream.
- `ParserDeterminism.v` proves directly, without an external assumption, that
  every parser relation is deterministic, including the complete namespace
  parser.
- `ImportResolution.v` models same-directory and ancestor candidates,
  first-existing-file selection, reading, recursive parsing, first matching
  import lookup, and parent attachment. It exposes the remaining bridge to the
  native filesystem operations.

## Trust boundary

All checked theorems compile without `Admitted` or a hidden axiom. Pure Coq
cannot execute the repository's native Rust binary or the host filesystem.
Consequently, `LexerCorrectness.v`, `ParserEquivalence.v`, and
`ImportResolution.v` express their native Rust connection as explicit
properties of a supplied runner/resolver. Proving those properties for the
compiled implementation requires a verified Rust-to-Coq translation or a
formal semantics of the relevant Rust code. The lexer reference currently
covers ASCII input; Rust's additional Unicode whitespace behavior remains on
the native side of that bridge.
