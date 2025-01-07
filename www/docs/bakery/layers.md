---
sidebar_position: 2
---

# Layers

Each layer is defined by a file `<layer name>.toml` in the `layers` directory. This file has the following structure:

```typescript
type Layer = {
    parent?: string;
    root?: boolean,
    url?: string;
    recipes?: string[];
    exclude?: string[];
    parameters?: {
        [recipe: string]: {
            [parameter: string]: string | boolean | number
        };
    }
}
```

A layer may be based upon a *parent* layer, that might be fetched from a URL, or be a *root* layer.

Rugpi comes with pre-defined base layers that can be used by setting `parent` to

- `core/debian-bookworm` for Debian Bookworm,
- `core/alpine-3-20` for Alpine Linux 3.20,
- `core/raspios-bookworm` for Raspberry Pi OS (Bookworm), and
- `core/raspios-bullseye` for Raspberry Pi OS (Bullseye).

Instead of using a pre-defined base layer, the recipes `core/alpine-bootstrap` and `core/debian-bootstrap` can be used for bootstrapping specific versions of Alpine Linux and Debian with `root` set to `true`.
For semi-reproducible[^1] images based on Debian snapshots, you can also set the snapshot parameter of `core/debian-bootstrap`.
For instance:

```toml
root = true

recipes = [
    "core/debian-bootstrap",
]

[parameters."core/debian-bootstrap"]
suite = "bookworm"
snapshot = "20240501T024440Z"
```

[^1]: Builds are not fully reproducible yet.

The `url` property can be set to an URL of an image, e.g., of Raspberry Pi OS, or a `.tar` file containing a root filesystem.
The URL can be either an HTTP URL or a file URL (starting with `file://`).
In the latter case, the URL is resolved relative to the project directory.
Using a `.tar` file as a basis enables layers based on root filesystems built with other tools, e.g., OpenEmbedded or Buildroot.
