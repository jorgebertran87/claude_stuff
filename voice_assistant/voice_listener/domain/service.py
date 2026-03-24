import threading

from voice_listener.domain.model import Language, WakeWord
from voice_listener.domain.ports import AudioCapturer, AudioSpeaker, OrderHandler, Transcriber

_ORDER_TIMEOUT = 10          # seconds to wait for the user to start speaking
_ORDER_RETRIES = 2            # how many times to re-prompt before giving up
_ORDER_PAUSE_THRESHOLD = 2.0  # seconds of silence before the order phrase is considered done


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
        waiting_for_answer = False
        while True:
            if waiting_for_answer:
                order = self.listen_for_order()
            else:
                inline_order = self.wait_for_wake_word()
                order = inline_order or self.listen_for_order()
            if order:
                print(f"Order received: {order!r}")
                response, stop_melody, melody_thread = self._handle_with_melody(order)
                print(f"Claudito: {response}")
                interrupted = self._speak_interruptible(response, stop_melody, melody_thread)
                if interrupted:
                    waiting_for_answer = False
                else:
                    waiting_for_answer = bool(response and "?" in response.rstrip())
            else:
                waiting_for_answer = False

    def _handle_with_melody(self, order: str) -> tuple[str, threading.Event, threading.Thread]:
        """Call the order handler while playing a waiting melody. Returns the melody still running."""
        stop_event = threading.Event()
        melody_thread = threading.Thread(
            target=self._speaker.play_melody, args=(stop_event,), daemon=True
        )
        melody_thread.start()
        response = self._order_handler.handle(order)
        return response, stop_event, melody_thread

    def _speak_interruptible(self, response: str, stop_melody: threading.Event, melody_thread: threading.Thread) -> bool:
        """Speak response while listening for the wake word. Returns True if interrupted."""
        speak_thread = threading.Thread(
            target=self._speaker.speak,
            args=(response, self._language),
            kwargs={"on_playback_start": stop_melody.set},
            daemon=True,
        )
        speak_thread.start()
        melody_thread.join()

        self._capturer.set_echo_reference(self._speaker.get_echo_reference())
        interrupted = False
        try:
            while speak_thread.is_alive():
                audio = self._capturer.capture(timeout=1, phrase_time_limit=2)
                if audio is not None:
                    text = self._transcriber.transcribe(audio, self._language)
                    if text:
                        print(f"Heard during speech: {text!r}")
                    if text and self._wake_word.matches(text):
                        print("Wake word detected — interrupting speech.")
                        self._speaker.stop()
                        self.listen_for_order()
                        interrupted = True
                        break
        finally:
            self._capturer.set_echo_reference(None)

        speak_thread.join()
        return interrupted
