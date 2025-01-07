---
sidebar_position: 5
---

# Introduction

Rugpi consists of two tools, *Rugpi Bakery* and *Rugpi Ctrl*.
Rugpi Bakery is for building customized system *images* while Rugpi Ctrl is for managing a system's state at runtime and installing over-the-air system updates.
An *image* contains a complete Linux root filesystem, a Linux kernel, and some additional files required for booting.
An image can be directly flashed onto some storage medium, e.g., an SD card, an NVMe drive, or a USB stick, from which a compatible system may then boot directly.
Rugpi Bakery allows you to specify multiple images while sharing customizations between them.
This is useful, for instance, if you want to build different variants of an image for different devices.
Every image is based on a *layer*.
Most importantly, a layer provides the root filesystem and kernel for an image.
Furthermore, it may also contain additional configuration files, e.g., for booting.
Each layer consists of *recipes* that correspond to customization that should be made.
For instance, a recipe may install a web server and configure it to serve a static site.
