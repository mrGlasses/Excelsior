use crate::domain::discord::InteractionKind;
use crate::engine::discord_engine::to_discord_interaction;
use twilight_model::application::interaction::Interaction;
use twilight_model::id::Id;

// Shapes of real interactions as Discord sends them (only some fields; twilight
// fills the rest with defaults or ignores them).
const SLASH_COMMAND_IN_SERVER: &str = r#"{
    "id": "1300000000000000001",
    "application_id": "1200000000000000000",
    "type": 2,
    "token": "interaction-token",
    "version": 1,
    "attachment_size_limit": 8388608,
    "authorizing_integration_owners": {},
    "entitlements": [],
    "guild_id": "1100000000000000000",
    "channel_id": "1000000000000000000",
    "member": {
        "user": {"id": "111", "username": "alice", "discriminator": "0", "avatar": null},
        "roles": [], "joined_at": "2026-01-01T00:00:00.000000+00:00",
        "deaf": false, "mute": false, "flags": 0
    },
    "data": {"id": "1250000000000000000", "name": "ping", "type": 1}
}"#;

const BUTTON_CLICK_IN_DM: &str = r#"{
    "id": "1300000000000000002",
    "application_id": "1200000000000000000",
    "type": 3,
    "token": "interaction-token-2",
    "version": 1,
    "attachment_size_limit": 8388608,
    "authorizing_integration_owners": {},
    "entitlements": [],
    "channel_id": "1000000000000000000",
    "user": {"id": "222", "username": "bob", "discriminator": "0", "avatar": null},
    "data": {"custom_id": "choose:yes:111", "component_type": 2}
}"#;

#[test]
fn test_slash_command_from_a_server_member() {
    let interaction: Interaction = serde_json::from_str(SLASH_COMMAND_IN_SERVER).unwrap();

    let ours = to_discord_interaction(&interaction).unwrap();

    assert_eq!(ours.id, 1300000000000000001);
    assert_eq!(ours.token, "interaction-token");
    // In a server, the author comes from `member.user`.
    assert_eq!(ours.user_id, 111);
    assert_eq!(ours.kind, InteractionKind::Command("ping".to_string()));
}

#[test]
fn test_button_click_from_a_direct_message() {
    let interaction: Interaction = serde_json::from_str(BUTTON_CLICK_IN_DM).unwrap();

    let ours = to_discord_interaction(&interaction).unwrap();

    // In a DM, the author comes from `user`.
    assert_eq!(ours.user_id, 222);
    assert_eq!(
        ours.kind,
        InteractionKind::Button("choose:yes:111".to_string())
    );
}

#[test]
fn test_interaction_without_data_is_ignored() {
    // type 1 = PING (Discord checking the endpoint): nothing for the bot to answer.
    let json = r#"{
        "id": "1300000000000000003",
        "application_id": "1200000000000000000",
        "type": 1,
        "token": "t",
        "version": 1,
        "attachment_size_limit": 0,
        "authorizing_integration_owners": {},
        "entitlements": []
    }"#;
    let interaction: Interaction = serde_json::from_str(json).unwrap();

    assert_eq!(to_discord_interaction(&interaction), None);
}

// ---------------------------------------------------------------------------------------
// EX0036: locale, server, options; defer; DMs; the Missing Access message
// ---------------------------------------------------------------------------------------
use crate::domain::discord::{Defer, DiscordButton, DiscordCommand, DiscordOption};
use crate::engine::discord_engine::{
    DiscordApi, DiscordExecutor, DiscordMessenger, DiscordSender, defer_response,
    to_twilight_command, update_response,
};
use httpmock::prelude::*;
use serde_json::json;
use std::sync::Arc;
use twilight_model::http::interaction::InteractionResponseType;

// /choose question:lunch? from a Brazilian client, in a server.
const SLASH_COMMAND_WITH_OPTION: &str = r#"{
    "id": "1300000000000000004",
    "application_id": "1200000000000000000",
    "type": 2,
    "token": "interaction-token-4",
    "version": 1,
    "attachment_size_limit": 8388608,
    "authorizing_integration_owners": {},
    "entitlements": [],
    "guild_id": "1100000000000000000",
    "channel_id": "1000000000000000000",
    "locale": "pt-BR",
    "guild_locale": "en-US",
    "member": {
        "user": {"id": "111", "username": "alice", "discriminator": "0", "avatar": null},
        "roles": [], "joined_at": "2026-01-01T00:00:00.000000+00:00",
        "deaf": false, "mute": false, "flags": 0
    },
    "data": {
        "id": "1250000000000000001", "name": "choose", "type": 1,
        "options": [
            {"name": "question", "type": 3, "value": "lunch?"},
            {"name": "count", "type": 4, "value": 3}
        ]
    }
}"#;

#[test]
fn test_locale_server_and_options_are_read() {
    let interaction: Interaction = serde_json::from_str(SLASH_COMMAND_WITH_OPTION).unwrap();

    let ours = to_discord_interaction(&interaction).unwrap();

    // The user's own language, not the server's.
    assert_eq!(ours.locale.as_deref(), Some("pt-BR"));
    assert_eq!(ours.guild_id, Some(1100000000000000000));
    assert_eq!(
        ours.options,
        vec![
            ("question".to_string(), "lunch?".to_string()),
            ("count".to_string(), "3".to_string())
        ]
    );
    assert_eq!(ours.option("question"), Some("lunch?"));
    assert_eq!(ours.option("missing"), None);
}

#[test]
fn test_direct_message_has_no_server() {
    let interaction: Interaction = serde_json::from_str(BUTTON_CLICK_IN_DM).unwrap();
    let ours = to_discord_interaction(&interaction).unwrap();
    assert_eq!(ours.guild_id, None);
    assert!(ours.options.is_empty());
}

#[test]
fn test_command_options_are_registered() {
    let command = DiscordCommand {
        name: "play".to_string(),
        description: "Join a queue".to_string(),
        options: vec![DiscordOption {
            name: "group".to_string(),
            description: "Which group".to_string(),
            required: true,
            choices: vec!["vehicles".to_string(), "animals".to_string()],
        }],
        admin_only: false,
    };

    let twilight = to_twilight_command(&command);

    assert_eq!(twilight.options.len(), 1);
    assert_eq!(twilight.options[0].name, "group");
    assert_eq!(twilight.options[0].required, Some(true));
    assert_eq!(
        twilight.options[0].choices.as_ref().map(|c| c.len()),
        Some(2)
    );
}

#[test]
fn test_defer_response_types() {
    let reply = defer_response(Defer::Reply { private: true });
    assert_eq!(
        reply.kind,
        InteractionResponseType::DeferredChannelMessageWithSource
    );
    assert!(reply.data.unwrap().flags.is_some());
    let update = defer_response(Defer::Update);
    assert_eq!(update.kind, InteractionResponseType::DeferredUpdateMessage);
    assert!(update.data.is_none());
}

// A twilight client that sends its REST calls to the fake server, over plain HTTP.
fn fake_discord_client(server: &MockServer) -> Arc<twilight_http::Client> {
    Arc::new(
        twilight_http::Client::builder()
            .token("fake-token".to_string())
            .proxy(server.address().to_string(), true)
            .ratelimiter(None)
            .build(),
    )
}

#[tokio::test]
async fn test_send_dm_opens_the_dm_channel_then_posts() {
    let server = MockServer::start_async().await;
    let open_dm = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/api/v10/users/@me/channels")
                .json_body_includes(r#"{"recipient_id": "111"}"#);
            then.status(200)
                .json_body(json!({"id": "555", "type": 1, "recipients": []}));
        })
        .await;
    let post = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/api/v10/channels/555/messages")
                .json_body_includes(r#"{"content": "Your turn!"}"#);
            then.status(200).json_body(json!({}));
        })
        .await;
    let sender: Arc<dyn DiscordMessenger> =
        Arc::new(DiscordSender::new(fake_discord_client(&server)));

    // From a plain task, as another worker would (not the bot worker).
    tokio::spawn(async move { sender.send_dm(111, "Your turn!".to_string(), vec![]).await })
        .await
        .unwrap()
        .unwrap();

    open_dm.assert_async().await;
    post.assert_async().await;
}

#[tokio::test]
async fn test_missing_access_explains_what_to_do() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method("PUT").path(
                "/api/v10/applications/1200000000000000000/guilds/302638652041986060/commands",
            );
            then.status(403)
                .json_body(json!({"message": "Missing Access", "code": 50001}));
        })
        .await;
    let api = DiscordApi::new(fake_discord_client(&server), Id::new(1200000000000000000));

    let err = api
        .register_commands(Some(302638652041986060), vec![])
        .await
        .unwrap_err()
        .to_string();

    assert!(err.contains("server 302638652041986060"), "{err}");
    assert!(err.contains("applications.commands"), "{err}");
    assert!(err.contains("unset DISCORD_GUILD_ID"), "{err}");
}

#[tokio::test]
async fn test_follow_up_posts_to_the_interaction_webhook() {
    let server = MockServer::start_async().await;
    let follow_up = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/api/v10/webhooks/1200000000000000000/tok")
                .json_body_includes(r#"{"content": "Done after 5 s", "flags": 64}"#);
            then.status(200).json_body(json!({}));
        })
        .await;
    let api = DiscordApi::new(fake_discord_client(&server), Id::new(1200000000000000000));

    api.follow_up(
        "tok".to_string(),
        "Done after 5 s".to_string(),
        vec![],
        true,
    )
    .await
    .unwrap();

    follow_up.assert_async().await;
}

// EX0039: a member with a server nickname and a display name.
const COMMAND_FROM_NICKNAMED_MEMBER: &str = r#"{
    "id": "1300000000000000003",
    "application_id": "1200000000000000000",
    "type": 2,
    "token": "interaction-token-3",
    "version": 1,
    "attachment_size_limit": 8388608,
    "authorizing_integration_owners": {},
    "entitlements": [],
    "guild_id": "1100000000000000000",
    "channel_id": "1000000000000000000",
    "member": {
        "user": {"id": "333", "username": "carol_c", "global_name": "Carol", "discriminator": "0", "avatar": null},
        "nick": "Captain Carol",
        "roles": [], "joined_at": "2026-01-01T00:00:00.000000+00:00",
        "deaf": false, "mute": false, "flags": 0
    },
    "data": {"id": "1250000000000000000", "name": "ping", "type": 1}
}"#;

#[test]
fn test_names_come_from_nickname_then_display_name_then_username() {
    let nicknamed: Interaction = serde_json::from_str(COMMAND_FROM_NICKNAMED_MEMBER).unwrap();
    let ours = to_discord_interaction(&nicknamed).unwrap();
    assert_eq!(ours.user_name.as_deref(), Some("carol_c"));
    assert_eq!(ours.display_name.as_deref(), Some("Captain Carol"));

    // No nickname, no display name: the username.
    let plain: Interaction = serde_json::from_str(BUTTON_CLICK_IN_DM).unwrap();
    let ours = to_discord_interaction(&plain).unwrap();
    assert_eq!(ours.user_name.as_deref(), Some("bob"));
    assert_eq!(ours.display_name.as_deref(), Some("bob"));
}

#[test]
fn test_admin_only_commands_need_a_permission_nobody_has_by_default() {
    let mut command = DiscordCommand {
        name: "admin".to_string(),
        description: "Admin tools".to_string(),
        ..Default::default()
    };
    assert_eq!(
        to_twilight_command(&command).default_member_permissions,
        None
    );

    command.admin_only = true;
    assert_eq!(
        to_twilight_command(&command).default_member_permissions,
        Some(twilight_model::guild::Permissions::empty())
    );
}

#[test]
fn test_update_response_replaces_the_buttons() {
    let paged = update_response(
        "Page 2".to_string(),
        vec![DiscordButton {
            label: "◀".to_string(),
            custom_id: "page:1".to_string(),
        }],
    );
    assert_eq!(paged.kind, InteractionResponseType::UpdateMessage);
    let data = paged.data.unwrap();
    assert_eq!(data.content.as_deref(), Some("Page 2"));
    assert_eq!(data.components.unwrap().len(), 1);

    // No buttons: an empty list, so Discord removes the old ones.
    let done = update_response("Done".to_string(), vec![]);
    assert_eq!(done.data.unwrap().components, Some(vec![]));
}

#[test]
fn test_buttons_wrap_five_per_row() {
    use crate::engine::discord_engine::button_rows;
    let buttons = |n: usize| -> Vec<DiscordButton> {
        (0..n)
            .map(|i| DiscordButton {
                label: format!("b{i}"),
                custom_id: format!("b:{i}"),
            })
            .collect()
    };
    assert!(button_rows(buttons(0)).is_empty());
    assert_eq!(button_rows(buttons(5)).len(), 1);
    assert_eq!(button_rows(buttons(7)).len(), 2);
    // Discord allows 5 rows: the rest are dropped instead of failing the message.
    assert_eq!(button_rows(buttons(30)).len(), 5);
}

// EX0041: a DM with a picture: the text as content, the picture as an embed.
#[tokio::test]
async fn test_send_dm_with_image_posts_an_image_embed() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method("POST").path("/api/v10/users/@me/channels");
            then.status(200)
                .json_body(json!({"id": "555", "type": 1, "recipients": []}));
        })
        .await;
    let post = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/api/v10/channels/555/messages")
                .json_body_includes(r#"{"content": "Opala"}"#)
                .json_body_includes(
                    r#"{"embeds": [{"image": {"url": "https://art.example/opala.png"}}]}"#,
                );
            then.status(200).json_body(json!({}));
        })
        .await;
    let sender = DiscordSender::new(fake_discord_client(&server));

    sender
        .send_dm_with_image(
            111,
            "Opala".to_string(),
            "https://art.example/opala.png".to_string(),
            vec![],
        )
        .await
        .unwrap();

    post.assert_async().await;
}

#[test]
fn test_image_embed_needs_a_web_url() {
    use crate::engine::discord_engine::image_embed;
    let embed = image_embed("https://art.example/a.png").unwrap();
    assert_eq!(embed.image.unwrap().url, "https://art.example/a.png");
    assert!(image_embed("not a url").is_err());
}
