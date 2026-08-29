content = """# Fase F (Backend Core Logic Relocation)

## Context
Menjalankan Phase F dari blueprint relocation.md untuk mengubah arsitektur Backend dari global horizontal layers (services, repo, domain, common) menjadi domain-oriented modular monolith (Vertical Slices).

## Changes
- Memindahkan seluruh direktori `services/`, `repo/`, `domain/`, dan `common/` ke dalam struktur `modules/*/application`, `modules/*/adapters/outbound`, dan `modules/*/domain`.
- Modul dipecah menjadi: `system`, `workspace`, `library`, `catalog`, `games`, `automation`, `browser`, `ingestion`.
- Komponen utilitas dipindahkan ke `platform/fs`, `platform/images`, dan `shared/`.
- Memperbaiki ribuan baris import paths secara massal melalui Python scripts.
- Menghapus folder `src-tauri/src/services`, `repo`, `domain`, `common` seluruhnya.

## Impacted Files
- `src-tauri/src/*` (Semua file backend dipindahkan secara terstruktur).
- `src-tauri/tests/*` (Diperbarui untuk merefleksikan import vertical).
- `src-tauri/src/lib.rs` (Root module exports).

## Goal
Backend sekarang terorganisasi secara rapi ke dalam Domain-Oriented Modular Monolith (Vertical Slices). Modul internal lebih tertutup dan terstruktur secara kohesif per fitur (Application, Domain, Adapters).

## Impact
- Perubahan arsitektur ini tidak mengubah behavior runtime manapun, melainkan murni refactor struktural.
- Modul backend sudah 100% mengikuti blueprint arsitektur baru.

## Notes
- Terdapat test asinkronus (`clearing_memory_cache_drops_resolved_folder_entries`) di `thumbnail_cache_tests` yang bersifat flaky karena balapan singleton state global, tetapi tidak terkait langsung dengan logika migrasi ini.
"""

with open("E:/code/projects/EMMM2/.docs/history/202608290003-phase-f-restructure.md", "w") as f:
    f.write(content)
