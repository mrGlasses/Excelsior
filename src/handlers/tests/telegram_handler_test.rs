use crate::domain::telegram::*;
use crate::engine::telegram_engine::MockTelegramExecutor;
use crate::handlers::telegram_handler::*;
use anyhow::anyhow;
use mockall::predicate::*;
use tokio_util::sync::CancellationToken;

fn command_update(update_id: i64, text: &str) -> Update {
    Update {
        update_id,
        message: Some(IncomingMessage {
            message_id: 1,
            chat: Chat { id: 5 },
            text: Some(text.to_string()),
        }),
        callback_query: None,
    }
}

fn button_tap(data: &str) -> Update {
    Update {
        update_id: 1,
        message: None,
        callback_query: Some(CallbackQuery {
            id: "tap-1".to_string(),
            data: Some(data.to_string()),
            message: Some(IncomingMessage {
                message_id: 43,
                chat: Chat { id: 5 },
                text: Some("Pick one:".to_string()),
            }),
        }),
    }
}

fn bot_user() -> BotUser {
    BotUser {
        id: 1,
        username: Some("CardBot".to_string()),
    }
}

#[test]
fn test_command_name() {
    assert_eq!(command_name("/ping"), "/ping");
    assert_eq!(command_name("/ping@CardBot"), "/ping");
    assert_eq!(command_name("/choose now please"), "/choose");
    assert_eq!(command_name(""), "");
}

#[tokio::test]
async fn test_ping_replies_pong() {
    let mut api = MockTelegramExecutor::new();
    api.expect_send_message()
        .with(eq(5), eq("PONG!".to_string()), eq(None))
        .times(1)
        .returning(|_, _, _| Ok(()));

    handle_update(&api, command_update(1, "/ping"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_ping_with_bot_name_in_group() {
    let mut api = MockTelegramExecutor::new();
    api.expect_send_message()
        .with(eq(5), eq("PONG!".to_string()), eq(None))
        .times(1)
        .returning(|_, _, _| Ok(()));

    handle_update(&api, command_update(1, "/ping@CardBot"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_unknown_command() {
    let mut api = MockTelegramExecutor::new();
    api.expect_send_message()
        .with(
            eq(5),
            eq("Unknown command. Try /ping.".to_string()),
            eq(None),
        )
        .times(1)
        .returning(|_, _, _| Ok(()));

    handle_update(&api, command_update(1, "/foo"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_plain_text_is_ignored() {
    // No expectations: any call to the mock would fail the test.
    let api = MockTelegramExecutor::new();

    handle_update(&api, command_update(1, "hello bot"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_choose_sends_yes_no_buttons() {
    let expected_keyboard = InlineKeyboardMarkup {
        inline_keyboard: vec![vec![
            InlineKeyboardButton {
                text: "Yes".to_string(),
                callback_data: "choose:yes".to_string(),
            },
            InlineKeyboardButton {
                text: "No".to_string(),
                callback_data: "choose:no".to_string(),
            },
        ]],
    };
    let mut api = MockTelegramExecutor::new();
    api.expect_send_message()
        .with(
            eq(5),
            eq("Pick one:".to_string()),
            eq(Some(expected_keyboard)),
        )
        .times(1)
        .returning(|_, _, _| Ok(()));

    handle_update(&api, command_update(1, "/choose"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_tap_yes_edits_message_and_stops_spinner() {
    let mut api = MockTelegramExecutor::new();
    api.expect_edit_message_text()
        .with(eq(5), eq(43), eq("You chose: Yes".to_string()))
        .times(1)
        .returning(|_, _, _| Ok(()));
    api.expect_answer_callback_query()
        .with(eq("tap-1".to_string()))
        .times(1)
        .returning(|_| Ok(()));

    handle_update(&api, button_tap("choose:yes")).await.unwrap();
}

#[tokio::test]
async fn test_tap_still_answered_when_edit_fails() {
    let mut api = MockTelegramExecutor::new();
    api.expect_edit_message_text()
        .times(1)
        .returning(|_, _, _| Err(anyhow!("message is not modified")));
    api.expect_answer_callback_query()
        .times(1)
        .returning(|_| Ok(()));

    let result = handle_update(&api, button_tap("choose:no")).await;

    assert_eq!(result.unwrap_err().to_string(), "message is not modified");
}

#[tokio::test]
async fn test_unknown_button_only_stops_spinner() {
    let mut api = MockTelegramExecutor::new();
    api.expect_answer_callback_query()
        .times(1)
        .returning(|_| Ok(()));

    handle_update(&api, button_tap("something:else"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_bot_handles_update_then_confirms_it_on_shutdown() {
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    let mut api = MockTelegramExecutor::new();

    api.expect_get_me().times(1).returning(|| Ok(bot_user()));
    // First poll: no offset yet, one /ping arrives.
    api.expect_get_updates()
        .with(eq(None), eq(POLL_TIMEOUT_SECS))
        .times(1)
        .returning(|_, _| Ok(vec![command_update(7, "/ping")]));
    api.expect_send_message()
        .with(eq(5), eq("PONG!".to_string()), eq(None))
        .times(1)
        .returning(|_, _, _| Ok(()));
    // Second poll asks for update 8 (confirming 7); meanwhile, Ctrl+C.
    api.expect_get_updates()
        .with(eq(Some(8)), eq(POLL_TIMEOUT_SECS))
        .times(1)
        .returning(move |_, _| {
            stop.cancel();
            Ok(vec![])
        });
    // Last call on the way out: confirm 7 without waiting.
    api.expect_get_updates()
        .with(eq(Some(8)), eq(0))
        .times(1)
        .returning(|_, _| Ok(vec![]));

    run_telegram_bot(Box::new(api), shutdown).await.unwrap();
}

#[tokio::test]
async fn test_failing_update_is_skipped_not_retried() {
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    let mut api = MockTelegramExecutor::new();

    api.expect_get_me().times(1).returning(|| Ok(bot_user()));
    api.expect_get_updates()
        .with(eq(None), eq(POLL_TIMEOUT_SECS))
        .times(1)
        .returning(|_, _| Ok(vec![command_update(7, "/ping")]));
    api.expect_send_message()
        .times(1)
        .returning(|_, _, _| Err(anyhow!("chat not found")));
    // Even though answering failed, the next poll moves on to update 8.
    api.expect_get_updates()
        .with(eq(Some(8)), always())
        .returning(move |_, _| {
            stop.cancel();
            Ok(vec![])
        });

    run_telegram_bot(Box::new(api), shutdown).await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn test_polling_error_is_retried() {
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    let mut api = MockTelegramExecutor::new();
    let mut calls = 0;

    api.expect_get_me().times(1).returning(|| Ok(bot_user()));
    api.expect_get_updates().times(2).returning(move |_, _| {
        calls += 1;
        if calls == 1 {
            Err(anyhow!("connection reset"))
        } else {
            stop.cancel();
            Ok(vec![])
        }
    });

    // Returns Ok: a network error while polling doesn't kill the worker.
    run_telegram_bot(Box::new(api), shutdown).await.unwrap();
}

#[tokio::test]
async fn test_wrong_token_fails_the_worker() {
    let mut api = MockTelegramExecutor::new();
    api.expect_get_me()
        .times(1)
        .returning(|| Err(anyhow!("telegram getMe failed: Unauthorized")));

    let result = run_telegram_bot(Box::new(api), CancellationToken::new()).await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "telegram getMe failed: Unauthorized"
    );
}
