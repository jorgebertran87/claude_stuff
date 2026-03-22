import io
import re

import pygame
from gtts import gTTS

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

    def speak(self, text: str, language: Language) -> None:
        lang_code = language.code.split("-")[0]
        tts = gTTS(text=_strip_markdown(text), lang=lang_code)

        mp3_buf = io.BytesIO()
        tts.write_to_fp(mp3_buf)
        mp3_buf.seek(0)

        pygame.mixer.music.load(mp3_buf)
        pygame.mixer.music.play()
        while pygame.mixer.music.get_busy():
            pygame.time.Clock().tick(10)
