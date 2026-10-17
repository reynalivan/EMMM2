use super::*;

#[tokio::test]
async fn shared_worker_rejects_duplicate_requests_without_queueing() {
    let permit = worker().clone().try_acquire_owned().unwrap();
    let result = validate_game_executable(ValidateGameExecutableRequest {
        game_type: crate::modules::games::domain::models::GameType::Efmi,
        executable_path: "C:/not-read/Endfield.exe".into(),
    })
    .await;
    assert!(
        matches!(result,Err(AppError::Validation(message)) if message.contains("already running"))
    );
    drop(permit);
    assert!(worker().clone().try_acquire_owned().is_ok());
}
