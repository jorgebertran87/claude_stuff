import speech_recognition as sr

from voice_listener.domain.model import AudioCapture
from voice_listener.domain.ports import AudioCapturer


class MicrophoneCapturer(AudioCapturer):
    def __init__(self) -> None:
        self._recognizer = sr.Recognizer()
        self._mic: sr.Microphone | None = None
        self._source = None

    def __enter__(self) -> "MicrophoneCapturer":
        self._mic = sr.Microphone()
        self._source = self._mic.__enter__()
        return self

    def __exit__(self, *args: object) -> None:
        if self._mic:
            self._mic.__exit__(*args)

    def calibrate(self, duration: float = 1.0) -> None:
        print("Calibrating for ambient noise...")
        self._recognizer.adjust_for_ambient_noise(self._source, duration=duration)

    def capture(self, timeout: float | None, phrase_time_limit: float) -> AudioCapture | None:
        try:
            raw = self._recognizer.listen(
                self._source,
                timeout=timeout,
                phrase_time_limit=phrase_time_limit,
            )
            return AudioCapture(raw=raw)
        except sr.WaitTimeoutError:
            return None
