import re
from dataclasses import dataclass
from difflib import SequenceMatcher
from typing import Any

_FUZZY_THRESHOLD = 0.80  # minimum similarity ratio to accept a match


@dataclass(frozen=True)
class WakeWord:
    value: str

    def __post_init__(self) -> None:
        if not self.value.strip():
            raise ValueError("WakeWord cannot be empty.")

    def matches(self, text: str) -> bool:
        wake = self.value.lower()
        words = re.findall(r"\w+", text.lower())
        return any(
            word == wake or SequenceMatcher(None, wake, word).ratio() >= _FUZZY_THRESHOLD
            for word in words
        )

    def extract_order(self, text: str) -> str | None:
        """Return the text that follows the wake word in the utterance, if any."""
        wake = self.value.lower()
        words = re.findall(r"\w+", text.lower())
        for i, word in enumerate(words):
            if word == wake or SequenceMatcher(None, wake, word).ratio() >= _FUZZY_THRESHOLD:
                rest = " ".join(words[i + 1:]).strip()
                return rest if rest else None
        return None


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
