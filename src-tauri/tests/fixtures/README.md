`seek.flac` is a generated two-second 440 Hz sine wave, with no third-party recording:

```sh
ffmpeg -v error -f lavfi -i 'sine=frequency=440:duration=2:sample_rate=22050' -c:a flac -y seek.flac
```

It checks desktop decoder seeking and rewinding without requiring an audio device.
