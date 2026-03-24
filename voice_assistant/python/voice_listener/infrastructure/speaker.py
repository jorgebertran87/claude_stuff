import array
import io
import math
import re
import threading

import pygame
from gtts import gTTS
from langdetect import detect as detect_language
from pydub import AudioSegment

from voice_listener.domain.model import Language
from voice_listener.domain.ports import AudioSpeaker


def _strip_markdown(text: str) -> str:
    text = re.sub(r'\[([^\]]+)\]\([^\)]+\)', r'\1', text)  # [label](url) -> label
    text = re.sub(r'https?://\S+', '', text)                # bare URLs
    text = re.sub(r'\*+([^*]*)\*+', r'\1', text)           # bold/italic
    text = re.sub(r'^#+\s+', '', text, flags=re.MULTILINE)  # headers
    text = re.sub(r'^[-*]\s+', '', text, flags=re.MULTILINE) # list bullets
    text = re.sub(r'`[^`]*`', '', text)                     # inline code
    return text.strip()


def _tts_segment(text: str, lang: str) -> AudioSegment:
    buf = io.BytesIO()
    try:
        gTTS(text=text, lang=lang).write_to_fp(buf)
    except ValueError:
        buf = io.BytesIO()
        gTTS(text=text, lang="en").write_to_fp(buf)
    buf.seek(0)
    return AudioSegment.from_mp3(buf)


def _alexa_spotify_parts(text: str) -> list[tuple[str, str]] | None:
    """If text is an Alexa/Spotify command, return [(chunk, lang), ...]; else None.

    The command frame uses Spanish; the song title uses its detected language.
    The title is expected to be the quoted string in the response.
    """
    if not (re.search(r'\balexa\b', text, re.IGNORECASE) and
            re.search(r'\bspotify\b', text, re.IGNORECASE)):
        return None
    m = re.search(r'(["\'])(.+?)\1', text)
    if not m:
        return None
    before, title, after = text[:m.start()], m.group(2), text[m.end():]
    try:
        title_lang = detect_language(title)
    except Exception:
        title_lang = "en"
    return [(before, "es"), (title, title_lang), (after, "es")]


class GTTSSpeaker(AudioSpeaker):
    def __init__(self) -> None:
        pygame.mixer.init()
        self._echo_reference: tuple[bytes, int, int] | None = None

    def beep(self, frequency: float = 880.0, duration_ms: int = 200, volume: float = 0.6) -> None:
        sample_rate = 44100
        n_samples = int(sample_rate * duration_ms / 1000)
        buf = array.array("h", [
            int(volume * 32767 * math.sin(2 * math.pi * frequency * i / sample_rate))
            for i in range(n_samples)
        ])
        sound = pygame.mixer.Sound(buffer=buf)
        sound.play()
        pygame.time.wait(duration_ms)

    def speak(self, text: str, language: Language, on_playback_start=None, speed: float = 1.5, pitch: float = 0.75) -> None:
        cleaned = _strip_markdown(text)
        lang_code = language.code.split("-")[0]

        parts = _alexa_spotify_parts(cleaned)
        if parts:
            chunks = [_tts_segment(chunk, lang) for chunk, lang in parts if chunk.strip()]
            segment = chunks[0]
            for chunk in chunks[1:]:
                segment = segment + chunk
        else:
            segment = _tts_segment(cleaned, lang_code)

        # Speed up (pitch-preserving via resample)
        processed = segment._spawn(
            segment.raw_data,
            overrides={"frame_rate": int(segment.frame_rate * speed)},
        ).set_frame_rate(segment.frame_rate)

        # Lower pitch (deepen voice): reduce frame_rate tag so playback frequencies shift down
        processed = processed._spawn(
            processed.raw_data,
            overrides={"frame_rate": int(processed.frame_rate * pitch)},
        ).set_frame_rate(processed.frame_rate)

        mono = processed.set_channels(1) if processed.channels > 1 else processed
        self._echo_reference = (mono.raw_data, mono.frame_rate, mono.sample_width)

        wav_buf = io.BytesIO()
        processed.export(wav_buf, format="wav")
        wav_buf.seek(0)

        pygame.mixer.music.load(wav_buf)
        if on_playback_start is not None:
            on_playback_start()
        pygame.mixer.music.play()
        while pygame.mixer.music.get_busy():
            pygame.time.Clock().tick(10)
        self._echo_reference = None

    def stop(self) -> None:
        pygame.mixer.music.stop()
        self._echo_reference = None

    def get_echo_reference(self) -> tuple[bytes, int, int] | None:
        return self._echo_reference

    def play_melody(self, stop_event: threading.Event) -> None:
        while not stop_event.is_set():
            self.beep(frequency=660, duration_ms=80, volume=0.15)
            stop_event.wait(timeout=2.0)
