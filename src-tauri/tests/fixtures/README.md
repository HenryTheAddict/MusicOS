`seek.flac` is a generated two-second 440 Hz sine wave, with no third-party recording:

```sh
ffmpeg -v error -f lavfi -i 'sine=frequency=440:duration=2:sample_rate=22050' -c:a flac -y seek.flac
```

It checks desktop decoder seeking and rewinding without requiring an audio device.

`display-p3.png` and `hdr-pq.png` are synthetic native ImageIO swatches, generated
by `make-color-fixtures.m`. They contain a Display P3 orange and three neutral
ITU-R 2100 PQ levels, respectively. They test real color conversion / SDR decoding
without any user photos. The generator's header contains reproduction commands.
