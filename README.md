# Syntixi-VM - A Lua VM (fork) in Rust

This is a side project for a side project. I don't suggest using this, but if
you want to...

To view progress, it's best advised to view the Git history. It contains
detailed, per-file information on progress made.

## Installation

```sh
cargo build  # Did you expect something else?..
```

## Testing

```sh
echo 'print("Hello World!")' | luac -
cargo run luac.out
```
