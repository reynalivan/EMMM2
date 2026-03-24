import sqlite3
import sys

with open('db_dump.txt', 'w', encoding='utf-8') as f:
    conn = sqlite3.connect(r'C:\Users\yusri\AppData\Roaming\com.emmm.app\app.db')
    c = conn.cursor()

    f.write("=== ENABLED MODS WITH OBJECTS ===\n")
    try:
        c.execute("""
            SELECT o.name, o.folder_path, m.actual_name, m.folder_path, m.status, o.id, m.object_id
            FROM objects o
            JOIN mods m ON m.object_id = o.id
            WHERE m.status = 'ENABLED'
            ORDER BY o.name, m.actual_name
        """)
        for r in c.fetchall():
            f.write(f"OBJ: {r[0]:15s} | MOD: {r[2]:40s} | M_PATH: {r[3]}\n")
    except Exception as e:
        f.write(f"  Error: {e}\n")

    f.write("\n=== ENABLED MODS WITHOUT OBJECT (NULL object_id) ===\n")
    try:
        c.execute("""
            SELECT m.actual_name, m.folder_path, m.object_id
            FROM mods m
            WHERE m.status = 'ENABLED' AND m.object_id IS NULL
        """)
        for r in c.fetchall():
            f.write(f"  MOD: {r[0]:40s} | PATH: {r[1]}\n")
    except Exception as e:
        f.write(f"  Error: {e}\n")

    conn.close()
