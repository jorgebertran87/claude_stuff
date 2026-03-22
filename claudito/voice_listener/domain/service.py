from voice_listener.domain.model import Language, WakeWord
from voice_listener.domain.ports import AudioCapturer, AudioSpeaker, OrderHandler, Transcriber


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

    def wait_for_wake_word(self) -> None:
        print(f'Waiting for wake word "{self._wake_word.value}"...')
        while True:
            audio = self._capturer.capture(timeout=None, phrase_time_limit=4)
            if audio is None:
                continue
            text = self._transcriber.transcribe(audio, self._language)
            if text:
                print(f"Heard: {text!r}")
            if text and self._wake_word.matches(text):
                print("Wake word detected!")
                return

    def listen_for_order(self) -> str | None:
        print("Listening for your order...")
        audio = self._capturer.capture(timeout=5, phrase_time_limit=None)
        if audio is None:
            print("No order detected.")
            return None
        return self._transcriber.transcribe(audio, self._language)

    def run(self) -> None:
        while True:
            self.wait_for_wake_word()
            order = self.listen_for_order()
            if order:
                print(f"Order received: {order!r}")
                response = self._order_handler.handle(order)
                print(f"Claudito: {response}")
                self._speaker.speak(response, self._language)
