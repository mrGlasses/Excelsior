use crate::utils::un_utils::*;

#[tokio::test]
async fn test_start_message() {
    start_message("vSpy vSpy - Don't tear it down".to_string()).await;
    assert!(true);
}
