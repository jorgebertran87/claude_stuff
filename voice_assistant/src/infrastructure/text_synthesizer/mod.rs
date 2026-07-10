pub mod gtts;
pub mod piper;

pub use gtts::{synthesize_text, synthesize_alexa_spotify, GttsTextSynthesizer};
pub use piper::PiperTextSynthesizer;
