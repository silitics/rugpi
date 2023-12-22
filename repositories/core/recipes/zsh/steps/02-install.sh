#!/bin/bash

set -euo pipefail

if [ "${RECIPE_PARAM_MAKE_DEFAULT}" = "true" ]; then
    chsh -s /bin/zsh
fi
