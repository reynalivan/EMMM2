# Collection and Safe Mode stability

Authorized by the user's instruction to repair the collection audit findings.

1. Add regression coverage for parent activation, missing/Object preview changes,
   structured rename errors, and recoverable Safe Mode transitions.
2. Include enabled children of parents being activated in collection diff planning.
3. Preserve live selection across Safe Mode using a persisted requested snapshot;
   keep transition tasks open until the config flag is persisted, and replay the
   recorded target/rollback intent during recovery.
4. Make preview wait for coherent disk projection and display missing mods and
   Object changes explicitly.
5. Repair incomplete query/mutation test fixtures blocking TypeScript compilation.
6. Run focused frontend/Rust tests, frontend build, formatting/lint checks, browser
   verification through the existing fixture-only demo, and document results.

Keep filesystem mutations inside the existing operation lease and journal.
Use additive migrations, existing dependencies, and the current checkout.
