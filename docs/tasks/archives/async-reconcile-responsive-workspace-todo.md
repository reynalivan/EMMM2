# Checklist: Responsive Mods Workspace and Async Reconcile

- [ ] T1 Characterize stage latency, first-render blocking, scan count, and real-folder baseline
- [ ] T2 Add non-blocking recovery status/generation and backend mutation readiness guard
- [ ] Checkpoint A: one recovery per game/source; pending recovery cannot mutate disk
- [ ] T3 Emit factual throttled per-run progress with truthful units and ETA
- [ ] T4 Return last-valid or shallow provisional workspace model without awaiting recovery
- [ ] T5 Make watcher/internal reconcile truly scoped; retain full offline/overflow recovery
- [ ] T6 Keep list rendered during sync; add accessible progress UI and authoritative mutation gating
- [ ] Checkpoint B: list is responsive, progress visible/valid, stale runs ignored
- [ ] T7 Remove only measured duplicate work and tune disk concurrency/parser when proven
- [ ] T8 Run full online/offline mutation, watcher, conflict, collection, import, and source recovery matrix
- [ ] Verify time-to-first-shell/list, first progress event, event rate, and main-thread responsiveness budgets
- [ ] Run Rust/TypeScript/i18n/build/E2E/diff checks and focused final review
- [ ] Checkpoint C: disk/DB/runtime/UI converge without scan storm, partial DB state, or overengineering
