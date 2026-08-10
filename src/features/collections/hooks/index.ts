/**
 * Collections Feature Hooks — Barrel File
 *
 * This module exports the UI data-fetching and state management hooks for the Collections system.
 *
 * Note on Feature Boundaries:
 * The Collections system is tightly coupled with the Corridor (Virtual Filesystem Projection),
 * Safe Mode (Corridor Switching), and PIN (Safe Mode Security). While these concerns are separated
 * in the backend (collection_service, disk_reconcile, pin_service), they are cohesive in the UI.
 *
 * Consumers outside of the `collections` feature (such as `ContextControls` or `PrivacyTab`)
 * should import from this barrel file to avoid deep imports.
 */

// Collection Management Hooks
export * from './useCollections';

// Corridor & Projection State Hooks
export * from './useCorridor';

// PIN Security Hooks
