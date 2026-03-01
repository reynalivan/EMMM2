---
name: code-documentation
description: Writing effective code documentation - API docs, README files, inline comments, and technical guides. Use for documenting commands, components, or writing developer guides.
---

# Code Documentation Skill

Standards for documenting the **EMMM2** codebase.

## 1. Rust Documentation (Backend)

- **Public Items**: MUST have `///` doc comments.
- **Commands**: specific `#[tauri::command]` documentation.
- **Modules**: `//!` module-level docs for major services (`src-tauri/src/services/mod.rs`).

## 2. TypeScript Documentation (Frontend)

- **Components**: Use TSDoc `/** ... */` for Props and Component description.
- **Hooks**: Document `params` and `returns` clearly.

## 3. Architecture Decisions (ADR)

- **Major Decisions**: Create an ADR in `.docs/adr/`.
- **Pattern**: Context -> Decision -> Consequences.

## References

- [Rust Style Guide](references/rust_docs.md)
- [TSDoc Style Guide](references/ts_docs.md)

## Templates

- [Standard README](templates/README.md)
- [Architecture Decision Record](templates/ADR.md)
