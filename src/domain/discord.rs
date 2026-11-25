// Our own small Discord types. Twilight's types are complete but huge; the handlers
// only need these few fields, and tests can build them in one line.

// A slash command we register with Discord.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiscordCommand {
    pub name: String,
    pub description: String,
    // EX0036: text options, e.g. `/choose question:<text>`. Empty = no options.
    pub options: Vec<DiscordOption>,
    // EX0039: hidden from everyone except the server's administrators (they can still
    // allow other roles in Server Settings > Integrations). Your handler must still check.
    pub admin_only: bool,
}

// A text option of a slash command. `choices` empty = the user types anything.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiscordOption {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub choices: Vec<String>,
}

// A button under a message. `custom_id` comes back when it's clicked.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscordButton {
    pub label: String,
    pub custom_id: String,
}

// Something a user did that the bot must answer: a slash command or a button click.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiscordInteraction {
    // Needed to answer: Discord gives each interaction an id and a short-lived token.
    pub id: u64,
    pub token: String,
    // Who did it.
    pub user_id: u64,
    // EX0039: their unique username ("nick_g") and the name others see (the server
    // nickname, else their display name). None when Discord doesn't send them.
    pub user_name: Option<String>,
    pub display_name: Option<String>,
    pub kind: InteractionKind,
    // EX0036: the user's client language ("pt-BR", "en-US"...), when Discord sends it.
    pub locale: Option<String>,
    // EX0036: the server it happened in; None in a direct message.
    pub guild_id: Option<u64>,
    // EX0036: the command's options as name/value text: `/choose question:lunch?` gives
    // [("question", "lunch?")]. Numbers, users, channels... come as their text or id.
    pub options: Vec<(String, String)>,
}

impl DiscordInteraction {
    // The value of one option, if the user gave it.
    pub fn option(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(option, _)| option == name)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InteractionKind {
    // The command name, without the slash: "ping".
    Command(String),
    // The clicked button's custom_id.
    Button(String),
}

impl Default for InteractionKind {
    fn default() -> Self {
        InteractionKind::Command(String::new())
    }
}

// EX0036: how to buy time when the answer takes longer than Discord's 3 seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Defer {
    // For a command: Discord shows "<bot> is thinking...", the answer comes as a follow-up.
    // `private` = only the user who acted will see it.
    Reply { private: bool },
    // For a button: nothing visible; the follow-up (or an edit) comes later.
    Update,
}
