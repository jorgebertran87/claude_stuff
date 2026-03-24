import array
import io
import math
import re
import threading

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
        # The Entertainer by Scott Joplin
        notes = [587, 659, 523, 440, 523, 659, 523, 587, 659, 523, 659, 784, 659, 523, 440, 440, 493, 523]
        durations = [100, 100, 100, 100, 200, 100, 300, 100, 100, 100, 100, 200, 300, 100, 100, 100, 100, 300]
        pause_ms = 600
        while not stop_event.is_set():
            for freq, dur in zip(notes, durations):
                if stop_event.is_set():
                    return
                self.beep(frequency=freq, duration_ms=dur, volume=0.3)
            stop_event.wait(timeout=pause_ms / 1000)
