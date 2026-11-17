use crate::domain::discord::InteractionKind;
use crate::engine::discord_engine::to_discord_interaction;
use twilight_model::application::interaction::Interaction;

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
