pub async fn start_message(location: String) {
    println!("             ___");
    println!(" ____       /_ _\\ ____   ____   _      ____   _____   _____   ____   ");
    println!("|  __|  _    / / /   _| |  __| | |    / ___| |_   _| /  _  \\ |  _ \\ ");
    println!("| |__  \\ \\  / / /  /    | |__  | |    | |__    | |   | | | | | |_| |");
    println!("|  __|  \\ \\/ /  |  |    |  __| | |    \\__  \\   | |   | | | | |    / ");
    println!("| |__   / /\\ \\  \\  \\__  | |__  | |__   __| |  _| |_  | |_| | | |\\ \\ ");
    println!(
        "|____| |_/  \\_|  \\____| |____| |____| |____/ |_____| \\_____/ |_| \\_\\   ver. {}",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Server listening on {}.", location)
}
