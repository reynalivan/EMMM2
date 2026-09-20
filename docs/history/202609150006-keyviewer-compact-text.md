# KeyViewer compact text layout

## Context

The viewport-anchored KeyViewer became too large and its glyphs looked soft after
the previous scale increase.

## Changes

- Restored the status text to native font scale and slightly reduced character text scale.
- Gave the status bar and character panel separate compact horizontal bounds.
- Kept left alignment and viewport-based anchoring unchanged.
- Bumped the generated layout revision so existing runtime artifacts are republished.

## Validation

- Updated generator assertions for the compact geometry and scales.
