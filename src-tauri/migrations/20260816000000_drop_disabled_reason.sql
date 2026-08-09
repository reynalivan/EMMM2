-- disabled_reason only ever held one value once the object->children cascade
-- was removed (the user is the only disabler), so it carried no information.
-- Status itself derives from the DISABLED folder-name prefix via disk
-- reconcile, the single writer of mods.status.
ALTER TABLE mods DROP COLUMN disabled_reason;
