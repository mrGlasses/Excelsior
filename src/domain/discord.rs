// Our own small Discord types. Twilight's types are complete but huge; the handlers
// only need these few fields, and tests can build them in one line.

// A slash command we register with Discord.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscordCommand {
    pub name: String,
    pub description: String,
}

// A button under a message. `custom_id` comes back when it's clicked.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscordButton {
    pub label: String,
    pub custom_id: String,
}

// Something a user did that the bot must answer: a slash command or a button click.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscordInteraction {
    // Needed to answer: Discord gives each interaction an id and a short-lived token.
    pub id: u64,
    pub token: String,
    // Who did it.
    pub user_id: u64,
    pub kind: InteractionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InteractionKind {
    // The command name, without the slash: "ping".
    Command(String),
    // The clicked button's custom_id.
    Button(String),
}
