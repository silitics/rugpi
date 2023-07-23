---
sidebar_position: 1
---

# System Customization

The main configuration file of Rugpi Bakery is `rugpi-bakery.toml`.
It specifies which recipes to include and their parameters.

At the top-level, the recipes to include are specified as a list:

```toml title="rugpi-bakery.toml"
recipes = [...]
```

Recipes marked as *default* are automatically included.
This includes builtin base recipes and all the recipes in the local `recipes` directory (unless `default` is set to `false` for a recipe).
To exclude specific default recipes, use the `exclude` directive:

```toml
exclude = [...]
```

## Recipes

A recipe describes modifications to the system.
Rugpi Bakery comes with a set of [builtin recipes](https://github.com/silitics/rugpi/tree/main/recipes) which you can use.

Each recipe has its own directory with a `recipe.toml` configuration file.
Recipes may have a `description`, a `priority`, and `dependencies`.
They can also be marked as default.
Checkout the builtin recipes for examples.

Recipes are always applied in the order of their priority.
In particular, `dependencies` are not guaranteed to be applied before the recipes that depend on them. 
You have to use priorities to control their order explicitly.

### Parameters

Recipes can have _parameters_.
Parameters are define in the `parameters` section of `recipe.toml`:

```toml
[parameters]
parameter_name = { default = "a default value" }
other_parameter = {}  # Required parameter without default value.
```

They are exposed to steps (see bellow) in environment variables of the form `RECIPE_PARAM_<PARAM_NAME>`. So, in case of our example, `RECIPE_PARAM_PARAMETER_NAME` and `RECIPE_PARAM_OTHER_PARAMETER`.

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

- `RUGPI_ROOT_DIR`: The root directory of the system.
- `RECIPE_DIR`: The path of the recipe.
- `RECIPE_STEP_PATH`: The path of the step being executed.
