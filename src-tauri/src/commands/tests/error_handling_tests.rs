#[cfg(test)]
mod tests {
    use crate::types::errors::CommandError;
    use serde_json;

    #[test]
    fn test_command_error_serialization() {
        // As per TC-36, ensure backend errors serialize correctly into JSON strings instead of panicking
        let err = CommandError::NotFound("Mod folder 'Amber' not found".into());

        let json = serde_json::to_string(&err).expect("Failed to serialize CommandError");

        // It should serialize to a string exactly matching its Display representation
        assert_eq!(json, "\"Not found: Mod folder 'Amber' not found\"");

        let db_err = CommandError::Database("duplicate target".into());
        let db_json = serde_json::to_string(&db_err).unwrap();
        assert_eq!(db_json, "\"Database error: duplicate target\"");
    }
}
