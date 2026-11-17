use tokio::net::TcpListener;
use tokio::task::JoinHandle;

// A fake Discord gateway for the tests that start the whole app: it accepts the bot's
// connection and never answers, so the bot just waits (no Ready = no REST calls at all).
// Tests must never talk to the real Discord. Returns the fake's task.
pub async fn start_fake_discord_gateway() -> JoinHandle<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let fake = tokio::spawn(async move {
        let mut open_connections = Vec::new();
        while let Ok((socket, _)) = listener.accept().await {
            open_connections.push(socket); // keep it open, say nothing
        }
    });
    unsafe {
        std::env::set_var("DISCORD_TOKEN", "fake-discord-token");
        std::env::set_var("DISCORD_GATEWAY_URL", format!("ws://{}", address));
    }
    fake
}
