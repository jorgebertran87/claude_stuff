#!/usr/bin/env python3
"""
Voice order listener — wakes on "Claudito", then captures the order.
Requires: pip install SpeechRecognition pyaudio
"""

import speech_recognition as sr

WAKE_WORD = "claudito"


def recognize(recognizer: sr.Recognizer, audio: sr.AudioData, language: str) -> str | None:
    try:
        return recognizer.recognize_google(audio, language=language).lower()
    except sr.UnknownValueError:
        print("Could not understand audio.")
        return None
    except sr.RequestError as e:
        print(f"Speech recognition service error: {e}")
        return None


def wait_for_wake_word(
    recognizer: sr.Recognizer,
    source: sr.Microphone,
    language: str,
) -> None:
    print(f'Waiting for wake word "{WAKE_WORD}"...')
    while True:
        try:
            audio = recognizer.listen(source, timeout=None, phrase_time_limit=4)
        except sr.WaitTimeoutError:
            continue

        text = recognize(recognizer, audio, language)
        print(text)
        if text and WAKE_WORD in text:
            print("Wake word detected!")
            return


def listen_for_order(
    recognizer: sr.Recognizer,
    source: sr.Microphone,
    language: str,
    phrase_time_limit: int = 10,
) -> str | None:
    print("Listening for your order...")
    try:
        audio = recognizer.listen(source, timeout=5, phrase_time_limit=phrase_time_limit)
    except sr.WaitTimeoutError:
        print("No order detected.")
        return None

    return recognize(recognizer, audio, language)


def main(language: str = "es-ES") -> None:
    print("Voice Order Listener")
    print("====================")
    print("Press Ctrl+C to quit.\n")

    recognizer = sr.Recognizer()

    with sr.Microphone() as source:
        print("Calibrating for ambient noise...")
        recognizer.adjust_for_ambient_noise(source, duration=1)

        while True:
            try:
                wait_for_wake_word(recognizer, source, language)
                order = listen_for_order(recognizer, source, language)
                if order:
                    print(f"Order received: {order!r}")
                    # TODO: dispatch order to your handler here
            except KeyboardInterrupt:
                print("\nStopping.")
                break


if __name__ == "__main__":
    main()
