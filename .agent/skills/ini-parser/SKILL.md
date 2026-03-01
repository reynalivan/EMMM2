---
name: ini-parser
description: Specialized Line-Based parser for 3DMigoto INI files. Use for reading/writing configuration where standard INI parsers fail due to duplicates, global variables, and comments.
---

# INI Parser Skill (3DMigoto)

Implements **Lossless Editing** for 3DMigoto (`d3dx.ini`) files.
**Standard parsers (serde_ini, configparser) WILL FAIL/CORRUPT these files.**

## Implementation Strategy

### 1. Read (Line-Based)

Do NOT parse into a HashMap. Read as `Vec<String>`.

- **Preserves:** Deduplicate sections (`[TextureOverride]`), Comments, Formatting.

### 2. Modify (Regex Targeting)

Target specific lines using Regex patterns.

- **Variables:** `^\s*(\$variable)\s*=\s*(.*)`
- **Key-Value:** `^\s*(key)\s*=\s*(.*)`

### 3. Write (Atomic)

Join lines with `\n` and use `atomic-fs` to save.

## Reference

> **Syntax Quirks:** See [syntax_guide.md](references/syntax_guide.md) for details on Duplicates & Global Vars.

## Examples

> **Code:** See [parser_test.rs](examples/parser_test.rs) for correct implementation pattern.
