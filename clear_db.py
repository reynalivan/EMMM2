import sqlite3

db_path = r'C:\Users\yusri\AppData\Roaming\com.emmm.app\app.db'
conn = sqlite3.connect(db_path)
c = conn.cursor()

tables_to_clear = [
    "mods",
    "objects",
    "collection_member",
    "collections"
]

for table in tables_to_clear:
    try:
        c.execute(f"DELETE FROM {table}")
        print(f"Cleared {table}")
    except Exception as e:
        print(f"Error clearing {table}: {e}")

conn.commit()
conn.close()
print("Database tables cleared successfully. Ready for rescan.")
