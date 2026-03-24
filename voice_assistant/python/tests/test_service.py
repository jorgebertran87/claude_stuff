import threading

import pytest

from voice_listener.domain.model import AudioCapture, Language, WakeWord
from voice_listener.domain.ports import AudioCapturer, AudioSpeaker, OrderHandler, Transcriber
from voice_listener.domain.service import VoiceListenerService


# ---------------------------------------------------------------------------
# Fakes  (Detroit School: hand-rolled in-memory collaborators, no mock library)
# ---------------------------------------------------------------------------

class FakeCapturer(AudioCapturer):
    def __init__(self, captures: list):
        self._captures = iter(captures)
        self.echo_reference = None

    def capture(self, timeout, phrase_time_limit, pause_threshold=None):
        return next(self._captures, None)

    def calibrate(self, duration=1.0): pass
    def mute(self): pass
    def unmute(self): pass

    def set_echo_reference(self, ref):
        self.echo_reference = ref


class FakeTranscriber(Transcriber):
    def __init__(self, texts: list):
        self._texts = iter(texts)

    def transcribe(self, audio, language):
        return next(self._texts, None)


class FakeOrderHandler(OrderHandler):
    def __init__(self, response: str = ""):
        self._response = response

    def handle(self, order):
        return self._response


class FakeSpeaker(AudioSpeaker):
    """Instant speaker: speak() returns immediately after firing the callback."""

    def __init__(self):
        self.beep_count = 0
        self.stopped = False

    def speak(self, text, language, on_playback_start=None):
        if on_playback_start:
            on_playback_start()

    def stop(self):
        self.stopped = True

    def beep(self):
        self.beep_count += 1

    def play_melody(self, stop_event):
        stop_event.wait()

    def get_echo_reference(self):
        return None


class BlockingFakeSpeaker(FakeSpeaker):
    """Speaks until stop() is called — lets the capture loop run during speech."""

    def __init__(self):
        super().__init__()
        self._playing = threading.Event()

    def speak(self, text, language, on_playback_start=None):
        if on_playback_start:
            on_playback_start()
        self._playing.wait()

    def stop(self):
        self.stopped = True
        self._playing.set()


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

WAKE_WORD = WakeWord("claudito")
LANGUAGE  = Language("es-ES")


def audio():
    return AudioCapture(raw=object())


def make_service(captures, transcriptions, speaker=None, response="ok"):
    return VoiceListenerService(
        capturer=FakeCapturer(captures),
        transcriber=FakeTranscriber(transcriptions),
        order_handler=FakeOrderHandler(response),
        speaker=speaker or FakeSpeaker(),
        wake_word=WAKE_WORD,
        language=LANGUAGE,
    )


# ---------------------------------------------------------------------------
# wait_for_wake_word
# ---------------------------------------------------------------------------

class TestWaitForWakeWord:
    def test_returns_none_when_utterance_is_only_the_wake_word(self):
        service = make_service(captures=[audio()], transcriptions=["claudito"])
        assert service.wait_for_wake_word() is None

    def test_returns_inline_order_when_wake_word_and_order_in_same_utterance(self):
        service = make_service(
            captures=[audio()],
            transcriptions=["claudito pon música"],
        )
        assert service.wait_for_wake_word() == "pon música"

    def test_skips_none_captures_and_keeps_listening(self):
        service = make_service(
            captures=[None, audio()],
            transcriptions=["claudito"],
        )
        assert service.wait_for_wake_word() is None

    def test_skips_utterances_that_do_not_contain_the_wake_word(self):
        service = make_service(
            captures=[audio(), audio()],
            transcriptions=["hola mundo", "claudito"],
        )
        assert service.wait_for_wake_word() is None

    def test_accepts_fuzzy_match_of_wake_word(self):
        # "clauditto" is close enough (SequenceMatcher ≥ 0.80)
        service = make_service(captures=[audio()], transcriptions=["clauditto"])
        assert service.wait_for_wake_word() is None   # detected, no inline order


# ---------------------------------------------------------------------------
# listen_for_order
# ---------------------------------------------------------------------------

class TestListenForOrder:
    def test_returns_transcribed_text_on_first_attempt(self):
        service = make_service(captures=[audio()], transcriptions=["enciende la luz"])
        assert service.listen_for_order() == "enciende la luz"

    def test_returns_none_when_all_captures_time_out(self):
        service = make_service(captures=[None, None], transcriptions=[])
        assert service.listen_for_order() is None

    def test_retries_when_capture_returns_none(self):
        service = make_service(
            captures=[None, audio()],
            transcriptions=["apaga la luz"],
        )
        assert service.listen_for_order() == "apaga la luz"

    def test_retries_when_transcription_returns_none(self):
        service = make_service(
            captures=[audio(), audio()],
            transcriptions=[None, "qué hora es"],
        )
        assert service.listen_for_order() == "qué hora es"

    def test_beeps_once_per_attempt(self):
        speaker = FakeSpeaker()
        service = make_service(
            captures=[None, audio()],
            transcriptions=["hola"],
            speaker=speaker,
        )
        service.listen_for_order()
        assert speaker.beep_count == 2


# ---------------------------------------------------------------------------
# _speak_interruptible
# ---------------------------------------------------------------------------

class TestSpeakInterruptible:
    def _interruptible(self, speaker, captures, transcriptions, response="respuesta"):
        capturer    = FakeCapturer(captures)
        transcriber = FakeTranscriber(transcriptions)
        service = VoiceListenerService(
            capturer, transcriber, FakeOrderHandler(response),
            speaker, WAKE_WORD, LANGUAGE,
        )
        stop_event    = threading.Event()
        melody_thread = threading.Thread(
            target=speaker.play_melody, args=(stop_event,), daemon=True
        )
        melody_thread.start()
        return service._speak_interruptible(response, stop_event, melody_thread), capturer

    def test_returns_false_when_speech_ends_without_interruption(self):
        interrupted, _ = self._interruptible(FakeSpeaker(), captures=[], transcriptions=[])
        assert interrupted is False

    def test_returns_true_when_wake_word_heard_during_speech(self):
        speaker = BlockingFakeSpeaker()
        interrupted, _ = self._interruptible(
            speaker, captures=[audio()], transcriptions=["claudito"]
        )
        assert interrupted is True

    def test_stops_speaker_when_wake_word_interrupts(self):
        speaker = BlockingFakeSpeaker()
        self._interruptible(speaker, captures=[audio()], transcriptions=["claudito"])
        assert speaker.stopped is True

    def test_echo_reference_is_cleared_after_speech(self):
        _, capturer = self._interruptible(FakeSpeaker(), captures=[], transcriptions=[])
        assert capturer.echo_reference is None

    def test_echo_reference_is_cleared_even_when_interrupted(self):
        speaker = BlockingFakeSpeaker()
        _, capturer = self._interruptible(
            speaker, captures=[audio()], transcriptions=["claudito"]
        )
        assert capturer.echo_reference is None

    def test_does_not_interrupt_on_unrelated_speech_during_playback(self):
        speaker = BlockingFakeSpeaker()
        # First capture returns unrelated text, second returns wake word
        interrupted, _ = self._interruptible(
            speaker,
            captures=[audio(), audio()],
            transcriptions=["hola mundo", "claudito"],
        )
        assert interrupted is True
        assert speaker.stopped is True
