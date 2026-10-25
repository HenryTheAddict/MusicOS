# Restore colorful Light artwork and connected dropdown motion

25 October 2026. Desktop-only repair following the user's clarification that the
colorful Light interface was wanted, and that the photo-brightness complaint was
about photos. The prior neutralizing change exceeded that scope.

## Changes

Light artwork uses the earlier curated-palette formula again: at 15% surface tint,
its original surface colors are present, subject to the existing 4.8:1 primary
text contrast safeguard. Saved appearance values and palette caches are unchanged.
Dark, original blue and explicit custom formulas are unchanged. Standard-brightness
photo decoding, sRGB display copies and original artwork are preserved separately.

Read more/Read less and Reset theme use compact, solid, soft filled Sketch buttons,
with 140 ms surface/press feedback, a maximum 2% inward press, and colored keyboard
focus. Reset uses the supplied `sync` icon, styled at a small fixed size. Attribution
remains a text link. Artist expansion stays in place and keeps Read less at the
top; album descriptions keep their existing dialog. Sidebar More uses the same
filled family without enlarging its row, label or hit area.

Section, Settings-choice, overflow and context menus share one finite presentation
hook. A separate absolutely positioned surface grows/retracts with its rounded
border and exterior shadow; the full-size menu contents are clipped independently.
The text and Popper collision bounds never scale or reflow during the animation.
This fixes the missing shadow caused by clipping the entire Library panel and the
missing close caused by instant-close CSS overrides. Superseded CSS animations
were removed. Full motion is 140 ms; reversals use the displayed frame and remaining
distance. Section headers remain attached through exit, then restore their lower
corners. The chevron has one down/closed, up/open transform owner.

Closing panels are immediately inert and hidden from accessibility, then unmount
after exit. Only the latest owner restores focus. Retained closed Radix layers
ignore outside dismissal and their own trigger's interaction, avoiding the open/
immediate-dismiss race when Library is reopened while retracting. Context exits
keep their pointer coordinates. Reduced Motion changes states immediately. Resize,
scroll, unmount and completion cancel temporary work; there is no idle frame loop.

## Verification

Palette goldens cover the earlier red, green, blue, muted and monochrome colors.
Extreme colors retain chroma while meeting text contrast. Tests no longer encode
the unwanted nearly-gray Light artwork surfaces. Source palettes remain unchanged.
UI tests supply the browser media-query API missing from jsdom for supplied icons.

Browser checks sample opening and closing frames, pause/reverse the real menu
animations, verify visible exterior shadows, stable labels/choice positions,
unscaled text, immediate inertness, cleanup, keyboard focus and Reduced Motion.
Disclosure and Reset workflows retain unrelated saved fields and persistence.
A 48-combination matrix covers 720×480 / 1280×800, all four scales, Light/Dark
artwork and blue, and extreme Light/Dark custom colors, checking menu bounds and
filled-button contrast. Existing launch, context-exit, collection-color and standard
photo-brightness regressions are included. Final results and delivery follow below.

## Delivery

Production UI and optimized native builds pass. All 154 UI unit tests pass.
Fifteen distinct focused browser cases pass across the regression and final-focus
runs, including the 48-combination matrix. Final frame checks also confirm that
navigating to Settings does not replace Library's outgoing labels mid-collapse.
There are no database, preference schema or playback-protocol changes.

Source: isolated desktop `ee090ed`, primary mirror `360fb8c`. Signed executable
SHA-256: `72857fa7198f4ae0cd4f5684fc78b4b63612a1dc1b42af4d51deaddc82077e1b`.
Strict signature verification passes. All eight embedded runtime/model files are
byte-identical to the prior installed app. OS implementation files were untouched.

Installation followed a fresh native paused check and a clean quit. The latest
private profile copy and verified SQLite backup were made immediately before the
swap. The previous app, release manifest, protected hashes and checkpoint pointer
are retained under `releases/desktop-color-menus-2026-10-25/`.

Native verification uses the user's latest Light artwork appearance at 125% scale,
150% saturation and 15% tint. The colorful artwork surfaces are visibly restored;
those saved settings were not edited. Reset theme now has its own compact filled
button with the supplied icon. Its destructive-to-theme action was tested in the
fixture, rather than resetting the real profile. The actual Library section menu
opens with a visible exterior shadow, closes with Escape, restores focus and
reopens. Appearance choices close and restore focus. Artist Read more / Read less
expand and collapse in place, retain attribution, and use the filled control.
More keeps its existing four destinations and closes normally. Native pointer
input verified the section anchor where an accessibility click was ignored.

The complete saved playback snapshot still matches, including song, queue,
position (2.0102084 seconds), volume, shuffle and repeat. Playback remains paused
at 0:02. All six collection tables, favorite flags, interface preferences and
integration configuration match the immediate pre-install checkpoint: 508 songs
and 34 favorites remain. The app is left on Appearance with menus closed.

Native screenshots, frame-check screenshots and passing logs are under
`out/qa/os-foundation/color-menu/` and `out/persistent-qa/`. Native checks verify
appearance and interaction, not a guaranteed display frame rate or physical HDR
luminance. The standard-brightness photo pipeline remains unchanged.
