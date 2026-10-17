# Clarify GameBanana README and project independence

## Context

The GameBanana upload needs a small information ZIP, and both projects need an explicit independence statement.

## Changes

- `README.md` (modified): clarifies that EMMM is not affiliated with any named game or its publisher.
- `dist/gamebanana/README.txt` and `EMMM2-GameBanana-README-only.zip` (local, ignored): provide project and support links in a one-file ZIP.
- `3dm-catalog-asset/README.md` (modified in sibling repository): adds the same independence statement.
- `3dm-catalog-asset/dist/1.4.0/README-GAMEBANANA.txt` and its outer ZIP (local, ignored): add the statement without changing the signed catalog pack.

## Goal

Readers can identify the official project links and understand that EMMM and its catalog are independent.

## Impact

Documentation and local upload artifacts only; no application behavior changes.
