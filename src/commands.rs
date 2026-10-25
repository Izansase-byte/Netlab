pub(crate) enum Commands {
    Quit,
    Help,
    Message,
}

pub(crate) fn parse_command(input: &str) -> Commands {
    match input {
        "/quit" => Commands::Quit,
        "/help" => Commands::Help,
        _ => Commands::Message,
    }
}
