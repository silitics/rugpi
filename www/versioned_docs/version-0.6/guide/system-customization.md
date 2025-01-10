---
sidebar_position: 1
---

# System Customization

Generally, the root filesystem of an image is defined by a *layer*.
The layer to use for an image is specified by the `layer` directive of the image.
Each layer is defined by a file `<layer name>.toml` in the `layers` directory.
This file specifies the *parent layer*, the recipes to enable, and their parameters.
Note that the parent layer can be an official release of Raspberry Pi OS, e.g., `core/raspios-bookworm`, or any other layer.
For an example, check out the [`customized` layer](https://github.com/silitics/rugpi-template/blob/main/layers/customized.toml) of the template.

At the top-level of the layer configuration, the recipes to enable are specified as a list:

```toml title="<layer name>.toml"
recipes = [...]
```
To exclude specific recipes, use the `exclude` directive:

```toml
exclude = [...]
```

## Repositories

*Repositories* provide additional recipes and layers.
The builtin `core` repository is always implicitly available.
Additional repositories can be included in `rugpi-bakery.toml`.
For instance, the quick-start template includes the `rugpi-extra` repository with:

```toml
[repositories]
rugpi-extra = { git = "https://github.com/silitics/rugpi-extra.git", branch = "v0.6" }
```

The recipes and layers provided by a repository can then be used by prefixing their name with the name given to the repository.
Note that this name is the key in the `repositories` section and can be freely chosen.
Repositories can also be stored in local directories.
In this case, they are included with `path` instead of `git`.
Note that the path must be relative to and contained in the root directory of the Rugpi Bakery project (that is the directory containing the `rugpi-bakery.toml` configuration file).

When using Git repositories, additionally `rev`, `branch`, and `tag` properties are supported to specify the Git revision, branch, or tag to use.
Among other things, this enables semantic versioning of recipes and layers.

## Recipes

A recipe describes modifications to be done to the system.
Rugpi Bakery comes with a set of [core recipes](https://github.com/silitics/rugpi/tree/main/repositories/core/recipes) which you can use.
Additional guides and recipes for reoccurring problems are also described in the [Rugpi Cookbook](../cookbook).

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

They are exposed to steps (see below) in environment variables of the form `RECIPE_PARAM_<PARAM_NAME>`. So, in case of our example, `RECIPE_PARAM_PARAMETER_NAME` and `RECIPE_PARAM_OTHER_PARAMETER`.

### Steps

A step is defined by a file in the `steps` directory of the recipe.
The filename must start with an integer followed by a `-` and the _step kind_.
The integer indicates the position of the step in the recipe, e.g., `00` to `99`.

There are three kinds of steps.

```plain title="XX-packages"
a
list
of
Debian
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

### Environment Variables

When running steps, the following environment variables are set.

- `RUGPI_ROOT_DIR`: The root directory of the system being build.
- `RUGPI_PROJECT_DIR`: The root directory of the Rugpi Bakery project.
- `RUGPI_ARCH`: The architecture of the build (`arm64` or `armhf`).
- `RECIPE_DIR`: The directory of the recipe which is applied.
- `RECIPE_STEP_PATH`: The path of the step being executed.

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