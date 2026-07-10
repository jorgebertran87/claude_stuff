/// Port for playing synthesized audio and releasing the output device.
pub trait AudioPlayer: Send + Sync {
    fn play(&self, bytes: &[u8]);
    fn disconnect(&self);
}
