# bursts

> A city skyline at night with rockets launching and exploding in front of your live OS logo.

A silhouette skyline at the bottom. Rockets launch upward, explode into particle bursts, and illuminate the scene (and the centered live logo). The logo sits in the sky and gets lit by nearby explosions.

## Visual elements

- **Skyline**. Buildings with individually lit windows.
- **Rockets**. Launching projectiles that arc toward the center.
- **Explosions**. Expanding particle bursts with lens flares and starbursts.
- **Background stars**. Twinkling stars that react to explosions.
- **Live logo**. The OS name + kernel rendered in the upper sky area.

## Dynamic / live behavior

- **Live logo**. Uses real-time `get_system_info()` for the logo text and kernel.
- **System load reactions**. Higher load increases rocket launch rate and explosion frequency / intensity. The show gets busier when your machine is working hard.
- **Host personality**. `host_bias` creates slight variations per computer.
- **Accent colors**. Explosions and lighting blend with your system accent.

## Configuration (registry)

Under `HKEY_CURRENT_USER\Software\local76\bursts`:

- `LaunchRate`: how many rockets can be in the air and the launch frequency.
- `SkylineStyle`: 0 = normal buildings, 1 = empty sky (no buildings).

Global settings apply as usual.

## Notes

- Very cinematic — explosions create nice light blooms on the skyline windows and logo.
- The skyline windows react to nearby explosions.
- Excellent with the dynamic logo sitting in the night sky.

Part of the [screensavers](https://github.com/local76/screensavers) collection. See the root README for installation.
