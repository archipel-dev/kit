# Measuring the reference IDE

The kit reproduces the Islands UI of Android Studio / IntelliJ. Values in
`docs/component-map.md` and `tokens.rs` were measured by running Android Studio and driving
it like a user: pointer, keyboard, screenshots and 60 fps screen recordings. These scripts
are that setup. Nothing from Android Studio is copied into the repository: only measured
facts (sizes, timings, colors) end up in tokens, with a line in the component map.

## Setup (Linux, also in a cloud container)

```bash
apt-get install -y xvfb openbox xdotool imagemagick ffmpeg x11-utils
# Android Studio for Linux from https://developer.android.com/studio, unpacked anywhere.
Xvfb :99 -screen 0 1600x1000x24 -nolisten tcp &
DISPLAY=:99 openbox &
printf -- "-Ddisable.android.first.run=true\n" > studio.vmoptions
DISPLAY=:99 STUDIO_VM_OPTIONS=$PWD/studio.vmoptions android-studio/bin/studio <project> &
```

Open a scratch copy of a project, never the kit's checkout (the IDE writes `.idea/`).
Measure at 100 % scale; values are logical points. Switch themes with
<kbd>Ctrl</kbd>+<kbd>`</kbd> → Theme, and measure every color in Islands Dark and Light.

## Tools

| Script | Use |
|---|---|
| `ui shot NAME [WxH+X+Y]` | Screenshot (optionally cropped) to `$SHOTS/NAME.png`; look at it. |
| `ui zoom NAME WxH+X+Y [PERCENT]` | Cropped, pixel-scaled screenshot for reading geometry. |
| `ui move X Y`, `ui click X Y`, `ui dclick X Y`, `ui drag X1 Y1 X2 Y2`, `ui key K…`, `ui type TEXT` | Drive the IDE like a user. |
| `px X Y …`, `px row Y X0 X1`, `px col X Y0 Y1` | Colors at points, or the color runs along a line: edges, insets, radii, fills. |
| `python3 -I timing.py OUT CROP IN OUT PROBE` | Records at 60 fps while the pointer enters and leaves; prints in which frames the probe region changes. Use it for delays and fades (subtract the 4–8 frame render latency of the virtual display). |

## Method

1. Find the component in the IDE and every state: rest, hover, pressed, focused (Tab),
   selected, disabled, narrow/minimum size, empty and overflowing content, both themes.
2. Measure with `px row`/`px col` against a neighbour of known position (the island edge,
   the window edge). Derive formulas from several instances (e.g. tab widths from six
   tabs) instead of one.
3. Measure motion with `timing.py`: when something appears, how long it waits before it
   disappears, whether it fades.
4. Render the kit's component at 1× and compare side by side, pixel-scaled. Fix until the
   geometry matches; the kit's own icons stay as they are.
5. Put the values in `tokens.rs` (or a theme color), the behavior in a test, and a row in
   `docs/component-map.md` under "Measured behavior".
