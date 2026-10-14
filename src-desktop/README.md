# MusicOS C++ desktop

Run from the repository root with `./run.sh`. Build and run the focused checks with `make -C src-desktop all test`.

The toolbar selector switches between Library, Devices, and Settings. `Cmd+,` opens Settings; Escape returns to Library or goes back from a detail page. Settings changes are saved in `preferences.txt` under SDL's `MusicOS / MusicOSDesktop` application preferences directory. Coverflow Colors defaults off, the name defaults to Henry, and interface size defaults to 100%.

Albums and artists open detail pages without playing audio. Play, Shuffle, track controls, overflow menus, and the toolbar transport share stable demo track sequences. All audio remains synthesized; track durations and collection statistics describe the demo records. No real library indexing, file playback, or playlist editing is provided by this iteration.

Coverflow Colors follows the viewed album or artist while browsing detail pages and the selected playing track elsewhere. Missing artwork falls back to blue. Artist portraits fall back to the first album cover. Palette extraction and soft backgrounds are cached; artwork changes crossfade once.

For repeatable visual checks, use the existing screenshot mode, for example:

```sh
./src-desktop/musicos-desktop --points 720x480 --view album --dump-png out/album.png
./src-desktop/musicos-desktop --points 1280x850 --view artist --adaptive --dump-png out/artist.png
./src-desktop/musicos-desktop --points 1280x850 --view settings --ui-scale 150 --dump-png out/settings.png
```

Screenshot mode starts from default preferences and does not save them. `--view` accepts `album`, `artist`, or `settings`; `--ui-scale` accepts 80, 100, 125, or 150. The settings scale is independent of Retina density. Larger settings require a correspondingly larger minimum window so persistent controls remain available.
