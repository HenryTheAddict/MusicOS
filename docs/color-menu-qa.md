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
