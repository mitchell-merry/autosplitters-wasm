# autosplitters-wasm

my collection of wasm autosplitters

This used to be at https://github.com/mitchell-merry/livesplit-zdoom, but I moved it here after it became more than just
zdoom games.

# MEGA DISCLAIMER

I am bad at Rust

This is a hard language

## Project Structure

I have some work done in a couple engines...

- [zdoom](./zdoom) - for ZDoom (and gzdoom/lzdoom) - used for a few games here
- [idtech](./idtech) - for the IdTech engine (used for DOOM: The Dark Ages)

There's helpers in [helpers](./helpers), and everything else is a game.

## Installation

### In LiveSplit

The recommend approach is to use the Edit Splits viewer, browse by game name, and then activate from inside LiveSplit.
However, if that is not possible (e.g. the splitter is not yet available there), follow these steps:

1. Download the autosplitter from the [Releases](https://github.com/mitchell-merry/autosplitters-wasm/releases) page
   (the `.wasm` file).
2. Open LiveSplit, and right click -> "Edit Splits". If a splitter is activated here, deactivate it, and click "OK" to
   save your changes.
3. Right click LiveSplit -> "Edit Layout"
4. Add "Control" -> "Auto-Splitting Runtime"
5. Edit the settings of that component and browse to the downloaded `.wasm` file.
6. Change settings as needed and apply the layout.

In most cases, you'll need to also right click LiveSplit and set "Compare Against" to "Game Time".

## Compilation

This auto splitter is written in Rust. In order to compile it, you need to
install the Rust compiler: [Install Rust](https://www.rust-lang.org/tools/install).

Afterwards install the WebAssembly target:

```sh
rustup target add wasm32-wasip1 --toolchain nightly
```

The autosplitters can now be compiled:

```sh
cargo b --release
```

The autosplitters are then available at:

```
target/wasm32-wasip1/release/<name>.wasm
```

Make sure to look into the [API documentation](https://livesplit.org/asr/asr/) for the `asr` crate.

## Development

> I have a brain dump for the Cuphead splitter here, if you prefer video format: https://youtu.be/ly9r0Hd2CnY (links in
> description)

You can use the [debugger](https://github.com/LiveSplit/asr-debugger) while
developing the auto splitter to more easily see the log messages, statistics,
dump memory, step through the code and more.

`cargo build` will build all the autosplitters. Specify `-p <package>` to only compile
a specific autosplitter.

Builds are released using a manually-triggered GitHub workflow.
