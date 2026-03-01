use crate::types::errors::CommandError;
use sqlx::Error as SqlxError;

#[test]
fn test_command_error_from_sqlx() {
    let sqlx_err = SqlxError::RowNotFound;
    let cmd_err = CommandError::from(sqlx_err);

    match cmd_err {
        CommandError::Database(msg) => {
            assert!(msg.contains("no rows returned"));
        }
        _ => panic!("Expected CommandError::Database"),
    }
}

#[test]
fn test_command_error_serialization() {
    let err = CommandError::NotFound("Item X not found".to_string());

    // CommandError serializes as just its Display string
    let serialized = serde_json::to_string(&err).unwrap();
    assert_eq!(serialized, "\"Not found: Item X not found\"");
}
