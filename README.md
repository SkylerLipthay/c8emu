# c8emu

A cross-platform [CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) emulator written in Rust.

## Example usage

```sh
$ cargo run --release -- roms/BLINKY
# Use --help for usage options
```

![](www/BLINKY.gif?raw=true)

The CHIP-8 input is emulated using the keyboard as follows:

```
CHIP-8      Keyboard

1 2 3 C     1 2 3 4
4 5 6 D     Q W E R
7 8 9 E     A S D F
A 0 B F     Z X C V
```

## Features

* Standard hardware emulation (4096 byte memory, 64x32 screen resolution, 16 word stack)
* Controllable frames per second and instructions per frame
* Game Boy inspired color palette
* No audio support
