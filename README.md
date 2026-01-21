# Extest - X11 XTEST to uinput Redirector

> Fork of [Supreeeme/extest](https://github.com/Supreeeme/extest), originally created for Steam Controller on Wayland. Modified to work on X11.

**Redirects X11 XTEST input to uinput, enabling keyd/KMonad and other evdev-based tools to intercept Deskflow input**

Extest is an LD_PRELOAD library that intercepts X11 XTEST input injection calls and redirects keyboard/button events through uinput, making them visible to kernel-level input tools like [keyd](https://github.com/rvaiya/keyd), [KMonad](https://github.com/kmonad/kmonad), and [interception-tools](https://gitlab.com/interception/linux/tools).

## The Problem

When using [Deskflow](https://github.com/deskflow/deskflow), [Input Leap](https://github.com/input-leap/input-leap), Synergy, or Barrier (software KVM solutions) to control a Linux machine from another computer, evdev-based tools like keyd cannot see the input. This is because these applications inject input via X11's XTEST extension, which operates above the kernel input layer.

This library solves that by:
- **Keyboard events** → Redirected through uinput (visible to evdev tools)
- **Mouse buttons** → Redirected through uinput (visible to evdev tools)
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
# With GUI
LD_PRELOAD=/path/to/libextest.so deskflow

# Or directly with deskflow-core
LD_PRELOAD=/path/to/libextest.so deskflow-core client <server-name>
```

### Configure keyd (example)

By default, keyd ignores virtual input devices. You need to explicitly add the extest device ID (`e17e:5700`) to `/etc/keyd/default.conf`:

```
[ids]
*
e17e:5700
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

## Changes from Original

- Replaced Wayland screen detection with X11 (`XDisplayWidth`/`XDisplayHeight`)
- Mouse motion passes through to real XTEST (required for X11 cursor movement)
- Keyboard and button events redirected to uinput

## Keywords

Deskflow keyd uinput, Deskflow KMonad, Input Leap keyd, Synergy keyd, Barrier keyd, key remapping Deskflow, software KVM key remapping Linux, evdev Deskflow
