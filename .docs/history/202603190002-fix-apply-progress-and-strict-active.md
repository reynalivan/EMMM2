# Fix Apply Progress And Strict Active

- fixed collection apply strict parity for object-state collections by applying object folder enable/disable before mod diff resolution
- added lightweight in-memory apply progress with `get_apply_progress` for phase, counts, and current item reporting
- updated apply modal to render preview shell earlier, stop blocking on secondary object-state queries, and keep the modal open for progress and success states
- made collection row thumbnails lazy-load on visibility so apply preview first paint is not gated by thumbnail fetches
- added backend regression for disabled object-state apply staying named on follow-up overview reads
