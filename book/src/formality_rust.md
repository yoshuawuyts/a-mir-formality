# `formality_rust`: the Rust model

`formality_rust` is the Rust-specific layer of a-mir-formality. It is organized into three main subsystems:

* `grammar/` defines Rust-specific terms
* `check/` runs semantic checking
* `prove/` discharges logical obligations

## `grammar/`

The `grammar` module defines the Rust terms used by the model: crates, items, types, where-clauses, trait references, and the MIR-like expression language used in function bodies.

Function signatures can carry the experimental annotation `with const`, as in
`fn prepare() with const {}`. Write it after the return type (or parameter list
if the return type is omitted), before any `where` clause. Like `where`
predicates, effects are stored as a list, empty when the clause is omitted.
This is notation only: checking the annotation does not enforce const rules,
and Rust lowering and MiniRust code generation do not support it yet.

Use `with const { ... }` as a nested block statement. The block has its own
local scope, just like an unannotated block; its annotation does not affect
the enclosing function. This syntax does not introduce block values.

Closure forms such as `|| with const {}` and
`|value: i32| with const { return value; }` can be parsed and printed. Closure
values cannot yet be type-checked or executed.

## `check/`

The main checking entry point is `check_all_crates`:

{judgment}`check_all_crates`

It inserts an empty `core` crate if the first crate is not `core`, then checks every prefix of the crate list (treating the last crate in each prefix as the current crate).

Within a crate, checking rejects duplicate item names, checks each crate item, and then runs coherence checking:

{judgment}`check_crate`

The different kinds of crate items are dispatched by `check_crate_item`:

{judgment}`check_crate_item`

## `prove/`

The `prove` module answers Rust-specific goals such as where-clauses, equality, subtyping, and outlives. Checking code calls into `prove` whenever it needs to establish those facts. The main entry point is `prove_predicate`:

{judgment}`prove_predicate`

## Pipeline

At a high level, checking a Rust program looks like this:

```text
Crates
  -> check_all_crates
  -> item checking / borrow checking
  -> prove obligations as needed
```
