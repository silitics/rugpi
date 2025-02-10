---
sidebar_position: 1
---

# Caching

Rugix Bakery will automatically cache layers, rebuilding them only when changes to their configuration or recipes are made. In addition to the builtin layer caching, Rugix Bakery uses a global cache available in recipes via the `RUGIX_CACHE_DIR` environment variable. This cache can be used to cache downloaded files or other data. Note that is a global cache shared among all recipes and layers.

To clean all caches, run:

```shell
./run-bakery cache clean
```

## Layer Caching

By default, layers are cached and only rebuilt if the recipes or their configuration file changes.
Note that sometimes recipes may use files from the project directory or build context.
In this case, the recipes should include the following line

```bash
echo "<path>" >> "${LAYER_REBUILD_IF_CHANGED}"
```

where `<path>` is the path of the used files (usually relative to the project directory).
This will make the caching mechanism aware of those files and will lead to the layer being rebuilt if any of the files changes.

A common use case would be to load a `.env` file with secret environment variables from the project directory:

```shell
# Rebuild the layer if the environment changes.
echo ".env" >> "${LAYER_REBUILD_IF_CHANGED}"
# Include the environment, if it exists.
if [ -f "$RUGIX_PROJECT_DIR/.env" ]; then
    . "$RUGIX_PROJECT_DIR/.env"
fi
```