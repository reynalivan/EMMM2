import sqlite3, sys
sys.stdout.reconfigure(encoding='utf-8')

db = r'C:\Users\yusri\AppData\Roaming\com.emmm.app\app.db'
conn = sqlite3.connect(db)
c = conn.cursor()

# List all tables
c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
print("TABLES:", [r[0] for r in c.fetchall()])

# Check collections table
try:
    c.execute("SELECT id, name, is_safe_context FROM collections LIMIT 5")
    print("\n== COLLECTIONS ==")
    for row in c.fetchall():
        print(f"  id={row[0][:8]}.. name={row[1]} safe={row[2]}")
except:
    pass

# Check collection_member table
try:
    c.execute("SELECT collection_id, path_key, kind, is_enabled FROM collection_member LIMIT 15")
    print("\n== COLLECTION MEMBERS (first 15) ==")
    for cid, pk, kind, en in c.fetchall():
        print(f"  coll={cid[:8]}.. kind={kind:6s} en={en} pk={pk}")
except Exception as e:
    print(f"Error: {e}")

# Check mods folder_path vs folder_path_key
print("\n== MODS: fp vs fpk (first 10 enabled) ==")
c.execute("""
    SELECT actual_name, folder_path, folder_path_key, status
    FROM mods WHERE status = 'ENABLED' ORDER BY actual_name LIMIT 10
""")
for row in c.fetchall():
    print(f"  {row[0]}: fp={row[1]} fpk={row[2]} st={row[3]}")

print("\n== MODS: fp vs fpk (first 10 disabled) ==")
c.execute("""
    SELECT actual_name, folder_path, folder_path_key, status
    FROM mods WHERE status = 'DISABLED' ORDER BY actual_name LIMIT 10
""")
for row in c.fetchall():
    print(f"  {row[0]}: fp={row[1]} fpk={row[2]} st={row[3]}")

# Check corridor table
print("\n== CORRIDOR ==")
try:
    c.execute("SELECT game_id, is_safe, active_collection_id, undo_collection_id, current_signature FROM corridor")
    for row in c.fetchall():
        print(f"  game={row[0][:8]}.. safe={row[1]} active={str(row[2])[:8] if row[2] else None}.. undo={str(row[3])[:8] if row[3] else None}")
except Exception as e:
    print(f"Error: {e}")

conn.close()
