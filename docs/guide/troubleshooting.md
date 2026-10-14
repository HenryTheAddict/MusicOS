# Troubleshooting

[Guide home](README.md)

| Symptom | Next step |
| --- | --- |
| New music is missing | Reconnect its source, then Rescan folders in Library settings |
| Quality or musical analysis is stale | Rescan audio in Quality & analysis; enable Recheck every available song only when a full refresh is intended |
| One song fails analysis | Inspect that song's failure details, confirm it plays, then retry; other songs can continue |
| Sparklist creation is disabled | Open Sparklist setup, verify consent and model installation, then analyze at least two songs including the selected seed |
| Model installation fails | Retry the exact staged candidate; checksum failures cannot be bypassed |
| Artist portrait is poorly framed | Expand the original image; missing or uncertain faces use original framing, and new framing versions recheck old negative results |
| Depth is absent | Verify model installation, parallax/depth preferences, reduced-motion setting, and compatible graphics; plain artwork is the fallback |
| Artist lookup fails offline | Cached content should remain; retry provider lookup later from metadata settings |
| DJ transition does not occur | Confirm DJmode On and an upcoming playable song; review accepted schedule/diagnostics rather than relying on an estimated BPM |

Technical errors belong in diagnostic details, not collection navigation. Report the packaged executable hash, macOS version, interface scale, window size, steps, and whether playback or analysis was active. Avoid sharing private file paths unnecessarily.

Before loading a new build, pause or stop playback. A rebuild does not update an already-running app process. Windows and Linux behavior remain unverified unless a release record explicitly says otherwise.
