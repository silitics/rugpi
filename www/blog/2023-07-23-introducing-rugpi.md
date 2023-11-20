---
slug: introducing-rugpi
title: Introducing Rugpi
authors: koehlma
tags: [introduction, rugpi]
---

We are thrilled to introduce _Rugpi_, the first open-source platform that empowers you to create innovative products based on Raspberry Pi. 🎉
At its core, Rugpi is designed to streamline the process of building commercial-grade, customized variants of [Raspberry Pi OS](https://www.raspberrypi.com/software/) for your projects.
Developed out of the need for a reliable platform for our customers, Rugpi boasts three core features:

#### (1) Modern Build Workflow with Rugpi Bakery

Instead of a manual golden-image workflow, Rugpi comes with a Docker-based toolchain, coined _Rugpi Bakery_, for building customized images based on a set of _recipes_.
Recipes allow you to cherry-pick and install only the software and configurations you need.
You can build images locally or via a CI system such as GitHub Actions or GitLab CI/CD.
This eliminates all the hassels and chores that come with a manual golden-image workflow, and streamlines development as well as deployment.
Furthermore, recipes can be shared with the community enabling reusability and composability.

#### (2) Robust Over-the-Air Updates

One of the most difficult challenges in maintaining embedded devices is ensuring seamless updates with minimal interruptions and without corrupting any data or leaving the system in a bricked state.
Rugpi tackles this challenge head-on with its robust over-the-air update feature. With rollback support for the entire system, including firmware files, you can update your devices remotely with complete peace of mind.
This allows you to deliver the latest features and enhancements to your product in a snap, without worrying about costly downtimes or potential damages due to incomplete updates.

#### (3) Managed State

Rugpi's managed state feature ensures that the important state of a device is preserved across reboots and updates.
At the same time, it safeguards against accidental state corrupting the system and makes implementing reliable factory resets or state backups a breeze.

### Next Step: Becoming Production-Ready

While the core features are there already, Rugpi is still experimental.
We plan to further fine-tune the design and welcome any feedback or suggestions from the community.
For now, our primary goal is to make Rugpi production-ready by consolidating its design and conducting thorough testing.

Stay tuned for future updates by staring or watching [the project on GitHub](https://github.com/silitics/rugpi)! 📣

### Commercial Support and Applications

Rugpi is and will stay open-source under the permissive MIT and Apache 2.0 licenses.
This is great for hobby projects and also minimizes risks for commercial applications.
Rugpi is backed by my company, [Silitics](https://www.silitics.com), and we offer commercial support as well as development and consulting services.
If you plan to built a commercial product with Rugpi, we are here to ensure your success.

### Technical Details in a Nutshell

For those curious about the technical details, here is a sneak peak.
For OTA updates, we use the recently introduced [`tryboot` feature of Raspberry Pi's bootloader](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html#fail-safe-os-updates-tryboot), enabling a fail-safe A/B update schema.
For state management and controlling OTA updates and rollbacks, we have developed custom software in Rust, ensuring reliable functionality.
Rugpi replaces the init process and uses overlay and bind mounts to set everything up before handing the controls over to Systemd.
While the system partition is mounted read-only at all times, preventing corruption, a writeable overlay and bind mounts are used to selectively persist important state across reboots and updates, and to discard any accidental state.
For further details, read the [user guide](/docs/guide/) and checkout [the source code on GitHub](https://github.com/silitics/rugpi/tree/main).

### Try it Today!

We invite you to try Rugpi.
Checkout [the quick-start guide](/docs/getting-started) to built your first image and share your recipes, questions, projects, ideas, and suggestions by opening [discussions on GitHub](https://github.com/silitics/rugpi/discussions). 🚀
