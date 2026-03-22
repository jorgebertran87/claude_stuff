import array
import io
import math
import re

import pygame
from gtts import gTTS
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


class GTTSSpeaker(AudioSpeaker):
    def __init__(self) -> None:
        pygame.mixer.init()

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

    def speak(self, text: str, language: Language, speed: float = 1.5, pitch: float = 0.75) -> None:
        lang_code = language.code.split("-")[0]
        tts = gTTS(text=_strip_markdown(text), lang=lang_code)

        mp3_buf = io.BytesIO()
        tts.write_to_fp(mp3_buf)
        mp3_buf.seek(0)

        segment = AudioSegment.from_mp3(mp3_buf)

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

        wav_buf = io.BytesIO()
        processed.export(wav_buf, format="wav")
        wav_buf.seek(0)

        pygame.mixer.music.load(wav_buf)
        pygame.mixer.music.play()
        while pygame.mixer.music.get_busy():
            pygame.time.Clock().tick(10)
