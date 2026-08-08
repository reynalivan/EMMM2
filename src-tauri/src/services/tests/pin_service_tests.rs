use super::PinVerdict;
use super::{get_status, set_pin, verify_pin};
use crate::repo::pin_repo;
use crate::test_utils::init_test_db;

#[tokio::test]
async fn verify_pin_persists_sixty_second_lockout_in_db() {
    let ctx = init_test_db().await;

    set_pin(&ctx.pool, "123456", None).await.expect("set pin");

    for _ in 0..5 {
        let verdict = verify_pin(&ctx.pool, "000000").await.expect("verify pin");
        assert!(!matches!(verdict, PinVerdict::Accepted));
    }

    let status = get_status(&ctx.pool).await.expect("get status");
    assert!(status.is_locked);
    assert!(status.lockout_seconds_remaining > 0);
    assert!(status.lockout_seconds_remaining <= 60);

    let db_status = pin_repo::get(&ctx.pool).await.expect("pin config");
    assert_eq!(db_status.failed_attempts, 0);
    assert!(
        db_status.lockout_until.is_some(),
        "lockout must survive service restart through pin_config"
    );
}
