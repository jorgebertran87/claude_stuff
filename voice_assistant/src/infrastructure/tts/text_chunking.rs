//! Text chunking utilities for TTS engines.
//! Splits text into sentences and chunks suitable for TTS synthesis.
//! The TTS engines themselves are in `engine.rs` and `google_tts.rs`.

const MAX_TTS_CHARS: usize = 200;

pub fn tts_chunks(text: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let sentences = split_sentences(text);
    let mut current = String::new();
    for sentence in sentences {
        if current.len() + sentence.len() <= MAX_TTS_CHARS {
            current.push_str(&sentence);
        } else {
            if !current.trim().is_empty() {
                chunks.push(current.trim().to_string());
            }
            if sentence.len() > MAX_TTS_CHARS {
                let mut word_buf = String::new();
                for word in sentence.split_whitespace() {
                    if word_buf.len() + word.len() + 1 > MAX_TTS_CHARS {
                        if !word_buf.trim().is_empty() {
                            chunks.push(word_buf.trim().to_string());
                        }
                        word_buf = word.to_string();
                    } else {
                        if !word_buf.is_empty() { word_buf.push(' '); }
                        word_buf.push_str(word);
                    }
                }
                current = word_buf;
            } else {
                current = sentence;
            }
        }
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    if chunks.is_empty() { chunks.push(text.to_string()); }
    chunks
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        current.push(c);
        if matches!(c, '.' | '!' | '?') {
            if chars.peek().map(|&n| n == ' ' || n == '\n').unwrap_or(true) {
                result.push(current.clone());
                current.clear();
            }
        } else if c == '\n' {
            result.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() { result.push(current); }
    result
}
