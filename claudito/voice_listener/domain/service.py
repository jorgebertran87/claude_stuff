from voice_listener.domain.model import Language, WakeWord
from voice_listener.domain.ports import AudioCapturer, AudioSpeaker, OrderHandler, Transcriber

_ORDER_TIMEOUT = 10           # seconds to wait for the user to start speaking
_ORDER_RETRIES = 2            # how many times to re-prompt before giving up
_ORDER_PAUSE_THRESHOLD = 4.0  # seconds of silence before the order phrase is considered done


class VoiceListenerService:
    def __init__(
        self,
        capturer: AudioCapturer,
        transcriber: Transcriber,
        order_handler: OrderHandler,
        speaker: AudioSpeaker,
        wake_word: WakeWord,
        language: Language,
    ) -> None:
        self._capturer = capturer
        self._transcriber = transcriber
        self._order_handler = order_handler
        self._speaker = speaker
        self._wake_word = wake_word
        self._language = language

    def wait_for_wake_word(self) -> str | None:
        """Block until the wake word is heard. Returns any inline order found in the same utterance."""
        print(f'Waiting for wake word "{self._wake_word.value}"...')
        while True:
            audio = self._capturer.capture(timeout=None, phrase_time_limit=8)
            if audio is None:
                continue
            text = self._transcriber.transcribe(audio, self._language)
            if text:
                print(f"Heard: {text!r}")
            if text and self._wake_word.matches(text):
                print("Wake word detected!")
                return self._wake_word.extract_order(text)

    def listen_for_order(self) -> str | None:
        """Listen for an order, retrying up to _ORDER_RETRIES times."""
        for attempt in range(_ORDER_RETRIES):
            self._speaker.beep()
            print("Listening for your order... speak now!")
            audio = self._capturer.capture(
                timeout=_ORDER_TIMEOUT,
                phrase_time_limit=None,
                pause_threshold=_ORDER_PAUSE_THRESHOLD,
            )
            if audio is None:
                if attempt < _ORDER_RETRIES - 1:
                    print("I didn't catch that. Please repeat your order.")
                continue
            text = self._transcriber.transcribe(audio, self._language)
            if text:
                return text
        print("No order detected.")
        return None

    def run(self) -> None:
        while True:
            inline_order = self.wait_for_wake_word()
            order = inline_order or self.listen_for_order()
            if order:
                print(f"Order received: {order!r}")
                response = self._order_handler.handle(order)
                print(f"Claudito: {response}")
                self._capturer.mute()
                self._speaker.speak(response, self._language)
                self._capturer.unmute()
