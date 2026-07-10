use crate::domain::model::{AudioCapture, Language};

pub trait Transcriber: Send + Sync {
    fn transcribe(&self, audio: &AudioCapture, language: &Language) -> Option<String>;
}
