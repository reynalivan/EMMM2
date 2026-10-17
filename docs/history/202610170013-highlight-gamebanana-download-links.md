# Highlight README-only GameBanana download

## Context

The GameBanana upload needs an immediate warning that its ZIP is a link guide, not an installer.

## Changes

- `docs/gamebanana-upload-readme.txt` (modified): adds an uppercase warning, ASCII framing, and emoji-marked homepage and download links.
- `dist/gamebanana/README.txt` and `dist/gamebanana/EMMM2-GameBanana-README-only.zip` (regenerated, ignored): package the revised one-file guide.

## Goal

Readers can identify the official download location without mistaking the GameBanana ZIP for the application.

## Impact

Documentation and local packaging only. No application behavior changes.
