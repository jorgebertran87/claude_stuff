from abc import ABC, abstractmethod

from voice_listener.domain.model import AudioCapture, Language


class AudioCapturer(ABC):
    @abstractmethod
    def calibrate(self, duration: float = 1.0) -> None: ...

    @abstractmethod
    def capture(self, timeout: float | None, phrase_time_limit: float | None) -> AudioCapture | None: ...


class Transcriber(ABC):
    @abstractmethod
    def transcribe(self, audio: AudioCapture, language: Language) -> str | None: ...


class OrderHandler(ABC):
    @abstractmethod
    def handle(self, order: str) -> str: ...
