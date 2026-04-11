#[derive(Debug)]
pub enum CliArgs {
    DirectOrder(String),
    ListenMode,
    TelegramMode,
    BothMode,
}

/// Parses the command-line arguments and returns the resolved mode.
/// Returns `Err` if `--order` is present but has no value.
pub fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    if let Some(pos) = args.iter().position(|a| a == "--order") {
        match args.get(pos + 1) {
            Some(order) => Ok(CliArgs::DirectOrder(order.clone())),
            None => Err("--order requires a value".into()),
        }
    } else if args.iter().any(|a| a == "--both") {
        Ok(CliArgs::BothMode)
    } else if args.iter().any(|a| a == "--telegram") {
        Ok(CliArgs::TelegramMode)
    } else {
        Ok(CliArgs::ListenMode)
    }
}
