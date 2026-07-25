#[cfg(test)]
mod tests {
    use crate::domain::errors::AppError;
    use serde_json;

    #[test]
    fn test_app_error_serialization() {
        // As per TC-36, ensure backend errors serialize correctly into JSON instead of panicking
        let err = AppError::NotFound("Mod folder 'Amber' not found".into());

        let json = serde_json::to_string(&err).expect("Failed to serialize AppError");

        assert_eq!(
            json,
            "{\"type\":\"NotFound\",\"payload\":\"Mod folder 'Amber' not found\"}"
        );

        let db_err = AppError::Db("duplicate target".into());
        let db_json = serde_json::to_string(&db_err).unwrap();
        assert_eq!(
            db_json,
            "{\"type\":\"Db\",\"payload\":\"duplicate target\"}"
        );
    }
}
