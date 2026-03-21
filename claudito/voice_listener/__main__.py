#!/usr/bin/env python3
from voice_listener.domain.model import Language, WakeWord
from voice_listener.domain.service import VoiceListenerService
from voice_listener.infrastructure.audio import MicrophoneCapturer
from voice_listener.infrastructure.speech import GoogleTranscriber


def main(language_code: str = "es-ES") -> None:
    print("Voice Order Listener")
    print("====================")
    print("Press Ctrl+C to quit.\n")

    wake_word = WakeWord(value="claudito")
    language = Language(code=language_code)
    transcriber = GoogleTranscriber()

    with MicrophoneCapturer() as capturer:
        capturer.calibrate(duration=1.0)
        service = VoiceListenerService(
            capturer=capturer,
            transcriber=transcriber,
            wake_word=wake_word,
            language=language,
        )
        try:
            service.run()
        except KeyboardInterrupt:
            print("\nStopping.")


if __name__ == "__main__":
    main()
