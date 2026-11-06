# The Policy Gate

A tiny 2D **metroidvania** where the *puzzle is ABAC*. Play the **Policy Keeper** in The Perimeters, inscribe attribute-based policies at every gate, let the right patrons in, and keep the minions out. Each zone unlocks a glyph that teaches a real ABAC concept.

## Run it

```bash
cargo run
# then open http://localhost:3000
```

## Run the tests

```bash
cargo test
```

The engine and puzzle content are pure Rust with **assumption-led tests** — each test encodes a belief about intended behaviour, and when code fails we re-derive the assumption instead of editing the test.

## Project shape

```
src/
  main.rs      # topcoat entry + routing
  model/       # pure-Rust ABAC engine + tests
  game/        # levels, gates, patrons, minions, glyphs
  view/        # topcoat-view templates
docs/
  ideation.md  # full design language (source of truth)
```

## Dependency note

Topcoat is pulled from **upstream crates.io** (`topcoat = "0.11"`), so a fresh clone builds and runs without local Path dependencies.

## What you'll learn

- Attribute axes (subject / object / action / environment)
- Policy composition: axes, operators, combinators
- Denial precedence & indeterminate outcomes
- Decision tracing (the "why" behind a decision)
- Policy lifecycle: testing, auditing, iterating
