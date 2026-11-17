use crate::domain::discord::{DiscordButton, DiscordInteraction, InteractionKind};
use crate::engine::discord_engine::MockDiscordExecutor;
use crate::handlers::discord_handler::*;
use mockall::predicate::*;
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
    }
}

fn click(user_id: u64, custom_id: &str) -> DiscordInteraction {
    DiscordInteraction {
        id: 901,
        token: "tok".to_string(),
        user_id,
        kind: InteractionKind::Button(custom_id.to_string()),
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
        )
        .times(1)
        .returning(|_, _, _| Ok(()));

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
fn test_commands_are_ping_and_choose() {
    let names: Vec<String> = commands().into_iter().map(|c| c.name).collect();
    assert_eq!(names, vec!["ping", "choose"]);
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
