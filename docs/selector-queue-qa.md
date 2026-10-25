# Section selector collapse and Up next — 2026-10-25

The Library / Settings selector now has one rounded surface spanning the existing
header and its choices. Its height and exterior shadow retract with the clipped
choices over 140 ms; the label, hit area and text geometry remain stationary.
Closing uses an even ease rather than losing most of its visible height at the
start. Section navigation commits after exit, preventing the header/page from
changing underneath the closing menu. Reopening or opening another menu cancels
a pending selection. Reduced Motion commits immediately. Other action menus keep
their immediate action behavior, keyboard access and focus restoration.

Up next in Compact and Wide uses a complete 30 px upcoming row, with a small
Up next label above the song title. The current song's header fits in 28 px;
its progress and the stationary transport row stay readable at 120 px height.
The Settings preview reuses the actual queue text component and row metrics,
with a thumbnail and remove affordance. It does not request metadata or control
playback. A Settings-specific small-text margin that clipped the preview has
been eliminated. Empty queues show a single fitting message at small sizes;
Standard retains its larger list. Duplicate entry keys, retained queue DOM,
scroll and queue controls are unchanged. No preferences, database or playback
protocol changes are introduced. Theme formulas and SDR photography are untouched.

## Verification

Production UI compilation and all 154 UI unit tests pass. Six focused browser
cases pass, covering intermediate opening/closing frames, a continuously joined
surface and shadow, fixed label position, unscaled choices, rapid reversals,
selection cancellation, Escape, focus restoration, replacement menus, Reduced
Motion and zero completed menu animation work. The existing Settings/More
regression passes as well.

Queue preview tests cover all three presets, populated/empty queues and duplicate
songs. Bounds checks confirm that complete upcoming text fits inside both its
row and the available list. The native-player fixture checks the Wide minimum,
stationary controls and retained queue entries across rapid switches. Preview
interaction issues no playback commands.

The matrix passes 32 window/scale/theme combinations, selecting all three presets
in each (96 preset states): 720×480 and 1280×800; 80%, 100%, 125%, 150%; Light/Dark
artwork, Light blue and extreme Dark custom colors. The 150% cases use empty
queues. No horizontal Settings overflow or clipped first-row text is introduced.

Build delivery and native verification follow below. Browser artifacts and logs
are retained in `out/persistent-qa/` and `out/qa/os-foundation/selector-queue/`.


## Signed build and native verification

Implementation is isolated desktop commit `b12b4e2`, mirrored as UI-only commit
`f70feaf`. Optimized native packaging and strict signature verification pass.
Signed executable SHA-256:
`1080e5114ecf77ebc28bda2557ed5ddad1b4323a98470c4b480f660fd6979bc2`.
All eight embedded runtime/model files match the previous signed app. OS sources
and artwork files were untouched.

Installation followed a fresh native paused check on Stateside at 2:14. The
latest profile and a verified SQLite backup were made immediately before loading.
The prior app, protected hashes and checkpoint pointer are retained under
`releases/desktop-selector-queue-2026-10-25/`.

Native screenshots verify the single rounded Library surface and exterior
shadow, its return to the header, section selection and the Wide Up next preview.
The actual Wide mini-player was checked at 320×120: current cover/title/progress,
a complete Up next / Romeo row with thumbnail and remove action, and stationary
transport controls. Playback remains paused at 2:14. Restoring the original
preset/view and final profile checks is pending a second Mac unlock; the lock
screen interrupted this last step.
