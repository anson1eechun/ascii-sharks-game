# Changelog

## 0.1.0 - Unreleased

First version, built from [ascii-sharks-term](https://github.com/anson1eechun/ascii-sharks-term) 0.3.1.

### Added
- A title screen over the live sea: pick your shark (shown swimming in a preview window) and the number of rivals. A pause menu (Esc or `p`) to resume, restart, read the controls, return to the title or quit. `-p` skips the title.
- Menus drawn with depth: lit, raised slabs with soft shadows, a recessed preview window with shafts of light, an embossed logo in the sharks' ASCII skin, and drifting caustics. Arrow keys, `WASD`, `hjkl`, Enter, and the mouse all work.
- You play the smallest shark in the sea and grow by eating fish, until you are the biggest. Growth slows as you get bigger.
- Steering with the arrow keys, `WASD`, `hjkl` or the mouse, and a dash on space or a click.
- A status line with your size, fish eaten, rank by length and dash; a "you" tag at the start, a gold outline on your shark, and a banner when you become the biggest.
- `-k` picks your shark, `-o` and `-n` the others.
- The difficulty is tuned with an autopilot, and a test keeps it there.

### Removed (compared with ascii-sharks-term)
- The screensaver extras: clock and message, `--still`, `--print`, `--screensaver`, rain, feeding, click ripples, fish fleeing the pointer and names on hover.
