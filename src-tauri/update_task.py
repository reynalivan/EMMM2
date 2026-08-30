import os

path = r"C:\Users\yusri\.gemini\antigravity\brain\8b2acecf-cb61-42d1-8d4b-87bd8485baea\task.md"
if os.path.exists(path):
    with open(path, "r") as f:
        c = f.read()
    c = c.replace("[ ] Buat file `api.rs` di setiap modul", "[x] Buat file `api.rs` di setiap modul")
    c = c.replace("[ ] Ekspor `application` dan fungsi dari facade lama", "[x] Ekspor `application` dan fungsi dari facade lama")
    c = c.replace("[ ] Ubah **visibilitas** internal di `mod.rs`", "[x] Ubah **visibilitas** internal di `mod.rs`")
    c = c.replace("[ ] Hapus file `facade.rs` lama", "[x] Hapus file `facade.rs` lama")
    c = c.replace("[ ] Refactor *integration tests*", "[x] Refactor *integration tests*")
    with open(path, "w") as f:
        f.write(c)
