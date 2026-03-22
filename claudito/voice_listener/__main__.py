#!/usr/bin/env python3
import os

from voice_listener.domain.model import Language, WakeWord
from voice_listener.domain.service import VoiceListenerService
from voice_listener.infrastructure.audio import MicrophoneCapturer
from voice_listener.infrastructure.claude_handler import ClaudeCodeHandler
from voice_listener.infrastructure.speaker import GTTSSpeaker
from voice_listener.infrastructure.speech import GoogleTranscriber


def main(language_code: str = os.environ.get("VOICE_LANGUAGE", "es-ES")) -> None:
    print("Voice Order Listener")
    print("====================")
    print("Press Ctrl+C to quit.\n")

    wake_word = WakeWord(value=os.environ.get("WAKE_WORD", "claudito"))
    language = Language(code=language_code)
    transcriber = GoogleTranscriber()
    order_handler = ClaudeCodeHandler()
    speaker = GTTSSpeaker()

    with MicrophoneCapturer() as capturer:
        capturer.calibrate(duration=1.0)
        service = VoiceListenerService(
            capturer=capturer,
            transcriber=transcriber,
            order_handler=order_handler,
            speaker=speaker,
            wake_word=wake_word,
            language=language,
        )
        try:
            service.run()
        except KeyboardInterrupt:
            print("\nStopping.")


if __name__ == "__main__":
    main()
