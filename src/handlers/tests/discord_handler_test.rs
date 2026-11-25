use crate::domain::discord::{
    Defer, DiscordButton, DiscordCommand, DiscordInteraction, InteractionKind,
};
use crate::engine::discord_engine::MockDiscordExecutor;
use crate::engine::discord_engine::{DiscordApp, DiscordExecutor};
use crate::handlers::discord_handler::*;
use anyhow::Result;
use mockall::predicate::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use twilight_model::id::Id;
use twilight_model::user::CurrentUser;

const ALICE: u64 = 111;
const BOB: u64 = 222;

fn command(user_id: u64, name: &str) -> DiscordInteraction {
    DiscordInteraction {
        id: 900,
        token: "tok".to_string(),
        user_id,
        kind: InteractionKind::Command(name.to_string()),
        ..Default::default()
    }
}

fn click(user_id: u64, custom_id: &str) -> DiscordInteraction {
    DiscordInteraction {
        id: 901,
        token: "tok".to_string(),
        user_id,
        kind: InteractionKind::Button(custom_id.to_string()),
        ..Default::default()
    }
}

fn bot_user(name: &str, discriminator: u16) -> CurrentUser {
    CurrentUser {
        accent_color: None,
        avatar: None,
        banner: None,
        bot: true,
        discriminator,
        email: None,
        flags: None,
        global_name: None,
        id: Id::new(1),
        locale: None,
        mfa_enabled: false,
        name: name.to_string(),
        premium_type: None,
        public_flags: None,
        verified: None,
    }
}

#[tokio::test]
async fn test_ping_replies_pong_publicly() {
    let mut api = MockDiscordExecutor::new();
    api.expect_reply()
        .with(
            eq(900),
            eq("tok".to_string()),
            eq("PONG!".to_string()),
            eq(vec![]),
            eq(false),
        )
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));

    handle_interaction(&api, command(ALICE, "ping"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_choose_sends_buttons_that_remember_the_owner() {
    let expected_buttons = vec![
        DiscordButton {
            label: "Yes".to_string(),
            custom_id: format!("choose:yes:{}", ALICE),
        },
        DiscordButton {
            label: "No".to_string(),
            custom_id: format!("choose:no:{}", ALICE),
        },
    ];
    let mut api = MockDiscordExecutor::new();
    api.expect_reply()
        .with(
            eq(900),
            eq("tok".to_string()),
            eq("Pick one:".to_string()),
            eq(expected_buttons),
            eq(false),
        )
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));

    handle_interaction(&api, command(ALICE, "choose"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_owner_click_updates_the_message() {
    let mut api = MockDiscordExecutor::new();
    api.expect_update_message()
        .with(
            eq(901),
            eq("tok".to_string()),
            eq("You chose: Yes".to_string()),
            eq(vec![]),
        )
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    handle_interaction(&api, click(ALICE, &format!("choose:yes:{}", ALICE)))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_someone_else_click_gets_a_private_no() {
    // No update_message expectation: the original message must stay as it was.
    let mut api = MockDiscordExecutor::new();
    api.expect_reply()
        .with(
            eq(901),
            eq("tok".to_string()),
            eq("This choice isn't yours.".to_string()),
            eq(vec![]),
            eq(true),
        )
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));

    handle_interaction(&api, click(BOB, &format!("choose:no:{}", ALICE)))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_unknown_command_gets_a_private_answer() {
    let mut api = MockDiscordExecutor::new();
    api.expect_reply()
        .with(
            always(),
            always(),
            eq("Unknown command.".to_string()),
            always(),
            eq(true),
        )
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));

    handle_interaction(&api, command(ALICE, "old-command"))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_unknown_button_is_an_error() {
    // No expectations: nothing may be sent.
    let api = MockDiscordExecutor::new();

    let result = handle_interaction(&api, click(ALICE, "something:else")).await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "unknown button: something:else"
    );
}

#[test]
fn test_parse_choice() {
    assert_eq!(parse_choice("choose:yes:42"), Some(("Yes", 42)));
    assert_eq!(parse_choice("choose:no:42"), Some(("No", 42)));
    assert_eq!(parse_choice("choose:maybe:42"), None);
    assert_eq!(parse_choice("choose:yes"), None);
    assert_eq!(parse_choice("choose:yes:abc"), None);
    assert_eq!(parse_choice("choose:yes:42:extra"), None);
    assert_eq!(parse_choice("other:yes:42"), None);
}

#[test]
fn test_display_name_old_and_new_usernames() {
    assert_eq!(display_name(&bot_user("CardBot", 1234)), "CardBot#1234");
    assert_eq!(display_name(&bot_user("CardBot", 7)), "CardBot#0007");
    assert_eq!(display_name(&bot_user("cardbot", 0)), "cardbot");
}

#[test]
fn test_commands_are_ping_choose_and_slow() {
    let names: Vec<String> = commands().into_iter().map(|c| c.name).collect();
    assert_eq!(names, vec!["ping", "choose", "slow"]);
    // /choose takes an optional text option (EX0036).
    let choose = &commands()[1];
    assert_eq!(choose.options.len(), 1);
    assert_eq!(choose.options[0].name, "question");
    assert!(!choose.options[0].required);
}

#[test]
fn test_guild_id_from_env() {
    assert_eq!(discord_guild_id_from(None), None);
    assert_eq!(discord_guild_id_from(Some("".to_string())), None);
    assert_eq!(
        discord_guild_id_from(Some(" 123456789 ".to_string())),
        Some(123456789)
    );
}

#[test]
#[should_panic(expected = "DISCORD_GUILD_ID must be a server id (a number).")]
fn test_guild_id_not_a_number_panics() {
    discord_guild_id_from(Some("my-server".to_string()));
}

// ---------------------------------------------------------------------------------------
// EX0036: options, defer/follow-up, parallel interactions, settings
// ---------------------------------------------------------------------------------------

#[tokio::test]
async fn test_choose_puts_the_question_option_in_the_text() {
    let mut api = MockDiscordExecutor::new();
    api.expect_reply()
        .withf(|_, _, text, buttons, _| text == "Pick one: lunch?" && buttons.len() == 2)
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));
    let mut interaction = command(ALICE, "choose");
    interaction.options = vec![("question".to_string(), "lunch?".to_string())];

    handle_interaction(&api, interaction).await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn test_slow_defers_then_follows_up() {
    let mut api = MockDiscordExecutor::new();
    let mut order = mockall::Sequence::new();
    api.expect_defer()
        .with(
            eq(900),
            eq("tok".to_string()),
            eq(Defer::Reply { private: false }),
        )
        .times(1)
        .in_sequence(&mut order)
        .returning(|_, _, _| Ok(()));
    api.expect_follow_up()
        .withf(|token, text, _, private| token == "tok" && text == "Done after 5 s" && !private)
        .times(1)
        .in_sequence(&mut order)
        .returning(|_, _, _, _| Ok(()));

    let started = tokio::time::Instant::now();
    handle_interaction(&api, command(ALICE, "slow"))
        .await
        .unwrap();
    assert!(started.elapsed() >= SLOW_DEMO_DELAY);
}

// An app that records which interactions finished, in order. "slow" takes 5 s.
struct RecordingApp {
    finished: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl DiscordApp for RecordingApp {
    fn commands(&self) -> Vec<DiscordCommand> {
        vec![]
    }
    async fn handle(
        &self,
        _api: Arc<dyn DiscordExecutor>,
        interaction: DiscordInteraction,
    ) -> Result<()> {
        let InteractionKind::Command(name) = interaction.kind else {
            return Ok(());
        };
        if name == "slow" {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        self.finished.lock().unwrap().push(name);
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn test_slow_interaction_does_not_delay_the_next_one() {
    let app = Arc::new(RecordingApp {
        finished: Mutex::new(vec![]),
    });
    let api: Arc<dyn DiscordExecutor> = Arc::new(MockDiscordExecutor::new());
    let mut tasks = InteractionTasks::new(DEFAULT_MAX_CONCURRENT);

    tasks
        .spawn(app.clone(), api.clone(), command(ALICE, "slow"))
        .await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    tasks
        .spawn(app.clone(), api.clone(), command(BOB, "ping"))
        .await;
    // /ping, sent 1 s after /slow, is done right away.
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(*app.finished.lock().unwrap(), vec!["ping"]);

    tasks.finish(Duration::from_secs(10)).await;
    assert_eq!(*app.finished.lock().unwrap(), vec!["ping", "slow"]);
}

#[tokio::test(start_paused = true)]
async fn test_shutdown_drops_interactions_that_take_too_long() {
    let app = Arc::new(RecordingApp {
        finished: Mutex::new(vec![]),
    });
    let api: Arc<dyn DiscordExecutor> = Arc::new(MockDiscordExecutor::new());
    let mut tasks = InteractionTasks::new(DEFAULT_MAX_CONCURRENT);
    tasks.spawn(app.clone(), api, command(ALICE, "slow")).await;
    assert_eq!(tasks.running(), 1);

    // 1 s to finish, the interaction needs 5: it's dropped, never recorded.
    tasks.finish(Duration::from_secs(1)).await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert!(app.finished.lock().unwrap().is_empty());
}

#[tokio::test(start_paused = true)]
async fn test_max_concurrent_makes_the_next_interaction_wait() {
    let app = Arc::new(RecordingApp {
        finished: Mutex::new(vec![]),
    });
    let api: Arc<dyn DiscordExecutor> = Arc::new(MockDiscordExecutor::new());
    let mut tasks = InteractionTasks::new(1);
    tasks
        .spawn(app.clone(), api.clone(), command(ALICE, "slow"))
        .await;

    // Only 1 at a time: the second spawn waits until /slow is done (5 s).
    let started = tokio::time::Instant::now();
    tasks.spawn(app.clone(), api, command(BOB, "ping")).await;
    assert!(started.elapsed() >= Duration::from_secs(5));
    tasks.finish(Duration::from_secs(1)).await;
    assert_eq!(*app.finished.lock().unwrap(), vec!["slow", "ping"]);
}

#[test]
fn test_max_concurrent_from_env() {
    assert_eq!(max_concurrent_from(None), DEFAULT_MAX_CONCURRENT);
    assert_eq!(max_concurrent_from(Some(" 8 ".to_string())), 8);
}

#[test]
#[should_panic(expected = "DISCORD_MAX_CONCURRENT must be a number greater than 0.")]
fn test_max_concurrent_zero_panics() {
    max_concurrent_from(Some("0".to_string()));
}

#[tokio::test]
async fn test_demo_app_registers_its_commands_and_handles_ping() {
    let app = DemoDiscordApp;
    assert_eq!(app.commands(), commands());
    let mut api = MockDiscordExecutor::new();
    api.expect_reply()
        .withf(|_, _, text, _, _| text == "PONG!")
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));

    app.handle(Arc::new(api), command(ALICE, "ping"))
        .await
        .unwrap();
}
