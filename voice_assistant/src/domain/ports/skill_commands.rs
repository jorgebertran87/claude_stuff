/// Port for the slash-command skills exposed by the Telegram bot.
pub trait SkillCommands: Send + Sync {
    fn bus(&self, model: &str, stop_code: &str) -> String;
    fn volume(&self, arg: &str) -> String;
    fn usage_report(&self, log_file: &str) -> String;
    fn connect_speakers(&self) -> String;
}
