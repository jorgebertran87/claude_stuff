#!/usr/bin/env python3
import pyaudio

pa = pyaudio.PyAudio()
print(f"Device count: {pa.get_device_count()}\n")

for i in range(pa.get_device_count()):
    info = pa.get_device_info_by_index(i)
    if info["maxInputChannels"] > 0:
        print(f"[{i}] INPUT  {info['name']}")
    else:
        print(f"[{i}] output {info['name']}")

try:
    default = pa.get_default_input_device_info()
    print(f"\nDefault input: [{default['index']}] {default['name']}")
except OSError:
    print("\nNo default input device found.")

pa.terminate()
