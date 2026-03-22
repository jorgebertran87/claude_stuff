import speech_recognition as sr

from voice_listener.domain.model import AudioCapture
from voice_listener.domain.ports import AudioCapturer


class MicrophoneCapturer(AudioCapturer):
    def __init__(self, pause_threshold: float = 2.0) -> None:
        self._recognizer = sr.Recognizer()
        self._recognizer.pause_threshold = pause_threshold
        self._mic: sr.Microphone | None = None
        self._source = None

    def __enter__(self) -> "MicrophoneCapturer":
        self._mic = sr.Microphone()
        self._source = self._mic.__enter__()
        return self

    def __exit__(self, *args: object) -> None:
        if self._mic:
            self._mic.__exit__(*args)

    def mute(self) -> None:
        if self._source and self._source.stream:
            self._source.stream.pyaudio_stream.stop_stream()

    def unmute(self) -> None:
        if self._source and self._source.stream:
            self._source.stream.pyaudio_stream.start_stream()

    def calibrate(self, duration: float = 1.0) -> None:
        print("Calibrating for ambient noise...")
        self._recognizer.adjust_for_ambient_noise(self._source, duration=duration)
        # Lock the energy threshold so it doesn't climb during speech and
        # misinterpret natural pauses between words as silence.
        self._recognizer.dynamic_energy_threshold = False
        if self._recognizer.energy_threshold > 17000:
            self.calibrate(duration)
            return

    def capture(
        self,
        timeout: float | None,
        phrase_time_limit: float | None,
        pause_threshold: float | None = None,
    ) -> AudioCapture | None:
        original = self._recognizer.pause_threshold
        if pause_threshold is not None:
            self._recognizer.pause_threshold = pause_threshold
        try:
            raw = self._recognizer.listen(
                self._source,
                timeout=timeout,
                phrase_time_limit=phrase_time_limit,
            )
            return AudioCapture(raw=raw)
        except sr.WaitTimeoutError:
            return None
        finally:
            self._recognizer.pause_threshold = original
