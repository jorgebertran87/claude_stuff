import speech_recognition as sr

from voice_listener.domain.model import AudioCapture, Language
from voice_listener.domain.ports import Transcriber


class GoogleTranscriber(Transcriber):
    def __init__(self) -> None:
        self._recognizer = sr.Recognizer()

    def transcribe(self, audio: AudioCapture, language: Language) -> str | None:
        try:
            return self._recognizer.recognize_google(
                audio.raw,
                language=language.code,
            ).lower()
        except sr.UnknownValueError:
            print("Could not understand audio.")
            return None
        except sr.RequestError as e:
            print(f"Speech recognition service error: {e}")
            return None
