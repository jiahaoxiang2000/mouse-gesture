# Apple Devices on Arch Linux

Apple's _Magic Mouse_ and _Magic Keyboard_ are well known for their **excellent design** and **functionality**. Although Apple does not officially support these devices on _Arch Linux_, you can still use them with the help of community-supported packages and a few configuration steps.

---

## Magic Mouse

> This guide is for the _Magic Mouse 2 USB-C 2024_.

To use the 2024 USB-C version of the Magic Mouse 2, you can rely on [this community repository](https://github.com/mr-cal/Linux-Magic-Trackpad-2-USB-C-Driver). It provides the necessary drivers and instructions to build kernel modules like `hid_magicmouse` for your device.

Once the modules are installed, you can connect the Magic Mouse via Bluetooth. It will then work as a standard mouse on your system.

### Configuration for module loading

```bash
filename:       /lib/modules/6.11.0-25-generic/updates/dkms/hid-magicmouse.ko.zst
license:        GPL
description:    Apple "Magic" Wireless Mouse driver
parm:           emulate_3button:Emulate a middle button (bool)
parm:           emulate_scroll_wheel:Emulate a scroll wheel (bool)
parm:           scroll_speed:Scroll speed, value from 0 (slow) to 63 (fast)
parm:           scroll_acceleration:Accelerate sequential scroll events (bool)
parm:           report_undeciphered:Report undeciphered multi-touch state field using a MSC_RAW event (bool)

```

```text /etc/modprobe.d/hid-magicmouse.conf
options hid-magicmouse emulate_3button=0 emulate_scroll_wheel=1 scroll_speed=32 scroll_acceleration=0 report_undeciphered=0
```

it can be recognized by the `evtest /dev/input/event26`, but not recognized by the `libinput` .

> The detail event information can be checked by on [multi-touch-protocol](https://www.kernel.org/doc/Documentation/input/multi-touch-protocol.txt) on linux kernel documentation.

```bash
  Event type 0 (EV_SYN)
  Event type 1 (EV_KEY)
    Event code 256 (BTN_0)
    Event code 272 (BTN_LEFT)
    Event code 273 (BTN_RIGHT)
  Event type 2 (EV_REL)
    Event code 0 (REL_X)
    Event code 1 (REL_Y)
    Event code 6 (REL_HWHEEL)
    Event code 8 (REL_WHEEL)
    Event code 11 (REL_WHEEL_HI_RES)
    Event code 12 (REL_HWHEEL_HI_RES)
 Event type 3 (EV_ABS)
    Event code 47 (ABS_MT_SLOT)
      Value     10
      Min        0
      Max       15
    Event code 48 (ABS_MT_TOUCH_MAJOR)
      Value      0
      Min        0
      Max     1020
      Fuzz       4
    Event code 49 (ABS_MT_TOUCH_MINOR)
      Value      0
      Min        0
      Max     1020
      Fuzz       4
    Event code 52 (ABS_MT_ORIENTATION)
      Value      0
      Min      -31
      Max       32
      Fuzz       1
    Event code 53 (ABS_MT_POSITION_X)
      Value      0
      Min    -1100
      Max     1258
      Fuzz       4
      Resolution      26
    Event code 54 (ABS_MT_POSITION_Y)
      Value      0
      Min    -1589
      Max     2047
      Fuzz       4
      Resolution      70
    Event code 57 (ABS_MT_TRACKING_ID)
      Value      0
      Min        0
      Max    65535
```

---

## Magic Keyboard

> This guide is for the _Magic Keyboard 2021 (no Touch ID)_.

The Magic Keyboard (2021, no Touch ID) is supported out of the box by the Linux kernel firmware. You do **not** need to build or install any extra kernel modules.

However, to ensure everything works smoothly, you should add a module load rule. Create the following file:

- `/etc/udev/rules.d/91-apple-keyboard.rules`

This will make sure the `hid` device is loaded, which in turn enables the `hid_apple` module.

You can further configure the Magic Keyboard by editing:

- `/etc/modprobe.d/hid-apple.conf`

This file lets you customize keyboard behavior to your liking.

---
