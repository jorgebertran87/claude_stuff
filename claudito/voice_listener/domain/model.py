from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class WakeWord:
    value: str

    def __post_init__(self) -> None:
        if not self.value.strip():
            raise ValueError("WakeWord cannot be empty.")

    def matches(self, text: str) -> bool:
        return self.value.lower() in text.lower()


@dataclass(frozen=True)
class Language:
    code: str

    def __post_init__(self) -> None:
        if not self.code.strip():
            raise ValueError("Language code cannot be empty.")


@dataclass(frozen=True)
class AudioCapture:
    """Opaque wrapper around a captured audio frame. Domain code never inspects raw."""
    raw: Any
