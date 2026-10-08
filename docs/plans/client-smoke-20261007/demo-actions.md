# Browser-demo action evidence

55 recorded action/scenario results: 39 PASS, 13 BLOCKED, 3 FAIL. These are **not** unique controls and **not** native integration results. Demo state is in-memory. Observations span changing development snapshots; subsequent concurrent edits are not automatically certified. Inventories were sampled on 19 views/dialogs, including ten Settings tabs; underlying controls in declarative dialogs can remain in the HTML inventory. This is not an exhaustive actionable-control count.

| Action/scenario                  | Result  | Observed proof or boundary                                 |
| -------------------------------- | ------- | ---------------------------------------------------------- |
| Dashboard key search, valid      | PASS    | One matching Nekomata row                                  |
| Dashboard literal special search | PASS    | Percent, underscore and backslash produce no wildcard rows |
| Clear filters                    | PASS    | Three mappings restored                                    |
| Character classification         | PASS    | Matching member and row count                              |
| Environment classification       | PASS    | Matching member and row count                              |
| UI classification                | PASS    | Matching member and row count                              |
| Unclassified classification      | PASS    | Empty state                                                |
| All classifications              | PASS    | Three data rows restored                                   |
| Dashboard Play                   | BLOCKED | Real process execution intentionally unavailable           |
| Dashboard Quick Play             | BLOCKED | No native launch certified                                 |
| App Menu Settings                | PASS    | Settings rendered                                          |
| System theme                     | PASS    | Real control selects fixture value                         |
| Light theme                      | PASS    | Real control selects fixture value                         |
| Onyx theme                       | PASS    | Real control selects fixture value                         |
| Close after launch               | BLOCKED | Missing demo handler; rolls back                           |
| Privacy close button             | PASS    | Three-section content; button closes                       |
| Privacy Escape                   | FAIL    | Dialog remains after Escape                                |
| Terms close button               | PASS    | Read-only content closes; no agreement accepted            |
| Download theme template          | BLOCKED | Missing handler; visible failure                           |
| Import theme                     | BLOCKED | Native picker/import unavailable                           |
| Check updates                    | BLOCKED | Native updater not certified                               |
| Support link                     | BLOCKED | Native external dispatcher absent                          |
| Homepage save/refetch            | FAIL    | Reads game ID `demo-zenless`, not URL                      |
| Homepage reset/refetch           | FAIL    | Reads game ID again                                        |
| Retention 0                      | PASS    | Validation and saved 30 restored                           |
| Retention 1                      | PASS    | Fixture accepts lower bound                                |
| Retention 365                    | PASS    | Fixture accepts upper bound                                |
| Clear old downloads              | PASS    | Fixture count updates; no disk deletion proof              |
| Blank Safety keyword             | PASS    | No keyword created                                         |
| Unicode Safety keyword           | PASS    | Trim/lowercase rendered                                    |
| Normalized duplicate keyword     | PASS    | One row retained, warning                                  |
| Remove last keyword              | PASS    | Empty state restored                                       |
| Overlay F5 draft conflict        | PASS    | Save blocked                                               |
| Overlay F6 draft conflict        | PASS    | Save blocked                                               |
| Overlay F8 draft conflict        | PASS    | Save blocked                                               |
| Hotkey row reset                 | PASS    | Overlay returns to F7                                      |
| Global/Beta draft independence   | PASS    | Global-off disables bindings, not Beta control             |
| Hotkey reset-all draft           | PASS    | Defaults restored; no OS registration                      |
| Hotkey save                      | BLOCKED | Demo handler absent                                        |
| AI show/hide                     | PASS    | Invalid QA draft masked/revealed                           |
| AI save                          | PASS    | Fixture presence updated; no vault proof                   |
| AI test connection               | BLOCKED | Simulated handler is not actual service proof              |
| AI remove key                    | PASS    | QA fixture presence cleared                                |
| Blank game form                  | PASS    | Submit disabled                                            |
| Maintenance/cache actions        | BLOCKED | Missing handlers; native checked separately                |
| Reset Cancel                     | PASS    | Dialog closes; Settings retained                           |
| Integration picker/release       | BLOCKED | Native/external dependency excluded                        |
| Logs level filter                | PASS    | Error/All options selectable; empty state                  |
| Inbox clear selection            | PASS    | Review disabled at zero selection                          |
| Review confidence filters        | PASS    | High row and Errors empty state                            |
| Review Skip                      | BLOCKED | `setImportItemDecision` missing                            |
| Review Close/Resume              | PASS    | Same pending batch reopens                                 |
| Review Cancel                    | BLOCKED | `cancelImportBatch` missing                                |
| Processed empty state            | PASS    | No processed imports; select-all disabled                  |
| Optimizer Keep Cancel            | PASS    | Modal closes, four fixture groups retained                 |

Further interactive inventory was interrupted by repeated browser transport timeouts during concurrent workspace/native build activity. Those attempts are not added as passing actions. No claim of mobile/responsive, full accessibility, Core Web Vitals, external integration or all-buttons completeness is made.
