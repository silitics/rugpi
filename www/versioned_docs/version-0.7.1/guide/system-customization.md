---
sidebar_position: 2
---

# System Customization

Generally, the root filesystem of an image is defined by a *layer*.
The layer to use for an image is specified by the `layer` directive of the image.
Each layer is defined by a file `<layer name>.toml` in the `layers` directory having the following structure:

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

A layer may be based upon a *parent* layer, may be fetched from a URL, or be a *root* layer.

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

## Recipes

At the top-level of the layer configuration, the recipes to enable are specified as a list:

```toml title="<layer name>.toml"
recipes = [...]
```
To exclude specific recipes, use the `exclude` directive:

```toml
exclude = [...]
```


A recipe describes modifications to be done to the system.
Rugpi Bakery comes with a set of [core recipes](https://github.com/silitics/rugpi/tree/main/bakery/repositories/core/recipes) which you can use.

Each recipe has its own directory with a `recipe.toml` configuration file.
Recipes may have a `description`, a `priority`, and `dependencies`.
Checkout the builtin recipes for examples.
Recipes are always applied in the order of their priority.
In particular, `dependencies` are not guaranteed to be applied before the recipes that depend on them.

### Parameters

Recipes can have _parameters_.
Parameters are defined in the `parameters` section of `recipe.toml`:

```toml
[parameters]
parameter_name = { default = "a default value" }
other_parameter = {}  # Required parameter without a default value.
```

They are exposed to steps (see bellow) in environment variables of the form `RECIPE_PARAM_<PARAM_NAME>`. So, in case of our example, `RECIPE_PARAM_PARAMETER_NAME` and `RECIPE_PARAM_OTHER_PARAMETER`.

Parameters are set as part of the layer configuration.

### Steps

A step is defined by a file in the `steps` directory of the recipe.
The filename must start with an integer followed by a `-` and the _step kind_.
The integer indicates the position of the step in the recipe, e.g., `00` to `99`.

There are three kinds of steps.

```plain title="XX-packages"
a
list
of
packages
```

```bash title="XX-run.*"
#!/usr/bin/env bash

echo "This runs on the host system."
```

```bash title="XXX-install.*"
#!/usr/bin/env bash

echo "This runs via chroot in the system being built."
```

Note that `run` and `install` are not limited to Bash scripts.

You can also use `XX-packages.apk` and and `XX-packages.apt` to install different packages depending on whether the recipe is used to build an Alpine Linux or Debian based system.

### Environment Variables

When running steps, the following environment variables are set.

- `RUGPI_ARCH`: The architecture of the build (`arm64` or `armhf`).
- `RUGPI_ROOT_DIR`: The directory of the root filesystem.
- `RUGPI_PROJECT_DIR`: The directory of the Rugpi Bakery project.
- `RUGPI_BUNDLE_DIR`: The directory of the Rugpi Bundle being built.
- `RECIPE_DIR`: The directory of the recipe which is applied.
- `RECIPE_STEP_PATH`: The path of the step being executed.

## Repositories

*Repositories* provide additional recipes and layers.
The builtin `core` repository is always implicitly available.
Additional repositories can be included in `rugpi-bakery.toml`.
For instance, the quick-start templates may include the `rugpi-extra` repository with:

```toml
[repositories]
rugpi-extra = { git = "https://github.com/silitics/rugpi-extra.git", branch = "v0.7" }
```

The recipes and layers provided by a repository can then be used by prefixing their name with the name given to the repository.
Note that this name is the key in the `repositories` section and can be freely chosen.
Repositories can also be stored in local directories.
In this case, they are included with `path` instead of `git`.
Note that the path must be relative to and contained in the root directory of the Rugpi Bakery project (that is the directory containing the `rugpi-bakery.toml` configuration file).

When using Git repositories, additionally `rev`, `branch`, and `tag` properties are supported to specify the Git revision, branch, or tag to use.
Among other things, this enables semantic versioning of recipes and layers.

## Layer Caching

By default, layers are cached and only rebuilt if the recipes or their configuration file changes.
Note that sometimes recipes may use files from the project directory.
In this case, the recipes should include the following line

```shell
echo "<path-relative-to-project-dir>" >> "${LAYER_REBUILD_IF_CHANGED}"
```

where `<path-relative-to-project-dir>` is the path of the used files relative to the project directory.
This will make the caching mechanism aware of those files and will lead to the layer being rebuilt if any of the files changes.

A common use case would be to load a `.env` file with secret environment variables from the project directory:

```shell
# Rebuild the layer if the environment changes.
echo ".env" >> "${LAYER_REBUILD_IF_CHANGED}"
# Include the environment, if it exists.
if [ -f "$RUGPI_PROJECT_DIR/.env" ]; then
    . "$RUGPI_PROJECT_DIR/.env"
fi
```

