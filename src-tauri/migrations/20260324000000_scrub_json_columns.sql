-- Update corrupted JSON payloads in objects
UPDATE objects SET hash_db = NULL WHERE hash_db IS NOT NULL AND json_valid(hash_db) = 0;
UPDATE objects SET custom_skins = NULL WHERE custom_skins IS NOT NULL AND json_valid(custom_skins) = 0;
