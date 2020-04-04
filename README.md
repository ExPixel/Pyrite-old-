Pyrite
===

[![Build Status](https://github.com/ExPixel/Pyrite/workflows/Tests/badge.svg)](https://github.com/ExPixel/Pyrite/actions?query=workflow%3ATests)

GBA emulator. Unlike in the original pyrite actually rendering to the screen and playing audio is abstracted away from the GBA itself
so it can be run in a headless mode.

To use the emulator:

```sh
cargo run -- <ROM>
```

Building
---
On **Windows** [LLVM]() is required in in order to generate the bindings to ImGui (this will be removed soon).
On **Linux** the ALSA development files are required. These are provided as part of the `libasound2-dev`
package on Debian and Ubuntu distributions and `alsa-lib-devel` on Fedora.

Screenshots
---

**Pokemon Emerald**  
![Pokemon Emerald Screenshot](https://raw.githubusercontent.com/ExPixel/Pyrite/master/misc/screenshots/pokemon-emerald.png)

**Pokemon Leaf Green**  
![Pokemon Leaf Green Screenshot](https://raw.githubusercontent.com/ExPixel/Pyrite/master/misc/screenshots/pokemon-leaf-green.png)

**Final Fantasy 6 Advance**  
![Final Fantasy 6 Advance](https://raw.githubusercontent.com/ExPixel/Pyrite/master/misc/screenshots/final-fantasy-6-advance.png)
