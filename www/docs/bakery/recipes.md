---
sidebar_position: 3
---

# Recipes

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