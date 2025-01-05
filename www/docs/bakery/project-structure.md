---
sidebar_position: 1
---

# Project Structure

A Rugix Bakery project generally contains the following files and directories:

- `run-bakery`: Shell script for running Rugix Bakery pinned to a major version.
- `rugix-bakery.toml`: Global project configuration file.
- `layers`: Directory containing project-specific layer configurations.
- `recipes`: Directory containing project-specific recipes.
- `tests`: Directory containing integration tests for system images.

We also refer to the directory containing all these files and directories as the _project directory_.

Note that you must always run `run-bakery` inside the project directory.
