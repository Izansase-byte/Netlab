pub(crate) enum Commands {
    Quit,
    Help,
    Message,
    Users,
}

pub(crate) fn parse_command(input: &str) -> Commands {
    match input {
        "/quit" => Commands::Quit,
        "/help" => Commands::Help,
        "/users" => Commands::Users,
        _ => Commands::Message,
    }
}
