import threading
from abc import ABC, abstractmethod
from collections.abc import Callable

from voice_listener.domain.model import AudioCapture, Language


class AudioCapturer(ABC):
    @abstractmethod
    def calibrate(self, duration: float = 1.0) -> None: ...

    @abstractmethod
    def capture(
        self,
        timeout: float | None,
        phrase_time_limit: float | None,
        pause_threshold: float | None = None,
    ) -> AudioCapture | None: ...

    @abstractmethod
    def mute(self) -> None: ...

    @abstractmethod
    def unmute(self) -> None: ...


class Transcriber(ABC):
    @abstractmethod
    def transcribe(self, audio: AudioCapture, language: Language) -> str | None: ...


class OrderHandler(ABC):
    @abstractmethod
    def handle(self, order: str) -> str: ...


class AudioSpeaker(ABC):
    @abstractmethod
    def speak(self, text: str, language: Language, on_playback_start: Callable[[], None] | None = None) -> None: ...

    @abstractmethod
    def stop(self) -> None: ...

    @abstractmethod
    def beep(self) -> None: ...

    @abstractmethod
    def play_melody(self, stop_event: threading.Event) -> None: ...
