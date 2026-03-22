import numpy as np
import noisereduce as nr
import speech_recognition as sr

from voice_listener.domain.model import AudioCapture, Language
from voice_listener.domain.ports import Transcriber


def _denoise(audio: sr.AudioData) -> sr.AudioData:
    samples = np.frombuffer(audio.get_raw_data(), dtype=np.int16).astype(np.float32)
    reduced = nr.reduce_noise(y=samples, sr=audio.sample_rate, stationary=True, prop_decrease=0.9)
    return sr.AudioData(reduced.astype(np.int16).tobytes(), audio.sample_rate, audio.sample_width)


class GoogleTranscriber(Transcriber):
    def __init__(self) -> None:
        self._recognizer = sr.Recognizer()

    def transcribe(self, audio: AudioCapture, language: Language) -> str | None:
        try:
            return self._recognizer.recognize_google(
                _denoise(audio.raw),
                language=language.code,
            ).lower()
        except sr.UnknownValueError:
            print("Could not understand audio.")
            return None
        except sr.RequestError as e:
            print(f"Speech recognition service error: {e}")
            return None
