# The Policy Gate — Ideation & Design

**Source of truth** for the game, engine, architecture, and the *why* behind decisions.

## 1. Core idea

A minimal 2D "accessed-realm" platformer where the *puzzle* is literally writing **ABAC** policies.

- You play the **Policy Keeper** in a shattered digital realm, **The Perimeters**.
- Every **gate** is an access request: a candidate (subject) wants to touch an object, do an action, in a context.
- The gate stays locked until you **inscribe a policy** yielding the correct decision: allow the *patrons* in, block the *minions* out.
- Solving each zone unlocks a **glyph** — a mechanic mapping to a real ABAC concept (environment auth, denial precedence, relation graphs, …).

The game *is* the lesson.

## 2. Game design

**Story & tone.** The Perimeters: floating crystalline data-wards where information flows. Access was once role-based and broke. You are the **Policy Keeper**, restoring *principled, attribute-based* access. Low combat, high precision.

**Protagonist.** The **Keeper** — small, fast, unarmed. Can *read a gate* and *inscribe a policy tile* on its wall. Tight platformer traversal: walk, dash, wall-jump.

**The Gate puzzle.** A gate shows **Subject** (name, role, clearance, device), **Object** (chart, file, server), **Action** (read/write/delete), and **Context** (time, location, network trust).

Two hidden rules decide pass/fail: **Patrons** → must get in; **Minions** → must be blocked.

You build a policy from **attribute axes** + **operators** (`=`, `>=`, `in`, `matches`, `time-of-day`) + **combinators** (`and` / `or` / `deny-override`). On deploy you get a live **decision trace** (which attributes matched and why). All patrons pass **and** all minions fail ⇒ the gate opens.

**Zones → abilities.**

| Zone | ABAC concept | Player glyph unlocks |
|---|---|---|
| The Atrium | Subject + object + action | Policy ruler (`=`, `>=`, `and`) |
| The Wards | Environment (time, location, device) | Context window (`time`, `location`) |
| The Spire | Relationship + precedence | Deny-override, relation graph |
| The Core | Conflict + indeterminate | `permits-unless`, conflict resolution, auditor mode |

## 3. Technical architecture

**Stack.** Topcoat (`topcoat = "0.11"`), a server-rendered full-stack Rust framework. Engine, levels, and state live in **pure Rust**, evaluated **server-side** — easily unit-tested. The playfield is an HTML5 `<canvas>` driven by a browser `requestAnimationFrame` loop; it sends actions to the server, renders the result.

**Layout.**

```
policy-gate/
├── Cargo.toml
├── src/
│   ├── main.rs      # topcoat entry + module_router
│   ├── model/       # pure-Rust ABAC engine + tests
│   ├── game/        # levels, gates, patrons, minions, glyphs
│   └── view/        # topcoat-view templates (shell, panel, editor, HUD, mini-map)
├── docs/ideation.md
└── README.md
```

**Dependencies** — upstream crates.io for portability:

```toml
topcoat = { version = "0.11", features = ["view", "router", "serve", "runtime"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "signal"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1"
```

## 4. The ABAC engine model

**Attribute axes.** Four axes:
- **Subject**: role, clearance, department, device, risk…
- **Object**: type, sensitivity, owner, record…
- **Action**: read, write, delete…
- **Environment**: time-of-day, location, network-trust…

**Policy IR.** A policy is a list of rules: `axis` (subject/object/action/environment) + `attribute` key + `operator` (`=`, `!=`, `>=`, `in`, `matches`, `time-of-day`) + `effect` (`Permit` / `Deny`). Rules may be combinable into sub-trees.

**Evaluation → decision.** Evaluate each rule against the candidate's attributes. Matched rules yield `Permit` or `Deny`. **Denial precedence**: a `Deny` wins over `Permit`, unless a later rule explicitly redefines it (`permits-unless`). No definitive outcome ⇒ `Indeterminate`. Return the verdict plus a **trace** of checked attributes and matched rules (the educational "why"), e.g., why `time-of-day = "23:00"` → `Deny`.

## 5. Testing philosophy — assumption-led, not "write a test"

Every test encodes an **assumption** about intended behaviour. On failure we *redo the assumption*, not the test. Examples:
- *Zero matching conditions ⇒ `Indeterminate`.*
- *A `Deny` on the subject axis blocks even when others permit.*
- *Environment `time-of-day` compares against the policy's stored context, not wall-clock time.*
- *No combiner outcome ⇒ `Indeterminate`, listing conflicting sub-decisions.*
- *Golden-master: a built-in zone's patron/minion set yields the expected verdict + trace.*

Loop: **encode assumption → run test → on failure step back, re-derive assumption → fix the engine → re-run.** Never edit a failing test to pass.

## 6. Design system (Refactoring-UI)

| System | Values |
|---|---|
| Spacing | `4, 8, 12, 16, 24, 32, 48, 64, 96, 128` px |
| Type scale | `12/14/16/18/20/24/32/48` px; weight + color first, size last |
| Palette | ink `#0B1020` → `#101828`; glyph cyan `#5EEAD4`; amber `#F5A524`; slate text |
| Depth | two-part shadows; elevation tiers |
| Finishing | custom focus rings; labels a last resort; notched empty states |

## 7. Roadmap / commits

| Commit | Work |
|---|---|
| #1 | Scaffold + portable crates.io deps + minimal topcoat entry |
| #2 | `docs/ideation.md` + `README.md` |
| #3 | ABAC engine + assumption-led tests |
| #4 | Topcoat shell + canvas playfield + gate panel |
| #5 | Level 1 (Atrium): patrons/minions wired to server engine |
| #6 | Polish + design system + deploy |

## 8. Open questions / risks

- **Deny-precedence**: chose "deny wins over permit unless a later rule redefines it." Revisit this single assumption (and its test) if a specific ABAC profile disagrees.
- **topcoat 0.11 API**: verified compiling; may need small `main.rs` tweaks.
- **wasm32 target**: not installed; not needed for the server-driven design.
