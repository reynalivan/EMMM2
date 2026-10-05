# Implementation Plan: Stable, Responsive Disk-First Mod Switching

Tanggal: 2026-10-04. Basis audit: `main`, commit `218c31d`.

Status: **implementation complete; automated gates passed; manual release acceptance remains**. Sisa writer/identity integration sudah dituntaskan tanpa dependency, schema DB, scheduler, atau mutation lock baru. Checkbox hanya ditandai jika pekerjaan dan bukti sesuai scope-nya tersedia; full UI/storage matrix bukan klaim dari fixture otomatis.

Follow-up plan: [Native acceptance, consumer consistency and measured performance closure](#9-native-acceptance-consumer-consistency-and-measured-performance-closure) disusun pada 2026-10-05 dan dilanjutkan pada 2026-10-06; **native rerun dan final gates sedang berjalan**. Status implementation complete di atas merujuk slice kode yang sudah selesai, bukan seluruh penerimaan native pada follow-up ini.

## Execution evidence — 2026-10-04

### Approved coordination follow-up

- [x] Preserve scoped reconcile errors instead of converting them into a full scan; only unproven scope selects full discovery. Preserve distinct source-unavailable/user-resolution handling and owned checkpoint validation.
- [x] Show a coalesced, current-intent sync-repair warning after disk acknowledgement without rolling back the switch or relocking UI.
- [x] Add disk/SQLite/consumer integration regressions and correlated projection/runtime diagnostics. Keep measurement boundaries explicit; native UI/bulk fairness is not inferred from synthetic admission timing.
- [x] Retain conservative mixed leaf/subtree runtime fallback; record when it selects full composition before considering incremental merge.
- [x] Run targeted/full gates and focused review; record results and remaining acceptance limits.

- Scoped DB failure now retries the proven scope rather than scanning the entire game; typed source-unavailable and rename-confirmation results are not erased. Real disk/SQLite/journal regressions verify a rapid burst survives projection failure, remains mutable after disk acknowledgement and converges after retry. Collection capture, current preview and saved apply preview agree on disabled ancestor semantics.
- Rename-confirmation ownership is partitioned by overlapping operation roots; Object/top-level ambiguity remains deliberately broad. Existing native replacements isolate affected child/parent lineage for repair instead of polling indefinitely; a missing target remains retryable. Independent operations complete without clearing unrelated dirty authority or crossing a repair-hole checkpoint. Both new classification regressions failed before the fix and pass with real disk/SQLite/journal evidence.
- Repair warnings are bounded to 64 recently shown game/root keys with a 30-second dedupe window and current-intent notification guard. Same-epoch revisit resumes idle acknowledged receipts through the existing shared scheduler, skips historical path rewrites and fences record pointer/intent/disk revision/epoch. Lifecycle cleanup prevents an old completion from clearing state after a fast A→B→A return. Two hooks under StrictMode cover node/path revisits, successor clicks, early return and absence of further refresh after settlement; inactive bulk settlement semantics are unchanged.
- Debug diagnostics correlate projection game/epoch/disk revision, proven-vs-full scope, lease wait and elapsed time; runtime diagnostics report publication generation/permit wait and elapsed time. Mixed leaf/subtree composition remains conservatively Full, now with a recorded reason. No new queue, tracker, error DTO, schema or dependency was added.
- Follow-up final gates: Rust **1.399 passed / 14 ignored**, frontend **1.132 passed / 1 skipped** across 205 test files; TypeScript, full ESLint, architecture lint, Vite production build, rustfmt, Clippy all-targets `-D warnings` and focused read-only re-review passed. Two redundant clones in new test assertions were replaced with borrowed slices after Clippy feedback; no warning was suppressed. Measurement/native release limits below remain unchanged.

### Prior core slice

- Physical path admission memakai filesystem identity + root epoch, exact aliases, dan revalidation setelah menunggu lease. Relative/absolute requests berbagi target yang sama; dua folder nyata prefix-equivalent tidak saling supersede. Object batch yang belum mulai mempertahankan participant non-overlap.
- Core completion mempunyai proven root identity terpisah dari projection/dirty state. Known leaf tidak menunggu projection lag atau dirty root lain; target/ancestor dirt dan coverage gap tetap membutuhkan repair.
- Watcher membedakan planned/committed evidence, mengikuti successor/ancestor lineage, dan mempertahankan dependency saat watermark pruning. RED reverse/ancestor regressions terbukti gagal sebelum fix; 20 suppressor tests termasuk 1.000 physical temporary-directory renames lulus. Replay tersebut bukan live debouncer stress.
- Projection ownership pindah ke reconciliation API. Pure scope/legacy proof di application; AppHandle-dependent orchestration tetap di Tauri adapter sesuai aturan arsitektur. Epoch difence pada receipt, snapshot, journal, checkpoint, dan publication.
- Journal/active format 3 membaca format 1/2. Foreign-root acknowledged commits menjadi projection `NeedsRepair`, tetap nonterminal dan durable, tidak masuk normal retry atau history trimming, serta tidak di-rollback. Regression reopen/recovery lulus.
- Frontend memakai shared target record, satu snapshot IPC aktif/game, satu refresh owner/game/epoch, union scopes/latest revision, dan retry bersama. Hanging snapshot tidak membuat IPC paralel; 100 failed receipts tidak membuat 100 retry owners. Transient epoch/cancellation/query failure pulih tanpa memblokir switch.
- Immutable collection/conflict/Safe Mode/preset transactions dibatasi empat running-or-queued/game sebelum lock wait; ordinary latest-desired toggles tidak terkena limit.
- Prior core-slice verification: Rust **1.352 passed / 12 ignored**, frontend **1.117 passed / 1 skipped**; TypeScript, full ESLint, architecture lint, Vite build, rustfmt, Clippy all-targets `-D warnings`, binding export/permission tests, dan focused read-only review lulus. Final completion evidence below supersedes these counts.

### Completion slice — 2026-10-04

- Audited collection capture/apply, parent activation, conflict fix/trash, randomizer, Safe Mode/preset, import, organizer, delete/rename and metadata. Structural/randomizer admission uses the same bounded permit; physical root/source proofs survive lock waits. Current-state capture retains game plus operation leases. Safe Mode reads/captures/decides empty only after that coherent barrier, without a second admission permit or nested lease.
- Runtime scope reservation is synchronous from known old/new rewrite roots before lease release. Optional SQL leaf derivation/INI harvesting runs in the existing worker; no new queue or scheduler.
- Removed duplicate organizer implementation. Rename/publication/compensation use no-overwrite plus ownership validation; disk metadata is retained when only projection fails.
- Listings carry validated native identity through node/path, object-proof and frozen bulk IPC. Shared frontend records use physical identity and an exact-path index; replacement folders cannot inherit another record merely by reusing a DB ID/path. Bare path/DB-object ownership starts at backend capture, before lease waits; original/configured root and prepared participant proofs are retained through subset retries.
- Rename healing selects exact source rows plus descendants and validates every target's native identity. Object updates use the verified row ID; prefix-normalized siblings, NULL identity descendants and replacement children cannot yield a false checkpoint. A rowless container is handled through its proven descendants.
- A same-epoch `NeedsRepair` hole caps worker/snapshot checkpoints. Coherent collection/Safe Mode barriers also inspect repair evidence when pending work is empty. Proof-based closure follows completed successor lineage and includes its validated revision, without clearing repair blindly or copying an unvalidated pending highwater. Frontend repair snapshots stop the shared poll/refresh waiter with an actionable error while preserving disk observations; a later repaired snapshot can converge normally.
- Toggle collision checks concern the actual destination, not another prefix-equivalent folder. A request-scoped name index retains all folded candidates; successful rename/retries do not rescan siblings. Native source validation plus atomic no-overwrite remain mandatory, with a fresh collision census only on failure. Legacy logical naming policy for organizer/import/rename is unchanged. Raced namespace exclusion follows the OS, not a virtual case-insensitive lock on case-sensitive filesystems.
- Cross-volume import returns and retains the published target identity through rollback, rather than recapturing a replacement. An interrupted copy that lacks durable target ownership remains `FailedNeedsRepair` with payload/evidence preserved; no new journal format or guessed destructive recovery was added.
- Existing live collection, ancestor and KeyViewer consumers already use effective-active semantics; historical preset membership remains independent of current state. No parallel consumer truth/cache was added.
- Native Windows watcher tests use actual `notify`/debouncer and same-watcher takeover, four slow plus 41 rapid renames, and replacement followed by rename/INI changes. Verified internal echoes or explicit dirty repair are accepted; native event coalescing is not assumed to preserve every intermediate edge.
- Subprocess termination/reopen exercises eight journal boundaries: before plan, planned, applying, renamed/unsettled, settled, disk committed, DB committed and completed. Recovery is checked twice for idempotence; pre-ack compensation and post-ack retention preserve folder identity/payload. This does not simulate power loss midway through an OS write or a real SQLite checkpoint crash.
- Final completion gates: Rust **1.392 passed / 14 ignored** (1.406 total); frontend **1.124 passed / 1 skipped**, 205 test files. TypeScript, full ESLint, architecture lint, Vite production build, rustfmt, Clippy all-targets `-D warnings`, generated binding/permission tests and focused read-only review passed. The test-only validation wrapper is compiled only under `cfg(test)`; production calls the same ownership validator directly. No warning/check was suppressed.
- Validation findings were resolved rather than hidden: the organizer regression fixture now persists the native identity supplied by a real core index and still verifies deferred collection/path updates; no production guard was weakened. An unchanged frontend listener timing test hit its timeout under concurrent heavy validation, passed in isolation and passed again in the final full run with four workers; its timeout was not increased.

### Measurement and remaining gates

Manual existing journal I/O benchmark lulus pada temporary files Windows: tujuh samples/cell, 100/1.000/10.000 steps, history 1/64/256, successful commit dan partial rollback. Pada history 64, successful-commit p95 masing-masing **23,081 / 49,399 / 312,967 ms**; lima writes/sample. Ini characterization journal, bukan physical toggle/IPC/paint benchmark dan bukan before/after speedup. Metadata CPU/storage tidak tersedia karena diagnostic CIM access ditolak.

Native physical leaf benchmark uses 100 timed samples after eight warmups/cell, actual rename/identity validation and durable `DiskCommitted`, isolated local temporary folders on Windows, debug build, i7-13700K and PNY CS2241 500 GB NVMe (C:). Timing excludes IPC/UI and post-ack projection/cleanup; SQL/runtime are not invoked. Sibling lookup previously fetched metadata for every entry twice; streaming names/cached entry types plus removal of the redundant successful-apply census removes that work without a persistent cache or weaker atomic overwrite guard.

| Entries | Layout | Before p95 / p99 (ms) | Final repeat p95 / p99 (ms) |
| ------- | ------ | --------------------- | --------------------------- |
| 100     | flat   | 19.172 / 20.936       | 17.742 / 22.594             |
| 100     | nested | 14.715 / 16.635       | 22.987 / 24.204             |
| 1,000   | flat   | 284.912 / 462.597     | 24.351 / 24.782             |
| 1,000   | nested | 16.400 / 19.486       | 17.628 / 21.265             |
| 10,000  | flat   | 486.858 / 594.029     | 22.600 / 25.050             |
| 10,000  | nested | 20.735 / 24.584       | 15.623 / 19.863             |

Admission plus lease p95 final repeat: 0.113–0.171 ms. The intermediate streaming-only repeat measured flat 10k p95/p99 36.526/40.229 ms. After removing the successful-apply census, the first final run measured flat 10k 21.294/23.763 ms but nested 10k had a 139.784/206.449 ms outlier (an earlier intermediate run also had a 196.214 ms p95 outlier). All six final cells were rerun; the repeat is reported above. All reported runs passed correctness assertions. Host I/O variance remains visible: these are not universal latency guarantees or UI end-to-end measurements, and the first nested final run did not meet the 100 ms p95 target.

Belum dibuktikan: full native UI rage-click/click-to-paint, complete persistent OS sharing/ACL/power-loss matrix, cold/network storage, serta mixed bulk throughput/fairness/peak memory untuk 10k selection sambil projector/runtime tertahan. Jangan tandai keseluruhan done criteria hijau berdasarkan mock/replay atau leaf/journal timing.

Scope decisions: immutable writers capture/revalidate physical proofs in the backend rather than adopting a new global opaque DTO framework. Existing listing physical identity is carried through the consumed switch IPC seam, including frozen bulk selection. Bare path/DB-object entrypoints start ownership proof at backend admission; object proofs survive lock waits and subset retries. Capture/coherent collection barriers remain stricter than leaf toggles. Runtime reservation still precedes lease release so an old collection cannot override a newer toggle; only optional derivation was moved to the worker.

Plan ini melengkapi [storage-first switching](../storage-first-switching/implementation_plan.md) dan [priority-game indexing](../priority-game-indexing/implementation_plan.md). Fondasi journal, identity validation, latest-intent admission, scoped reconcile, watcher takeover, dan storage receipt yang sudah ada dipertahankan. Temuan historis dalam plan lama tidak dianggap masih berlaku seluruhnya.

## 1. Goals dan batas scope

Urutan prioritas wajib:

1. UI menerima setiap klik dan langsung menampilkan **desired state terbaru**.
2. Setelah safety check minimum, backend memprioritaskan **rename folder fisik**, memvalidasi hasil disk, lalu mencatat commit secara durable.
3. Command mengembalikan bukti disk, tanpa menunggu DB projection, runtime, collection descriptor, KeyViewer, thumbnail, atau query refresh.
4. Consumer menyusul berdasarkan identity, epoch, dan revision yang sama; hasil lama tidak boleh menimpa hasil baru.

Target stabilitas:

- Last wins per target fisik, termasuk grid, preview, context menu, object, bulk, dan operation yang overlap.
- Tidak ada self-conflict karena path alias, delayed watcher echo, atau state DB tertinggal.
- Core indexing yang selesai tidak kembali menjadi belum-index hanya karena projection/refresh tertunda.
- Watcher tidak kehilangan external rename dan tidak menganggap internal rename berantai sebagai perubahan eksternal tanpa bukti.
- Setelah input berhenti dan dependency sehat, disk, DB, collection, ancestor status, dan KeyViewer konvergen.
- Pending desired-state work bounded terhadap jumlah target/scope, bukan jumlah klik; non-coalescible transactions mempunyai bounded admission tersendiri.

Non-goals: rewrite seluruh engine, generic event bus/scheduler, dependency baru, parallel mutation locks baru, visual redesign, atau perubahan schema DB tanpa kebutuhan terukur. Push/build installer tidak termasuk tahap plan ini.

Responsif bukan berarti mengabaikan source replacement, collision nyata, permission, atau OS sharing lock. Multi-folder operation tidak atomik secara filesystem; journal/compensation tetap dibutuhkan. Jangan menjanjikan nol latensi disk pada semua storage.

## 2. Temuan audit yang mendasari perubahan

| Area                 | Bukti kode saat audit                                                                                                      | Gap yang ditutup                                                                                                                                                                  |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Intent identity      | `mutation/coordinator.rs`: `IntentKey::new`, `logical_path_key` menghapus prefix disabled pada setiap komponen             | `Mods/A/Skin` dan `Mods/DISABLED A/Skin` yang benar-benar berbeda dapat berbagi key. Alias path tidak boleh menjadi bukti identity fisik.                                         |
| Watcher echo         | `watcher/suppressor.rs`: verifikasi identity pada historical `new_path`                                                    | Pada `A -> DISABLED A -> A`, destination intermediate sudah hilang saat echo lama tiba. Echo internal dapat jatuh menjadi external dirty event.                                   |
| Readiness            | `disk_reconcile/orchestrator/state.rs`: `initial_recovery_readiness`; `watcher/lifecycle.rs`: activation acceptance        | Core completion, authority dirty, dan pending sync mempunyai definisi eligibility yang tersebar. Repair eligibility sudah ada; perlu satu kontrak konsisten.                      |
| Projection ownership | `workspace/adapters/tauri/workspace_cmds.rs`: `queue_toggle_projection` dan worker                                         | Application orchestration hidup di adapter dan retry dapat terus mengulang ketika coverage tidak tersedia.                                                                        |
| Epoch fencing        | Journal, projected event, dan frontend projection waiter                                                                   | Disk revision sudah monotonic; bukan reset-counter bug. Tetapi receipt/event/waiter belum konsisten mengikat root epoch sehingga root replacement perlu hardening.                |
| Frontend ownership   | `useWorkspaceSwitchActions.ts`: beberapa pending/desired/version/override maps; `workspaceSwitchOps.ts`: waiter per result | Key sudah game-scoped, tetapi lifecycle node/path/bulk terpisah. Banyak waiter dapat mem-poll game sama; refresh rejection dapat meninggalkan override tanpa recovery yang jelas. |
| Refresh impact       | `optimistic/descriptorBuilders.ts`: folder-switch fallback; berbagai consumer                                              | Scope-specific refresh sudah ada. Perlu coalescing revision/scope lintas operasi, bukan menganggap setiap toggle sekarang selalu full scan.                                       |

Gap di atas adalah risiko struktural dari kode. Audit belum membuktikan p95 native, crash safety semua boundary, atau reproduksi data loss pada folder pengguna. Plan mensyaratkan fixture untuk membuktikannya.

## 3. Flow target dan invariant

```text
Input -> shared desired-state record -> latest-intent admission
      -> existing mutation lease + core eligibility + target disk validation
      -> durable journal plan -> no-overwrite rename -> disk validation
      -> durable DiskCommitted -> disk receipt -> UI confirmed-path patch
                                      |
                                      v
                           managed journal-backed projection
                           -> DB checkpoint -> coherent query snapshot
                           -> collection / ancestor / runtime / KeyViewer

Watcher -> identity-aware expected-echo lineage
        -> consume proven internal echoes
        -> union genuine external dirty scopes -> same reconcile owner
```

Invariants:

1. Path adalah location hint; physical identity adalah ownership. Lookup prefix-normalized tidak boleh memilih sibling pertama ketika ambiguity ada.
2. Intent revision, disk revision, core epoch, dan runtime generation berbeda makna; tidak saling menggantikan.
3. Desired UI bukan disk truth. Success toast hanya setelah disk outcome terverifikasi dan durable.
4. DiskCommitted yang sudah diakui tidak dibalik hanya karena projection/runtime gagal.
5. Semua writer dan reader memakai root/identity fencing yang sama; old-root work tidak boleh publish ke root baru.
6. Publication/checkpoint hanya mengakui scope yang benar-benar selesai; supersession tidak boleh membuang scope target lain.
7. Saat watcher coverage hilang atau identity ambiguity, sistem fail closed pada safety yang terpengaruh dan menawarkan repair yang jelas; tidak memalsukan readiness/complete.

## 4. Kontrak teknis yang diubah

### 4.1 Identity dan latest intent

- Reuse filesystem identity yang sudah tersedia. Key target: `(gameId, rootEpoch, physicalIdentity)`; object/batch membawa participant identities dan scope hierarchy.
- Identity mencakup namespace volume/root yang diperlukan; nama folder, stripped path, dan inode/file-id tanpa scope tidak cukup.
- UI meneruskan native identity dari validated listing/receipt melalui existing DTO; tidak menambah global opaque-reference framework. Path-only entrypoint terlebih dahulu resolve identity, bukan menjadikan prefix alias sebagai admission key. ObjectId tanpa supplied proof mengambil bukti physical root sebelum lock wait; ownership tidak diklaim sebelum admission.
- Validation di bawah lease tetap memastikan source identity, root containment, destination occupancy, dan ancestor relationship. Identity yang hilang/replaced tidak otomatis di-rebind ke folder pengganti.
- Satu target mempunyai satu running operation dan satu desired intent terbaru. Klik pertama langsung dispatch, tanpa debounce rename. Klik berikutnya mengganti pending desired state.
- Opposite intent yang datang saat rename berlangsung diproses setelah outcome aman. Result lama boleh memperbarui disk observation, tetapi tidak menghapus desired revision yang lebih baru.
- Alias map hanya menghubungkan lokasi lama/baru yang terbukti milik identity sama. `identityPathKey` tetap boleh dipakai untuk alias/cache kompatibilitas, bukan uniqueness fisik.

### 4.2 Core readiness, disk authority, dan sync progress

Satu application owner menyediakan snapshot eligibility dan backend preflight; frontend tidak membuat aturan readiness kedua.

| Dimensi                       | Makna                                                 | Efek pada switch                                                                                                                                 |
| ----------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Core lifecycle per root epoch | Unindexed / Indexing / Indexed / Failed               | Root yang belum selesai core tetap diblokir; optional indexing tidak menghalangi.                                                                |
| Disk authority                | Clean / Dirty(scopes) / CoverageGap / Unavailable     | Scoped validation/repair sesuai kebutuhan. Dirty unrelated scope tidak otomatis memaksa reindex game penuh.                                      |
| Projection progress           | Required DB checkpoint dan pending journal work       | Tidak memblokir normal disk-first toggle yang bisa dibuktikan aman dari disk+journal. Snapshot-sensitive operation dapat memakai barrier khusus. |
| Derived progress              | Runtime, collection descriptor, KeyViewer, UI queries | Status sync terpisah, tidak membuat switch menjadi loading/disabled.                                                                             |

- Core Indexed stabil dalam epoch yang sama; invalidate hanya karena root/config identity berubah atau bukti core tidak lagi valid, bukan karena event refresh biasa.
- Dirty ancestor/target scope diperiksa sebelum rename. Coverage gap tidak boleh dianggap Clean: repair/checkpoint yang memulihkan continuity tetap wajib.
- Semua entrypoint memakai selector/guard sama: single, path, object, bulk, collection apply, conflict fix, hotkey, randomizer, dan writer lain yang overlap.
- Onboarding membuka dashboard setelah core game pertama dan watcher continuity terbukti. Game lain background; saat membuka game belum-ready, promote/join existing indexing dan tampilkan loading core game itu saja.
- Getter status tetap pure. Start/join/promote/retry hanya melalui API eksplisit, tidak dipicu diam-diam oleh status query.

### 4.3 Disk receipt dan epoch fencing

Extend typed result yang ada, bukan membuat command family baru tanpa kebutuhan.

Receipt membawa operation ID, game ID, root epoch, intent revision, disk revision, resolved target identity/reference, current path/path rewrites, verified local-enabled state, dan sync status terpisah. Bulk membawa summary serta hasil sesuai kontrak existing; full journal evidence tidak harus dikirim ulang ke UI tanpa batas.

- Bedakan local-enabled dengan effective-active: `localEnabled && allAncestorsEnabled`. Collection active preview dan KeyViewer menggunakan effective-active, bukan local flag saja.
- Bind epoch pada journal record, projection checkpoint/event/snapshot, receipt, dan frontend waiters. Watcher session ID terpisah dari root epoch.
- Reuse definisi root/source epoch existing; tentukan durabilitas/compatibility berdasarkan root identity/config version. Jangan membuat epoch ephemeral startup yang menyebabkan pending journal valid tidak bisa direcover.
- Journal format upgrade additive/versioned. Pending legacy record hanya boleh diadopsi setelah configured root dan participant identity dibuktikan; ambiguity ditahan untuk recovery, tidak diabaikan.
- Event dan snapshot memakai tuple fencing yang sama. Listener + snapshot handshake menutup event yang datang sebelum subscription; stale response tidak boleh acknowledge epoch baru.

### 4.4 Watcher expected-echo lineage

- Evidence scoped pada `(rootEpoch, watcherSession, physicalIdentity)` dan ordered rename edges dari operation plan/verified receipt.
- Registrasi sebelum rename tetap ada; evidence dibedakan planned, committed, atau aborted. Gagal rename tidak meninggalkan expected echo yang dapat menelan external event.
- Echo yang tiba ketika evidence masih planned di-buffer/defer sampai rename outcome terbukti, dengan batas eksplisit. Planned-only evidence tidak boleh langsung suppress event. Abort, buffer overflow, atau outcome yang tidak dapat diverifikasi menjadi dirty scope/repair.
- Echo historis cocok pada edge yang belum consumed, dengan proof identity di endpoint committed terbaru. Intermediate path yang sudah hilang karena successor bukan otomatis external.
- Bila intermediate destination kini berisi identity lain, event tambahan tidak cocok, lineage ambigu, atau coverage gap: jangan suppress; kirim scoped dirty/repair.
- Handle event paired/split/reordered dan partial batch rename dengan contract watcher existing; tidak semua event platform dapat dibuktikan sebagai internal.
- Tidak blanket-ignore subtree, tidak hapus seluruh echo target saat successor masuk, dan tidak memakai timeout sebagai bukti event sudah diproses.
- Retention bounded. Setelah verified reconcile/checkpoint melalui observed watermark, prune evidence yang sudah covered. Jika batas evidence terlampaui sebelum proof, eskalasi explicit repair, bukan silent drop.
- Pertahankan takeover watcher sejak sebelum initial census: buffer, watermark, dan transition session tidak boleh mempunyai uncovered interval atau membuang debouncer backlog.

### 4.5 Managed projection owner

- Pindahkan orchestration worker dari `workspace_cmds.rs` ke reconciliation application module yang cohesive; adapter hanya meneruskan command dan membentuk result/event.
- Reuse operation coordinator, durable journal, reconcile writer, dan runtime generation queue. Tidak membuat projector paralel yang juga menulis DB.
- Pending scope adalah union committed operations; coalesce work terbaru tanpa kehilangan changed scope target lain. Checkpoint hanya maju setelah required projection transaction sukses.
- Foreground intent mempunyai priority; existing lease/cancellation semantics dipertahankan. Tidak memegang storage-critical mutation lease saat menunggu optional work.
- Transient failure mendapat retry bounded/backoff. Coverage unavailable/permanent configuration issue mem-park work dengan reason dan wake condition, bukan full reconcile berulang setiap beberapa detik.
- Wake dari watcher recovery, new relevant evidence, explicit retry, atau dependency recovery. Required work tetap durable/pending; parking bukan complete.
- Runtime/KeyViewer publication fenced epoch + revision/generation; late job dibuang, affected scopes tetap dijadwalkan.
- Save current / coherent snapshot capture / collection apply memakai barrier khusus sesuai data yang dibutuhkan; barrier tersebut tidak menjadi global gate semua switch.

### 4.6 Satu frontend switch record dan projection tracker

- Consolidate pending maps menjadi satu record per target: latest desired+intent revision, in-flight reference, verified disk observation+path+revision, serta sync progress/error.
- Hook menjadi adapter ke owner bersama. Grid/preview/sidebar/path entrypoint tidak mempunyai desired truth terpisah.
- Satu projection tracker per game/root epoch, satu event subscription, dan maksimal satu snapshot poll aktif. Poll hanya ketika ada pending work, backoff bounded, lifecycle cancellation jelas.
- Stale receipt tidak menghapus new desired state; stale query tidak mengembalikan path lama. Listing/query publication dibandingkan checkpoint sebelum meng-clear overlay.
- Setelah disk commit, switch tidak menjadi loading control. Sync pending/error ditampilkan terpisah tanpa blocking normal next intent.
- Refresh failure mempunyai retry/revalidation path: retain verified disk observation, fetch/revalidate target disk snapshot, lalu reconcile record menurut revision. Jangan clear overlay setelah timeout sembarang atau membiarkannya pending tanpa status/recovery.
- Coalesce refresh affected scopes pada revision boundary, reuse query invalidation existing; no-op/superseded tidak memicu broad refresh.
- Context-menu Explorer resolve current physical path/identity dari confirmed observation atau backend resolver, termasuk disabled dan ancestor rename.
- Pertahankan toast aggregation 500 ms existing: hanya applied verified receipts, dedup operation/revision, stale-game suppressed, error actionable tidak tenggelam oleh success burst.

### 4.7 Conflict policy dan operation overlap

- Resource hash, runtime-key overlap, dan same-target advisory **tidak menolak enable/disable**. Advisory boleh menyusul, self identity dikecualikan dan result stale dibuang.
- Folder collision hanya jika destination namespace benar-benar ditempati **identity lain**. Nama sama beda parent sah; enabled/disabled spelling dari identity sendiri bukan dua folder.
- Source missing/replaced, root invalid, parent confirmation, permission, dan persistent I/O failure tetap safety blocker, bukan hash conflict.
- Conflict fix/dialog confirmation membawa expected identities/revision; revalidate ketika apply. Indexing/watcher tidak boleh menerapkan keputusan dialog lama pada folder baru.
- Bulk selection frozen pada listing revision; membership tidak meluas karena refresh. Existing best-effort vs compensated-batch semantics dijaga dan didokumentasikan per entrypoint.
- Batch yang sudah menjalankan rename harus selesai atau compensation/recovery terminal sebelum overlapping successor. Last wins bukan membatalkan transaction di tengah.
- Not-started batch hanya superseded utuh bila policy/membership setara. Partial overlap mempertahankan pekerjaan target lain; conflicting successor dijadwalkan dengan acceptance order per participant/scope.
- Desired-state batch yang policy-nya mengizinkan dapat dibangun ulang dari latest participant intents sebelum mulai. Collection/fix transaction yang tidak boleh direbuild memakai bounded operation admission: coalesce hanya request identik yang aman, lalu explicit superseded/rejected outcome ketika limit tercapai. Jangan diam-diam drop atau menjanjikan antrean arbitrary immutable snapshots tetap O(unique targets).
- Parent/child scopes tetap serialize dengan lock existing. Jangan tambah per-folder parallel locking pada refactor ini.

## 5. Tahapan implementasi dan file impact

### P0 — Reproduksi dan baseline sebelum perubahan

- [x] Tambah deterministic regression: dua identity pada prefix-equivalent paths, delayed reverse-rename echo, first toggle setelah onboarding, refresh rejection, dan root replacement.
- [ ] Rekam timeline intent accepted -> lease -> rename -> disk validation -> durable receipt -> projection -> query publication; operation/epoch/revision correlation tanpa logging path sensitif berlebihan.
- [x] Ukur native single-leaf fixture temporary 100 / 1.000 / 10.000 entries, flat/nested; pisahkan disk acknowledgement dari IPC/UI dan journal characterization.
- [ ] Ukur mixed bulk throughput/fairness/peak memory dan click-to-paint native UI. Ini acceptance measurement lanjutan, bukan klaim dari deterministic alias/selection tests.

Gate: reproduksi masing-masing gap atau catatan jelas bila hanya structural risk; baseline tersedia, tidak mengklaim native performance dari unit test.

### P1 — Identity dan receipt contract

- [x] Ubah admission/resolve identity pada `mutation/coordinator.rs` dan workspace switch preparation/command adapters.
- [x] Extend switch DTO/listing proof, journal/recovery, checkpoints/events dengan root epoch dan resolved identity; generated contracts/export tetap konsisten.
- [x] Sesuaikan frontend generated contracts dan path-only resolution. Alias tidak lagi admission identity.
- [x] Tes source replacement, ambiguous sibling, actual collision, same basename beda parent, restart/legacy journal.

File impact utama: `mutation/{coordinator,journal,recovery}.rs`, `workspace/domain/workspace/switch.rs`, `workspace/application/workspace/switch.rs`, `workspace/adapters/tauri/workspace_cmds.rs`, `src/shared/lib/pathKey.ts`, `src/lib/bindings.gen.ts`.

Gate: tidak ada cross-target supersession/overwrite; disk receipt/journal konsisten dengan epoch dan identity.

### P2 — Watcher lineage dan continuity

- [x] Implement identity-aware lineage pada `watcher/suppressor.rs`; event classification dan lifecycle consume contract sama.
- [x] Prune melalui watermark/checkpoint, bukan age-only success; handle aborted/partial operations.
- [x] Tes deterministic planned/abort/overflow, delayed/split/ambiguous lineage, replacement, handoff dan recovery; tambahkan actual Windows debouncer/takeover stress.
- [ ] Jalankan seluruh matrix platform/OS offline, persistent sharing, ACL dan power-loss pada environment target. Native fixtures tidak mengklaim semua delivery ordering OS.

File impact utama: `workspace/application/scanner/watcher/{suppressor,events,event_filter,lifecycle}.rs`, `reconciliation/application/disk_reconcile/{watcher_batch,onboarding_session,rename_confirmation}.rs`.

Gate: proven internal chain tidak membuat false dirty; external edit dan coverage gap tetap terdeteksi/repair.

### P3 — Readiness dan projection ownership

- [x] Pisahkan core lifecycle, authority, required projection, derived sync pada owner existing; satukan mutation eligibility.
- [x] Move projection worker ke reconciliation API dengan pure application workset/native adapter; gunakan journal pending queue dan foreground yielding existing.
- [x] Implement parked/retry/wake reason; fence toggle checkpoint/publisher dengan epoch.
- [x] Verifikasi otomatis onboarding/activation authority, root switch, DB failure dan restart recovery; native subprocess menguji delapan journal boundaries.
- [ ] Smoke-test native onboarding 1-dari-5 lalu first toggle serta promote game lain pada UI release.

File impact utama: `disk_reconcile/orchestrator/{state,entry,run,request}.rs`, `disk_reconcile/{onboarding_recovery,source_recovery}.rs`, reconciliation API, watcher lifecycle, workspace command adapters, activation/onboarding frontend callers.

Gate: Indexed tidak relock karena ordinary sync; genuine coverage/core fault tetap fail closed dan recoverable. Optional workload tidak menahan rename.

### P4 — Frontend shared record dan scoped refresh

- [x] Consolidate desired/disk/sync state pada `useWorkspaceSwitchActions.ts` dan cohesive shared owner di layer yang sama.
- [x] Ganti per-result polling dengan per-game/epoch tracker pada `workspaceSwitchOps.ts`.
- [x] Migrasikan node/path, preview/context menu, grid/object bulk tanpa duplicate lifecycle.
- [x] Implement revision-aware cache merge, failed-refresh recovery, current-path Explorer, dan coalesced invalidation.

File impact utama: `src/features/workspace-runtime/actions/{useWorkspaceSwitchActions,workspaceSwitchOps,workspaceActionAvailability}.ts`, existing optimistic descriptor builders, `useFolderGridBulk.ts`, object/sidebar action hooks, relevant Explorer resolver.

Gate: semua surface menerima next click; no stuck override, no stale path rollback, satu polling stream, success toast tidak spam.

### P5 — Integrasi seluruh writer dan consumer

- [x] Audit callsite collection apply/capture, parent enable, conflict resolution, randomizer, Safe Mode/preset hotkey, import/move/delete/metadata writers.
- [x] Uji batas admission untuk non-coalescible transactions dan accounting accepted/superseded/rejected; unrelated object batch participants tidak dibuang.
- [x] Writer integration memakai ownership/lease/readiness contract sesuai kelas operasi; collection/Safe Mode capture memakai coherent snapshot barrier khusus, bukan gate global leaf toggle.
- [x] Effective-active digunakan konsisten untuk active collection preview, ancestor indicators, runtime dan KeyViewer. Saved preset membership tidak difilter berdasarkan current state saat membaca preset historis.
- [x] Hapus duplicate organizer path dan workspace projection worker lama; shared frontend owner/index menggantikan lifecycle paralel. Alias/null-proof compatibility dan journal reader 1/2 tetap dipertahankan secara eksplisit, bukan dihapus tanpa migrasi aman.

Gate: tidak ada backend/frontend entrypoint yang bypass fencing; runtime/KeyViewer tidak mengubah disk intent atau menahan switch.

### P6 — Regression, native stress, dan completion evidence

- [x] Jalankan deterministic regression matrix, native watcher/subprocess fixtures, disk benchmark, dan focused correctness/recovery review.
- [ ] Lengkapi native UI, mixed bulk memory/fairness, cold/network storage dan OS failure acceptance di environment target; ini release gate, bukan sisa kode yang diganti dengan abstraksi baru.
- [x] Jalankan lint/architecture/type/build frontend; targeted lalu full frontend/Rust tests sesuai risiko; rustfmt dan Clippy.
- [x] Update status plan/history dengan tes yang benar-benar dijalankan dan limitation native yang tersisa.

Dependency: `P0 -> P1 -> P2/P3 -> P4 -> P5 -> P6`. P2 dan P3 dapat dikerjakan sebagai slice terpisah setelah kontrak P1 stabil; tidak overlap edits. Setiap slice harus compile dan menjaga existing recovery.

## 6. Regression dan stress matrix wajib

Gunakan temporary mod roots, injectable watcher scheduling/DB failures, dan controlled Windows handles. Jangan rename folder mod pengguna untuk stress test.

| Skenario                                                                             | Assertion                                                                                                                                |
| ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| 100/1.000 rapid same-target clicks; explicit desired bercampur toggle, cross-surface | Latest accepted non-cancelled intent menang setelah settle; running+pending bounded, semua outcome accounted, bukan hanya state akhir.   |
| Banyak target dan equal names pada path berbeda                                      | Target tidak saling supersede; union projection scope lengkap.                                                                           |
| Dua folder nyata `A` dan `DISABLED A`, termasuk nested child alias                   | Admission identity berbeda; actual destination collision ditolak tanpa overwrite; tidak salah memilih folder.                            |
| `A -> DISABLED A -> A` dan chain lebih panjang dengan delayed echo                   | Internal identity lineage verified; tidak false reindex/dirty; external event yang disisipkan tetap direconcile.                         |
| Paired/split/reordered events; ambiguous event; overflow/offline/handoff             | Tidak silent suppression/drop; dirty scope atau explicit continuity repair sesuai bukti.                                                 |
| Parent/child intents, ancestor disabled                                              | Local state tetap benar; effective-active false selama ancestor disabled; latest overlapping intent dan confirmation tidak stale.        |
| Bulk same/opposite/partial-overlap + single + collection apply                       | Selection tetap; compensated batch terminal sebelum conflicting successor; best-effort outcomes honest; target non-overlap tidak hilang. |
| Conflict dialog/fix bersamaan indexing/external replace                              | Apply revalidates participant identities; hash overlap tidak blocker; true name collision saja dialog collision.                         |
| Onboarding 1 dari 5 games -> first toggle; open other game belum core-ready          | Dashboard first-game segera setelah required proof; first toggle tidak ditolak oleh stale gate; other game join/promote hanya core-nya.  |
| Projection/SQL tertahan, runtime lambat, query refresh gagal                         | Rename berikutnya tetap prioritas; disk ack tidak rollback; pending sync recoverable; stale result tidak menghapus desired baru.         |
| Root/game change saat intent, event, fetch, projection berjalan                      | Old epoch tidak mutate/publish/ack root baru; lifecycle waiters tidak bocor; pending durable work valid direcover.                       |
| Crash sebelum/antara rename, disk commit, DB checkpoint, journal cleanup             | Pre-commit compensation dan post-commit roll-forward sesuai journal; partial failure tidak dianggap success.                             |
| Windows sharing violation sementara/persisten, ACL, source replacement               | Bounded retry hanya error transient; error nyata actionable; tidak rename folder pengganti.                                              |
| Disabled mod Explorer, collection active preview, KeyViewer                          | Current disk path benar; effective-disabled tidak tampil sebagai active; overlay/keybind fitur berbeda tidak terganggu.                  |

Model/property tests melengkapi, bukan menggantikan, deterministic regressions. Native crash/handle tests dan timing tidak dapat dibuktikan dari React mocks.

## 7. Performance acceptance

Target berikut meneruskan plan storage-first; **bukan hasil pengukuran saat ini**. Catat hardware, storage, fixture depth/sibling count, DB size, cold/warm state, dan dependency delays.

| Metrik                                                     | Gate                                                                                                                                                                                                   |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Click -> optimistic paint                                  | Next frame; sasaran <=16 ms fixture normal. Rekam p95/p99 dan long tasks; tidak ada loading switch karena background refresh.                                                                          |
| Latest intent -> durable disk receipt, warm local SSD leaf | Sasaran p95 <=100 ms, p99 <=300 ms tanpa external lock. Laporkan lease/admission delay terpisah; jangan mengklaim universal sebelum native measured.                                                   |
| Critical storage path                                      | Tidak ada hash/INI harvest, full library walk, global count rebuild, optional runtime, atau broad query wait untuk healthy known leaf toggle. Ancestor/sibling safety I/O tetap boleh.                 |
| Healthy internal toggle                                    | Tidak memicu full reindex hanya karena expected echo. Full scan hanya dengan alasan core/coverage/ambiguity/recovery yang recorded.                                                                    |
| Slow background work                                       | Tahan projector 4 detik dan runtime 10 detik: tidak menambahkan durasi itu ke normal next rename.                                                                                                      |
| Pending/polling                                            | Desired-state work O(unique pending targets + union scopes), bukan O(click count); non-coalescible transaction admission bounded dengan explicit outcome; satu active snapshot request per game/epoch. |
| Bulk                                                       | Ukur 100/1.000/10.000 selected targets sesuai existing limit; time-to-first-commit, throughput, peak memory, fairness dan foreground p95/p99. Compensated batches dilaporkan terpisah.                 |
| Convergence                                                | Setelah input berhenti dan dependency sehat, required/derived checkpoints mencapai revision relevan terakhir, tanpa dropped scope atau stuck record.                                                   |

Jika target latency belum tercapai, laporkan bottleneck trace dan hasil aktual. Jangan menghapus durable write/identity validation untuk membuat angka hijau. Atomic/compensated batch, network disk, cold path, external locks, dan unavailable watcher adalah kategori terpisah.

## 8. Migration, verification, dan done criteria

Migration per slice, tanpa parallel old/new writers. Prefer additive DTO/journal change dan reuse existing state. Schema/dependency change hanya jika implementasi membuktikan existing storage tidak cukup, dengan review dan recovery test sebelum rollout.

Untuk IPC yang berubah: Rust command + Specta registration `src-tauri/src/lib.rs`, exact allowlist `src-tauri/permissions/app-commands.toml` bila command berubah, regenerate bindings via `cargo test specta_tests::export_bindings`, update frontend/mock/demo bersama. Jalankan `cargo test every_registered_command_is_allowed_by_the_app_permission`; jangan memperlebar permissions.

Validation commands saat implementasi, bukan sudah dijalankan oleh plan:

- Frontend: targeted Vitest pada switch actions/ops dan bulk hooks, lalu `pnpm exec vitest run`, `pnpm exec tsc --noEmit`, `pnpm lint`, `pnpm lint:arch`, `pnpm build`.
- Backend dari `src-tauri`: targeted coordinator/journal/recovery/watcher/reconcile tests, lalu `cargo test --lib`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- IPC/export gate, `git diff --check`, native fixture matrix dan measured performance report.

Implementation acceptance di bawah sudah dibuktikan pada regression/fixture scope yang dicatat. Ini tidak menggantikan manual release gates pada P0/P2/P3/P6 atau menjanjikan semua kondisi OS bebas bug:

- [x] Final physical folder state cocok latest accepted intent yang berhasil, tanpa mutation pada identity lain.
- [x] Disk receipt durable dan recovery menjaga acknowledged commit.
- [x] Core readiness stabil; dirty scope, coverage loss, pending projection, dan optional runtime dibedakan.
- [x] Internal echo lineage aman dan genuine external rename terdeteksi setelah beberapa rename berulang.
- [x] Switch tidak relock karena ordinary sync pada frontend regression; failed refresh punya recovery yang jelas. Native paint/long-task acceptance tetap terpisah.
- [x] Toggle conflict hanya actual destination identity collision; hash/runtime overlap tidak memblokir. Rename/import/organizer logical naming policy tetap kompatibel.
- [x] Consumer effective-state/revision/epoch dan scoped publication diaudit; ownership/descendant/repair regressions lulus.
- [x] Queue, polling, retry dan evidence retention bounded; permanent failure terlihat, tidak silent/stuck.
- [x] Gates dijalankan dan evidence dicatat; yang belum diuji dinyatakan eksplisit, bukan ditandai hijau.

Rollback kode tidak boleh menghapus journal format baru atau membalik acknowledged disk commits. Bila rollout perlu dihentikan, hentikan admission operasi baru secara terkontrol sambil menjaga recovery/projection untuk pending commits; compatibility reader harus tetap tersedia.

## 9. Native acceptance, consumer consistency and measured performance closure

Tanggal: 2026-10-05; resumed 2026-10-06. Status: **approved; final native bulk/performance and full validation gates in progress**. Tahap ini menutup batas bukti yang masih tersisa, bukan rewrite coordinator, watcher atau runtime. Hasil automated gates sebelumnya tetap historical evidence, tidak dihitung sebagai hasil native baru. Evidence aktual dan batas yang tersisa ada di [native-acceptance-report.md](native-acceptance-report.md).

### 9.1 Goals dan guardrails

1. Buktikan onboarding game pertama -> dashboard -> toggle pertama dapat mengubah disk tanpa menunggu optional indexing/runtime.
2. Buktikan latest-desired intent menang untuk operasi toggle yang sah; failure/cancellation/supersession tetap accounted dan collision nyata tidak overwrite.
3. Buktikan normal healthy leaf tetap responsif ketika projector/runtime tertunda dan best-effort bulk berjalan; ukur starvation dan memori, bukan hanya throughput.
4. Buktikan UI, DB, collection, ancestor dan KeyViewer berkonvergensi pada identity/epoch/revision yang sama setelah dependency pulih.
5. Perbaiki hanya gap yang direproduksi atau bottleneck yang terukur. Pertahankan satu mutation coordinator, satu projection worker/game dan shared frontend tracker/scheduler.

Tidak termasuk: scheduler/event bus baru, per-folder parallel locks, cache kebenaran baru, schema/journal upgrade, perubahan semantics compensated transactions, updater/release/push/installer. Tidak mengurangi identity proof, no-overwrite atau durable acknowledgement untuk mengejar angka. Hash/runtime-key overlap tetap advisory; source replacement, ACL, coverage loss dan destination collision tetap safety failures yang sah.

### 9.2 Baseline dan seam yang sudah tersedia

- `mod_bulk_cmds.rs` memakai `BULK_TOGGLE_CHUNK_SIZE = 32`, lease per chunk dan `yield_now()` antar-chunk. Foreground registration berlaku sepanjang bulk. Ini titik pengukuran fairness; adanya yield belum merupakan bukti latency atau convergence selama bulk.
- `mutation/native_tests.rs` sudah mempunyai native disk-ack benchmark dan subprocess recovery. `scanner/tests/native_watcher_stress.rs` sudah memakai watcher/debouncer nyata. Extend fixture tersebut, jangan buat mutation engine versi tes.
- `toggle_projection.rs`, `runtime_sync.rs` dan `post_apply.rs` sudah mencatat scope, revision dan worker wait/elapsed. Timestamp click/paint dan rincian storage path end-to-end belum lengkap.
- `wdio.conf.ts`, `tauri.e2e.conf.json` dan `tests/e2e/support/*` menyediakan native WebView2 harness dengan identifier E2E dan temporary mock games. Reuse harness ini untuk native UI; demo/browser mocks tidak membuktikan native switching.
- `phase3-mod-ops.e2e.ts` masih mempunyai assertion DB langsung setelah disk-first command. Sesuaikan readiness assertion menggunakan existing projection snapshot/checkpoint, bukan sleep tetap atau explicit full reconcile setelah toggle.
- `phase8-runtime.e2e.ts` menguji command surface, bukan render overlay di game nyata. Generated KeyViewer artifact dan actual in-game overlay harus dilaporkan sebagai kategori bukti berbeda.

### 9.3 Tahap N0 — Safety preflight dan reusable fixtures

- [x] Verifikasi binary dibangun dengan identifier `com.reynalivan.emmm.e2e`, app-data terpisah, semua Mods roots dari fixture yang dimiliki run ini, dan setting/hotkey tidak mengambil kontrol game pengguna.
- [x] Batasi cleanup harness ke PID proses/driver yang diluncurkan run ini. Existing `taskkill /IM` untuk driver global tidak boleh mematikan driver milik sesi lain.
- [x] Reuse `createMockGame`/`addMockMod`; gunakan unique temporary roots dan validasi resolved ownership sebelum recursive cleanup. Stop proses/watcher milik fixture sebelum menghapus payloadnya. Jangan menerima arbitrary path sebagai target cleanup.
- [x] Audit selector dan support readiness native yang sudah ada. Tambah helper hanya untuk kebutuhan kedua yang terbukti, tanpa test bridge yang membuat state aplikasi paralel.
- [ ] Siapkan corpus 100/1.000/10.000 mods, flat/nested, ancestor disabled, same basename di parent berbeda, actual `A` + `DISABLED A`, serta hash/key overlap. Fixture wajib classifier-valid dan memiliki payload sentinel/native identity assertions.
- [ ] Catat commit/build mode, OS/WebView2, CPU/RAM, storage/volume, DB size, refresh rate, fixture depth/sibling count dan cold/warm state. Verifikasi driver/binary tersedia sebelum expensive run; dependency download/install yang diperlukan harus eksplisit, bukan fallback diam-diam.

File impact: `wdio.conf.ts`, `tests/e2e/support/{fixtures,app,ipc}.ts` bila diperlukan, existing native fixture helpers. Tidak ada perubahan production flow pada N0.

Gate: fixture/DB/proses terisolasi dan cleanup target terverifikasi. Jika native driver atau game nyata tidak tersedia, tandai gate terkait unavailable; jangan mengganti labelnya menjadi native-pass dari demo atau mocks.

### 9.4 Tahap N1 — Timeline dan baseline yang dapat dikorelasikan

- [ ] Rekam span: input diterima -> optimistic DOM commit/next frame -> IPC dispatch -> admission -> lease acquired -> safety/preflight -> rename -> native validation -> durable DiskCommitted -> receipt diterima -> DB checkpoint -> query publication -> runtime artifact publication.
- [x] Reuse operation ID, game ID, intent/disk revision, root epoch dan runtime generation. Clock Rust memakai monotonic duration; clock WebView memakai `performance.now()`. Jangan mengurangi timestamp dari clock domain berbeda; laporkan IPC round-trip terpisah dari span native.
- [ ] Tambahkan hanya span yang belum tersedia di existing command/storage/action seams. Diagnostics opt-in/debug dan sample retention dibatasi; clear frontend marks setelah export. Tidak menulis log per-file atau mengumpulkan path/nama mod pribadi tanpa kebutuhan.
- [ ] Ukur idle baseline dan workload campuran, minimal 100 timed samples/cell setelah warmup, tiga repeat. Laporkan p50/p95/p99, maksimum, sample count, failed/superseded outcomes dan outlier; jangan hanya memilih repeat terbaik.
- [x] Bedakan DOM/rAF next-frame proxy dari actual rendered paint. WebDriver round-trip bukan click-to-paint; bila actual frame trace/capture tidak tersedia, klaim paint tetap pending.

File impact bila span belum ada: `workspace/adapters/tauri/workspace_cmds.rs`, `library/adapters/tauri/mod_bulk_cmds.rs`, `library/application/mods/core_ops/toggle.rs`, `mutation/coordinator.rs`, `reconciliation/adapters/tauri/{toggle_projection,runtime_sync}.rs`, frontend `useWorkspaceSwitchActions.ts`; gunakan existing diagnostics gate, tanpa IPC command publik baru.

Gate: satu sample dapat diikuti dari input hingga disk receipt dan derived publication; hasil disk benchmark tidak disamakan dengan UI/native end-to-end. Logging overhead dibandingkan diagnostics off/on dan dilaporkan.

### 9.5 Tahap N2 — Native lifecycle dan drift acceptance

Tambahkan cohesive spec `tests/e2e/specs/switching-stability.e2e.ts` menggunakan native harness/support existing. Seed melalui IPC diperbolehkan; scenario action yang diklaim UI wajib melalui kontrol aplikasi. Hindari duplicate suite luas.

| Scenario                                                                             | Assertion utama                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Onboarding lima fixture games, first-game core selesai, langsung toggle              | Dashboard tidak menunggu core games 2–5; first toggle mengubah folder fisik. Optional runtime/query refresh tidak mengubah readiness menjadi unindexed.                                                                                            |
| Buka game belum siap, lalu kembali ke game ready                                     | Game baru loading hanya sampai required core/coverage siap; existing indexing di-join/promote, tidak membuat owner kedua; game ready tidak relock karena sync biasa.                                                                               |
| 100 lalu 1.000 intents pada target sama, lintas grid/preview/context menu            | Latest accepted desired state yang berhasil cocok dengan disk identity; semua outcomes accounted; tidak self-conflict/loading-lock atau pertumbuhan per klik. Synthetic in-WebView burst dilabeli berbeda dari trusted native pointer input.       |
| Banyak target, parent/child, same basename beda parent                               | Tidak cross-target supersession; parent confirmation revalidates; effective-active false ketika ancestor disabled.                                                                                                                                 |
| Rename eksternal berulang pelan/cepat, disisipkan pada toggle dan inactive takeover  | Watcher tetap mengikuti native identity atau menandai dirty/repair eksplisit; perubahan berikutnya masih terdeteksi setelah catch-up. Jangan menuntut OS mengirim setiap intermediate edge.                                                        |
| A -> B -> A sebelum refresh A selesai, termasuk successor click dan root replacement | Fresh scope refresh sebelum overlay lama dibersihkan; old completion tidak menghapus successor; epoch lama tidak publish ke root baru.                                                                                                             |
| Destination collision raced dan replacement source                                   | Tidak overwrite/rebind folder lain; payload sentinel kedua folder utuh; repair/conflict hanya mengisolasi scope yang perlu. Hash/key overlap tidak menghalangi switch.                                                                             |
| Disabled mod -> Open in Explorer; toast burst                                        | Resolver memakai current disk path termasuk ancestor rename; request opener benar. Visible Explorer window merupakan manual check jika driver tidak dapat memverifikasinya. Toast success hanya sesudah disk proof, burst tidak menghasilkan spam. |

- [ ] Eksekusi matrix di atas dengan bounded predicate waits berbasis readiness/checkpoint/epoch, bukan sleeps yang dianggap bukti.
- [x] Jangan menjalankan reconcile manual setelah toggle/rename untuk membuat consumer assertion hijau: itu menutupi watcher/projector yang macet.
- [ ] Simpan failure bundle: scenario seed, last accepted intent, receipt, disk identities, watcher coverage/dirty reason, journal pending/repair IDs dan projection/runtime revisions. Hindari menyimpan user library paths.

Gate: zero wrong-identity mutation, unexpected relock, stuck overlay, lost target atau unaccounted outcome pada corpus; genuine safety failures tetap diharapkan dan tidak diperlakukan sebagai bug responsiveness.

### 9.6 Tahap N3 — Mixed bulk fairness, stalled dependency dan memory

- [ ] Jalankan best-effort bulk 100/1.000/10.000 targets pada actual command path dengan frozen selection. Sisipkan single toggle unrelated/overlapping dan opposite bulk; verifikasi latest acceptance order per participant dan hasil non-overlap tidak hilang.
- [ ] Ukur time-to-first-commit, throughput, foreground lease wait/disk-ack p95/p99, longest starvation, watcher backlog, projection/runtime catch-up dan process/WebView peak memory.
- [ ] Jalankan mixed leaf/subtree runtime requests dan rekam alasan Full composition. Biarkan fallback konservatif tetap ada sampai trace membuktikan biaya materialnya.
- [ ] Tahan projector 4 detik dan runtime 10 detik pada existing consumed application/worker test seams; lalu release dan buktikan rename tidak menunggu optional duration itu serta union scopes converge. Native UI run memakai workload nyata; jika delay injection hanya tersedia di Rust tests, jangan klaim UI delay-injection sudah tested. Tidak menambah production delay setting/command.
- [ ] Pisahkan compensated object/collection transaction yang sudah mulai dari best-effort bulk. Jangan memaksa yield/preemption di tengah rename/rollback transaction untuk memenuhi latency leaf.
- [ ] Bandingkan 100/1.000 intents pada jumlah physical targets yang sama dan repeated bulk cycles. Setelah settle, ephemeral records/waiters/timers kembali ke baseline; journal repair evidence yang sengaja durable tidak dihitung sebagai leak. Warm caches harus plateau sesuai bound existing.

Gate: tidak ada starvation sampai bulk habis untuk unrelated healthy leaf pada best-effort path; latency dilaporkan menurut chunk size/lease yang nyata, bukan microbenchmark admission. Memory mengikuti unique targets/union scopes/cache bounds, bukan jumlah klik atau repeated cycles tanpa batas.

### 9.7 Tahap N4 — Consumer consistency dan minimal fixes

Ambil satu quiescent snapshot `(game, rootEpoch, relevantDiskRevision)` setelah input berhenti dan dependency sehat. Coherent collection capture boleh menunggu required projection barrier; normal leaf tidak. Jangan membuat global settlement barrier baru untuk semua switch.

- [ ] Validasi disk local-enabled terhadap actual prefix/native identity; validasi effective-active terhadap semua ancestor sampai Mods root.
- [ ] Bandingkan DB paths/counts, sidebar ancestor indicator, current collection preview/capture, runtime contributions dan KeyViewer generated panels terhadap snapshot yang sama.
- [ ] Saved preset membership tetap historical; jangan memfilter saved preset dengan kondisi disk saat ini. Current-state preview/capture hanya effective-active.
- [ ] Enabled observer-compatible mod tanpa switcher keys tetap mempunyai empty KeyViewer panel; mod dengan disabled ancestor tidak aktif. Preset-status overlay tetap terpisah dari KeyViewer.
- [ ] Uji late query/event/runtime result dan repaired snapshot; hasil lama tidak menimpa identity/epoch/revision lebih baru. Actual in-game overlay/reload tetap manual gate pada game target yang diizinkan, bukan dibuktikan oleh fake executable.
- [ ] Untuk failure nyata: reproduksi regression RED di seam paling kecil, fix owner yang salah, jalankan GREEN lalu matrix terkait. Consolidate duplicate predicate/transition hanya setelah dua consumed usages terbukti sama; jangan memperkenalkan generic state machine atau cache tambahan.
- [ ] Jika trace membuktikan chunk fairness gagal, evaluasi chunk size/yield/lease lifetime pada owner existing. Jangan mengubah compensated semantics. Jika mixed runtime Full mahal, usulkan optimization terpisah dengan correctness evidence sebelum mengimplementasikannya.

File impact conditional: existing consumer tests di `disk_reconcile/reconcile_tests.rs`, `tests/e2e/specs/{phase3-mod-ops,phase7-collections,phase8-runtime}.e2e.ts`, frontend switch/bulk tests; production owners hanya jika failure menunjukkan mereka perlu diubah. Tidak semua file ini otomatis diubah.

Gate: disk/DB/current collection/ancestor/runtime/KeyViewer setuju setelah settle; ownership repair tidak dianggap success dan tidak membalik disk acknowledgement. Seluruh production edit mempunyai alasan, minimal diff dan regression evidence.

### 9.8 Acceptance metrics dan hasil yang harus dicatat

Target berikut belum merupakan hasil tahap ini; meneruskan Section 7 untuk warm local SSD, healthy known leaf, tanpa OS sharing lock.

| Metrik                               | Acceptance / cara pelaporan                                                                                                                                                                                                       |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Optimistic feedback                  | Next available frame; target <=16 ms pada fixture 60 Hz normal. Catat actual refresh rate, long tasks dan proxy vs actual paint. Tidak ada switch loading/disabled akibat ordinary sync.                                          |
| Input/intent -> durable disk receipt | Target p95 <=100 ms, p99 <=300 ms; laporkan frontend/IPC dan native spans terpisah. Cold/network/compensated waits dilaporkan terpisah, bukan dikeluarkan diam-diam.                                                              |
| Optional dependency isolation        | Holding projection/runtime tidak menambah durasi 4/10 detik ke healthy next leaf rename. Scoped safety repair yang benar tetap dicatat sebagai kategori lain.                                                                     |
| Bulk fairness                        | Foreground unrelated leaf mendapat kesempatan antar best-effort chunks, bukan setelah seluruh 10k selesai. Jika target latency gagal, laporkan actual bound/bottleneck dan test-first fix; ukuran chunk 32 bukan jaminan latency. |
| Convergence                          | Dalam healthy fixture, budget assertion awal 30 detik setelah terakhir input/dependency release; timeout adalah test failure dengan diagnostics, bukan alasan clear state/ack otomatis. Bukan universal runtime/storage SLA.      |
| Retention                            | Satu owner/poll stream existing per game/epoch; setelah settle tidak ada pending switch retry timer/record yang yatim. Peak dan settled memory dilaporkan untuk seluruh repeat.                                                   |
| Safety                               | Zero overwrite/wrong identity/lost scope/false checkpoint. Genuine collision, unavailable source atau replacement mempunyai outcome/repair eksplisit.                                                                             |

Output evidence: `docs/plans/stable-mod-switching/native-acceptance-report.md` saat eksekusi, berisi hardware/build/corpus, sample counts/raw distributions, scenario results, trace bottlenecks, actual changed files dan deferred gates. Tidak membuat report kosong atau menyalin hasil lama sebagai hasil baru.

### 9.9 Urutan, verification dan completion checklist

Urutan: `N0 safety -> N1 baseline -> N2 lifecycle -> N3 contention/memory -> N4 consistency/fixes -> final rerun`. Fix correctness yang terungkap pada tahap awal diselesaikan sebelum benchmark lanjutan; benchmark before/after memakai corpus/build configuration sama.

- [ ] Native lifecycle, bulk/fairness dan consumer gates memiliki actual run evidence, bukan checklist dari unit tests.
- [x] Production changes, bila ada, menjaga single writer/proof/checkpoint ownership dan API facade boundaries; update `src-tauri/AGENT.md` hanya jika flow berubah.
- [x] Targeted lalu full Rust/Vitest, TypeScript, E2E typecheck, ESLint/architecture lint, Vite build, rustfmt, Clippy `-D warnings`, formatting dan diff-check sesuai perubahan lulus. Cargo dijalankan dari `src-tauri` agar SQLx offline config berlaku.
- [ ] Reuse `pnpm test:e2e --spec ...`/WDIO setelah harness safety gate; native benchmark/recovery/watcher filters dieksekusi terpisah dan manual ignored benchmarks dilabeli eksplisit. Tidak menjalankan seluruh network/download/delete specs tanpa kebutuhan.
- [x] Focused review untuk material production/test-harness changes. Plan/history mencatat hasil yang benar-benar dijalankan, failed runs/outlier dan limit yang belum selesai.
- [ ] Native overlay/Explorer window dan cold/network/ACL/persistent-sharing/power-loss matrix yang belum tersedia tetap unchecked dengan prerequisite jelas; gate tersebut tidak dinyatakan passed karena unit/fixture berhasil.

Completion berarti acceptance scope yang tersedia sudah dibuktikan dan measured failure diperbaiki, bukan klaim aplikasi bebas semua bug di semua OS/storage. Push/build installer tetap langkah terpisah yang memerlukan request eksplisit.

### 9.10 Approved namespace-proof closure (2026-10-06)

User approved finishing the Windows `Modify(Any)` bottleneck. Ordinary directory dirt remains observable; it is not classified as a harmless echo.

- [x] Request-local complete parent-chain native/canonical proof captured before waits; existing physical intent admission continues to own source identity.
- [x] Direct same-parent prefix toggles and non-overlapping best-effort bulk may proceed through ordinary dirt, without changing strict object/ancestor/collection snapshot barriers. Echo registration uses the original validated storage proof/session instead of revoking physical coverage merely because projection is dirty.
- [x] Captured watcher session, completed indexed root, authority gaps and suppressor gaps are revalidated; replacement ancestry rejects rather than rebinding through repair.
- [x] Prepared plans share the proof and validate it before every physical rename attempt, including retries; no-overwrite and durable receipt remain mandatory.
- [x] Ordinary watcher batches reuse existing nonblocking catch-up and timed retry, so they do not queue scans between foreground bulk chunks. Deferred dirty evidence is retained.
- [x] Folded debouncer rename events complete the maximal contiguous same-ID chain after native/session/lineage proof; two-, three- and four-edge reverse chains are covered. Pending chains use the existing per-edge callbacks until full commit proof; classifier delivery remains conservative. Abort/expiry release repair evidence; replacements, foreign ownership and discontinuity reject.
- [x] Bulk foreground priority is registered before expensive identity resolution/admission. Projection completion validates the entire exact operation batch, then publishes one existing atomic journal snapshot before progress cleanup; invalid/repair IDs preserve all pending work. No storage acknowledgement durability is removed.
- [x] Final full local gates and actual native bulk/fairness/convergence rerun passed: 9/9 native scenarios, including 10k and post-bulk opposite intents. Failed/partial runs, missed latency targets and unavailable manual gates remain explicit in the acceptance report; Section 9 is not blanket release certification.
