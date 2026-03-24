"""
Integration tests — real components wired together, no audio hardware.

Pipelines under test
  1. TTS pipeline       : langdetect + gTTS + pydub  (requires network)
  2. Audio processing   : numpy + noisereduce         (no network, no hardware)
  3. Service threading  : real threads + state machine (no network, no hardware)
"""

import array
import math
import threading

import numpy as np
import pytest
import speech_recognition as sr
from pydub import AudioSegment

from voice_listener.domain.model import AudioCapture, Language, WakeWord
from voice_listener.domain.ports import AudioCapturer, AudioSpeaker, OrderHandler, Transcriber
from voice_listener.domain.service import VoiceListenerService
from claude_agent_sdk import ResultMessage
from voice_listener.infrastructure.audio import MicrophoneCapturer
from voice_listener.infrastructure.claude_handler import ClaudeCodeHandler
from voice_listener.infrastructure.speaker import _alexa_spotify_parts, _tts_segment
from voice_listener.infrastructure.speech import _denoise


# ---------------------------------------------------------------------------
# Shared fakes (hardware boundaries only)
# ---------------------------------------------------------------------------

class FakeCapturer(AudioCapturer):
    def __init__(self, captures):
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
    def __init__(self, texts):
        self._texts = iter(texts)

    def transcribe(self, audio, language):
        return next(self._texts, None)


class FakeOrderHandler(OrderHandler):
    def __init__(self, response):
        self._response = response

    def handle(self, order):
        return self._response


class FakeSpeaker(AudioSpeaker):
    def __init__(self):
        self.spoken = []
        self.stopped = False

    def speak(self, text, language, on_playback_start=None):
        self.spoken.append(text)
        if on_playback_start:
            on_playback_start()

    def stop(self): self.stopped = True
    def beep(self): pass
    def play_melody(self, stop_event): stop_event.wait()
    def get_echo_reference(self): return None


class BlockingFakeSpeaker(FakeSpeaker):
    def __init__(self):
        super().__init__()
        self._playing = threading.Event()

    def speak(self, text, language, on_playback_start=None):
        self.spoken.append(text)
        if on_playback_start:
            on_playback_start()
        self._playing.wait()

    def stop(self):
        self.stopped = True
        self._playing.set()


# ---------------------------------------------------------------------------
# Audio helpers
# ---------------------------------------------------------------------------

def sine_wave_audio(frequency=440, duration_ms=500, sample_rate=16000) -> sr.AudioData:
    """Generate a pure sine wave as sr.AudioData (no hardware required)."""
    n = int(sample_rate * duration_ms / 1000)
    samples = array.array("h", [
        int(32767 * math.sin(2 * math.pi * frequency * i / sample_rate))
        for i in range(n)
    ])
    return sr.AudioData(samples.tobytes(), sample_rate, sample_width=2)


def audio():
    return AudioCapture(raw=object())


# ===========================================================================
# 1. TTS pipeline  (langdetect + gTTS + pydub)
# ===========================================================================

class TestTtsPipeline:
    def test_tts_segment_returns_non_empty_audio_for_valid_language(self):
        segment = _tts_segment("hola mundo", "es")
        assert isinstance(segment, AudioSegment)
        assert len(segment) > 0

    def test_tts_segment_falls_back_to_english_for_unsupported_language_code(self):
        # "xx" is not a gTTS-supported code; the pipeline should recover
        segment = _tts_segment("hello world", "xx")
        assert isinstance(segment, AudioSegment)
        assert len(segment) > 0

    def test_alexa_spotify_command_produces_longer_audio_than_title_alone(self):
        """Concatenating three TTS segments (frame + title + frame) is longer than the title alone."""
        text = 'Alexa, pon "Shape of You" en Spotify'
        parts = _alexa_spotify_parts(text)
        assert parts is not None

        combined = None
        for chunk, lang in parts:
            if chunk.strip():
                seg = _tts_segment(chunk, lang)
                combined = seg if combined is None else combined + seg

        title_only = _tts_segment("Shape of You", "en")
        assert len(combined) > len(title_only)

    def test_alexa_spotify_frame_and_title_use_different_audio(self):
        """The command frame (Spanish) and the song title produce different audio bytes."""
        parts = _alexa_spotify_parts('Alexa, pon "Shape of You" en Spotify')
        frame_seg = _tts_segment(parts[0][0], parts[0][1])   # Spanish frame
        title_seg = _tts_segment(parts[1][0], parts[1][1])   # Detected title lang
        assert frame_seg.raw_data != title_seg.raw_data


# ===========================================================================
# 2. Audio processing pipeline  (numpy + noisereduce)
# ===========================================================================

class TestAudioProcessingPipeline:
    def test_denoise_preserves_audio_shape_and_type(self):
        original = sine_wave_audio()
        result = _denoise(original)
        assert isinstance(result, sr.AudioData)
        assert result.sample_rate == original.sample_rate
        assert result.sample_width == original.sample_width
        assert len(result.get_raw_data()) == len(original.get_raw_data())

    def test_denoise_modifies_the_signal(self):
        original = sine_wave_audio()
        result = _denoise(original)
        assert result.get_raw_data() != original.get_raw_data()

    def test_cancel_echo_returns_valid_audio_data(self):
        speech_audio = sine_wave_audio(frequency=300, duration_ms=500)
        echo_audio   = sine_wave_audio(frequency=880, duration_ms=500)

        capturer = MicrophoneCapturer()
        capturer.set_echo_reference((
            echo_audio.get_raw_data(),
            echo_audio.sample_rate,
            echo_audio.sample_width,
        ))

        result = capturer._cancel_echo(speech_audio)

        assert isinstance(result, sr.AudioData)
        assert result.sample_rate == speech_audio.sample_rate
        assert result.sample_width == speech_audio.sample_width

    def test_cancel_echo_attenuates_the_reference_frequency(self):
        """Energy at the echo frequency should drop after cancellation."""
        speech_audio = sine_wave_audio(frequency=300, duration_ms=500)
        echo_audio   = sine_wave_audio(frequency=880, duration_ms=500)

        # Mix: speech + echo (simulates mic picking up both)
        mixed_samples = (
            np.frombuffer(speech_audio.get_raw_data(), dtype=np.int16).astype(np.float32)
            + np.frombuffer(echo_audio.get_raw_data(), dtype=np.int16).astype(np.float32)
        ).clip(-32768, 32767).astype(np.int16)
        mixed_audio = sr.AudioData(mixed_samples.tobytes(), speech_audio.sample_rate, 2)

        capturer = MicrophoneCapturer()
        capturer.set_echo_reference((
            echo_audio.get_raw_data(),
            echo_audio.sample_rate,
            echo_audio.sample_width,
        ))

        cleaned = capturer._cancel_echo(mixed_audio)

        original_energy = np.mean(mixed_samples.astype(np.float32) ** 2)
        cleaned_samples = np.frombuffer(cleaned.get_raw_data(), dtype=np.int16).astype(np.float32)
        cleaned_energy  = np.mean(cleaned_samples ** 2)

        assert cleaned_energy < original_energy


# ===========================================================================
# 3. Service threading lifecycle  (real threads + state transitions)
# ===========================================================================

class TestServiceThreadingLifecycle:
    def _make_service(self, captures, transcriptions, speaker, response="ok"):
        return VoiceListenerService(
            capturer=FakeCapturer(captures),
            transcriber=FakeTranscriber(transcriptions),
            order_handler=FakeOrderHandler(response),
            speaker=speaker,
            wake_word=WakeWord("claudito"),
            language=Language("es-ES"),
        )

    def test_melody_thread_is_joined_before_speak_returns(self):
        speaker = FakeSpeaker()
        service = self._make_service([], [], speaker)

        _, stop_event, melody_thread = service._handle_with_melody("test order")
        service._speak_interruptible("respuesta", stop_event, melody_thread)

        assert not melody_thread.is_alive()

    def test_echo_reference_is_always_cleared_after_speech(self):
        speaker = FakeSpeaker()
        capturer = FakeCapturer([])
        service = VoiceListenerService(
            capturer, FakeTranscriber([]), FakeOrderHandler("ok"),
            speaker, WakeWord("claudito"), Language("es-ES"),
        )
        stop_event    = threading.Event()
        melody_thread = threading.Thread(target=speaker.play_melody, args=(stop_event,), daemon=True)
        melody_thread.start()

        service._speak_interruptible("respuesta", stop_event, melody_thread)

        assert capturer.echo_reference is None

    def test_question_response_sets_waiting_for_answer_and_skips_wake_word(self):
        """After a response ending with '?', the next order requires no wake word."""
        speaker  = FakeSpeaker()
        capturer = FakeCapturer([
            audio(),   # wake word utterance
            audio(),   # first order
            audio(),   # follow-up order (no wake word needed)
        ])
        transcriber = FakeTranscriber([
            "claudito",           # wake word
            "qué hora es",        # first order
            "en madrid",          # follow-up without wake word
        ])
        responses = iter(["¿En qué ciudad?", "Son las 12 en Madrid."])
        handler   = type("H", (OrderHandler,), {"handle": lambda self, o: next(responses)})()

        spoken_orders = []
        original_handle = handler.handle

        service = VoiceListenerService(
            capturer, transcriber, handler,
            speaker, WakeWord("claudito"), Language("es-ES"),
        )

        # Simulate two iterations of run() manually
        inline = service.wait_for_wake_word()
        first_order = inline or service.listen_for_order()
        response, stop, mt = service._handle_with_melody(first_order)
        interrupted = service._speak_interruptible(response, stop, mt)
        waiting_for_answer = not interrupted and "?" in response.rstrip()

        assert waiting_for_answer is True

        # Second turn: no wake word needed because waiting_for_answer is True
        second_order = service.listen_for_order()
        assert second_order == "en madrid"

    def test_interrupt_during_speech_sets_waiting_for_answer(self):
        """When speech is interrupted by wake word, the caller should set waiting_for_answer=True."""
        speaker  = BlockingFakeSpeaker()
        capturer = FakeCapturer([audio()])
        transcriber = FakeTranscriber(["claudito"])

        service = VoiceListenerService(
            capturer, transcriber, FakeOrderHandler("ok"),
            speaker, WakeWord("claudito"), Language("es-ES"),
        )
        stop_event    = threading.Event()
        melody_thread = threading.Thread(target=speaker.play_melody, args=(stop_event,), daemon=True)
        melody_thread.start()

        interrupted = service._speak_interruptible("respuesta larga", stop_event, melody_thread)

        # run() sets waiting_for_answer=True on interrupt so user can speak without wake word
        waiting_for_answer = interrupted
        assert waiting_for_answer is True


# ===========================================================================
# 4. ClaudeCodeHandler token logging  (real ResultMessage + real file I/O)
# ===========================================================================

def _fake_result_message(result="ok", input_tokens=18, output_tokens=735,
                         cache_read=38335, cache_creation=2610, cost=0.029965):
    return ResultMessage(
        subtype="success",
        duration_ms=1000,
        duration_api_ms=900,
        is_error=False,
        num_turns=1,
        session_id="test-session",
        stop_reason="end_turn",
        total_cost_usd=cost,
        usage={
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "cache_read_input_tokens": cache_read,
            "cache_creation_input_tokens": cache_creation,
        },
        result=result,
        structured_output=None,
    )


def _fake_query(message):
    async def _q(prompt, options):
        yield message
    return _q


class TestClaudeCodeHandlerTokenLogging:
    def test_token_log_is_written_after_handle(self, tmp_path, monkeypatch):
        log_file = tmp_path / ".orders_tokens"
        monkeypatch.setattr("voice_listener.infrastructure.claude_handler._TOKENS_LOG_FILE", log_file)
        monkeypatch.setattr("voice_listener.infrastructure.claude_handler.query",
                            _fake_query(_fake_result_message("respuesta")))

        ClaudeCodeHandler().handle("pon música")

        assert log_file.exists()

    def test_token_log_contains_order_and_all_token_fields(self, tmp_path, monkeypatch):
        log_file = tmp_path / ".orders_tokens"
        monkeypatch.setattr("voice_listener.infrastructure.claude_handler._TOKENS_LOG_FILE", log_file)
        monkeypatch.setattr("voice_listener.infrastructure.claude_handler.query",
                            _fake_query(_fake_result_message(cost=0.029965)))

        ClaudeCodeHandler().handle("mañana lloverá")

        line = log_file.read_text()
        assert "mañana lloverá" in line
        assert "input: 18" in line
        assert "output: 735" in line
        assert "cache_read: 38335" in line
        assert "cache_creation: 2610" in line
        assert "total: 41698" in line
        assert "0.029965" in line

    def test_token_log_appends_one_line_per_call(self, tmp_path, monkeypatch):
        log_file = tmp_path / ".orders_tokens"
        monkeypatch.setattr("voice_listener.infrastructure.claude_handler._TOKENS_LOG_FILE", log_file)

        for order in ("primera orden", "segunda orden"):
            monkeypatch.setattr("voice_listener.infrastructure.claude_handler.query",
                                _fake_query(_fake_result_message(order)))
            ClaudeCodeHandler().handle(order)

        lines = [l for l in log_file.read_text().splitlines() if l]
        assert len(lines) == 2
        assert "primera orden" in lines[0]
        assert "segunda orden" in lines[1]

    def test_handle_returns_result_from_message(self, tmp_path, monkeypatch):
        log_file = tmp_path / ".orders_tokens"
        monkeypatch.setattr("voice_listener.infrastructure.claude_handler._TOKENS_LOG_FILE", log_file)
        monkeypatch.setattr("voice_listener.infrastructure.claude_handler.query",
                            _fake_query(_fake_result_message("respuesta esperada")))

        result = ClaudeCodeHandler().handle("una orden")

        assert result == "respuesta esperada"
