# Preview INI batch and scroll coalescing

## Changes

- Replace one `read_mod_ini` command per file with one `read_mod_ini_documents` command.
- Parse the batch in the blocking pool with Rayon, preserving bounded parallel disk reads.
- Coalesce Object List sticky-row viewport updates to one `requestAnimationFrame` per frame.

## Benchmark

Warm-cache debug fixture, median of five runs:

| INI files | Serial parse | Batched parse |
| --- | ---: | ---: |
| 100 | 13.5639 ms | 5.4833 ms |
| 500 | 67.8635 ms | 21.3289 ms |

Discovery was 1.003 ms and 1.7719 ms respectively. The batch also removes one renderer-to-Rust command and React Query update per INI file.

## Validation

- Rust preview operation tests: 7 passed, 1 manual benchmark ignored.
- Frontend preview and Object List hook tests: 38 passed.
- Production frontend build passed.
