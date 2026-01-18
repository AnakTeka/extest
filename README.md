# Extest - X11 XTEST to uinput Redirector

Extest is an LD_PRELOAD library that intercepts X11 XTEST input injection calls and redirects keyboard/button events through uinput, allowing tools like [keyd](https://github.com/rvaiya/keyd) to intercept and remap them.

## Use Case

When using [Deskflow](https://github.com/deskflow/deskflow) (software KVM) to control a Linux machine from another computer, the input is injected via X11's XTEST extension. Since XTEST operates above the kernel input layer, key remapping daemons like keyd cannot intercept this input.

This library solves that by:
- **Keyboard events** → Redirected through uinput (keyd can intercept)
- **Mouse buttons** → Redirected through uinput (keyd can intercept)
- **Mouse motion** → Passed through to real XTEST (required for X11 cursor movement)

## Building

```sh
# Install Rust if not already installed
# https://www.rust-lang.org/learn/get-started

# Build 64-bit library
cargo build --release --target x86_64-unknown-linux-gnu
```

The library will be at `target/x86_64-unknown-linux-gnu/release/libextest.so`.

## Usage

### With Deskflow

```sh
LD_PRELOAD=/path/to/libextest.so deskflow-core client <server-name>
```

### Configure keyd

Add the extest device to `/etc/keyd/default.conf`:

```
[ids]
*
1234:5678
-feed:beef
```

Then restart keyd:

```sh
sudo systemctl restart keyd
```

### Verify it works

```sh
keyd -m
```

You should see `extest fake device` in the device list and keyboard events coming through it.

## How it Works

The library uses `LD_PRELOAD` to intercept calls to:
- `XTestFakeKeyEvent` - Redirected to uinput
- `XTestFakeButtonEvent` - Redirected to uinput
- `XTestFakeMotionEvent` - Passed through to real XTEST
- `XTestFakeRelativeMotionEvent` - Passed through to real XTEST

Mouse motion must use real XTEST because uinput absolute/relative positioning doesn't move the X11 cursor.

## Credits

Based on [Supreeeme/extest](https://github.com/Supreeeme/extest), originally developed for Steam Controller on Wayland. Modified to:
- Work on X11 (removed Wayland dependency for screen size detection)
- Pass mouse motion through to real XTEST for proper cursor movement
