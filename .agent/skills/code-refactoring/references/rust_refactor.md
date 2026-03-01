# Rust Refactoring Patterns

## 1. Idiomatic Rust

- **Expression-Oriented**: Prefer `if config.is_valid() { Ok(()) } else { Err(...) }` over mutable variables.
- **Error Handling**: Use `?` operator instead of `match` nesting for error propagation.
- **Clippy**: Run `cargo clippy` and apply suggestions (`cargo clippy --fix`).

## 2. Extracting Modules

If a file > 350 lines:

1.  Identifty cohesive structs/impls.
2.  Move to a new file in `src/`.
3.  Expose via `pub mod`.

## 3. Reduce Nesting (Guard Clauses)

**Before:**

```rust
fn process(item: Option<Item>) {
    if let Some(i) = item {
        if i.is_valid() {
            // ... logic
        }
    }
}
```

**After:**

```rust
fn process(item: Option<Item>) {
    let Some(i) = item else { return };
    if !i.is_valid() { return };
    // ... logic
}
```
