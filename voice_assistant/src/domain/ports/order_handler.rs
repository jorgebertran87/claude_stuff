pub trait OrderHandler: Send + Sync {
    fn handle(&self, order: &str) -> String;
    fn reset_session(&self);
}
