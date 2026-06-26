# Figma Target

Source file: `Music-01`, node `20:2`.

The Figma page contains three 1280x960 frames:

- `20:10` / `Frame 2`: boot setup screen.
- `20:3` / `Frame 1`: hello/setup instructions.
- `20:20` / `test`: hardware test-complete screen.

The RG35XX 640x480 mode uses exact half-scale coordinates and font sizes from
the 1280x960 Figma frames.

Key colors:

- Black: `#000000`
- White: `#ffffff`
- MusicOS blue: `#0059ff`
- Build label white: `#ffffff80`

Key setup geometry at 1280x960:

- Get Started button: `x=440 y=286 w=400 h=170 r=86`
- White card: `x=440 y=504 w=400 h=170 r=20`
- Build label: `x=3 y=900`, `RasterGothicTwentyEightCnM Bold`, size `64`
