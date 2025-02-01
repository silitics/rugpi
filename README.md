<p align="center">
    <img src="./www/static/img/logo.svg" width="12%" alt="Rugpi Logo">
</p>
<h1 align="center">
    Rugix
</h1>
<h4 align="center">
    Rugix is a suite of open-source tools to build <em>reliable</em> embedded Linux devices
    <br>with <em>efficient and secure</em> over-the-air update capabilities.
</h4>
<p align="center">
  <a href="https://github.com/silitics/rugpi/releases"><img alt="Rugix Version Badge" src="https://img.shields.io/github/v/tag/silitics/rugpi?label=version"></a>
  <a href="https://github.com/silitics/rugpi/actions"><img alt="Pipeline Status Badge" src="https://img.shields.io/github/actions/workflow/status/silitics/rugpi/check-and-lint.yml"></a>
</p>

<p align="center">
    <strong>We are in the process of renaming this project from “Rugpi” to “Rugix.”</strong>
</p>

💡 **TL;DR**: Rugix (formerly Rugpi) enables you to **build commercial-grade, customized variants of popular Linux distributions** for your devices. It boasts three core features designed to work seamlessly together: (1) A modern, flexible workflow to build customized system images, (2) **robust over-the-air system updates** with rollback support for the entire system, and (3) **managed state** that is preserved across reboots and updates.

## ✨ Features

- 🌈 Supports **Debian, Alpine Linux, and Raspberry Pi OS**.
- 🖥️ Supports **any EFI-compatible system and all models of Raspberry Pi**.
- ➡️ Supports **streaming of updates** without intermediate storage.
- 🔒 Enables [cryptographically **signed and verified updates**](https://rugpi.io/docs/advanced/signed-updates).
- 🙌 Supports root filesystems built with third-party tools.
- 🔌 Integrates well with [existing device management solutions](https://rugpi.io/docs/advanced/device-management).
- 🧩 Provides interfaces to built your own update workflow upon.
- 💾 Provides built-in state management inspired by Docker.

Checkout the [documentation](https://oss.silitics.com/rugpi/) for details and build your first image in less than an hour. 🚀

## 🤔 Why Rugix?

While many excellent tools are already available for building images, updating systems, and managing state, integrating them into a robust setup can be challenging. Rugix strives to simplify this process by bundling all essential functionalities into a cohesive package, allowing you to focus on what matters most to you and your users. We believe that building innovative devices shouldn't be as complicated as it often is today. Although Rugix may *currently* offer less flexibility and configurability than standalone solutions, it excels at delivering a robust base for your device right out of the box.

## ⚖️ Licensing

This project is licensed under either [MIT](https://github.com/silitics/rugpi/blob/main/LICENSE-MIT) or [Apache 2.0](https://github.com/silitics/rugpi/blob/main/LICENSE-APACHE) at your opinion.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache 2.0 license, shall be dual licensed as above, without any additional terms or conditions.

---

Made with ❤️ for OSS by [Silitics](https://www.silitics.com)
