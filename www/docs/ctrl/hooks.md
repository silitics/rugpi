---
sidebar_position: 4
---

# Hooks

_Hooks_ provide a powerful mechanism to inject custom behavior at various stages of Rugix Ctrl's operation.

Hooks are scripts that are executed at specific points in the execution of an operation. For instance, you can run custom scripts after an update is installed (but before the system is rebooted) or before it is committed. Hooks are organized based on the type of the operation and the point in time, referred to as _stage_, when they run. In addition, each hook has a _rank_, specifying the order in which hooks run. You can use hooks to customize and extend various parts of Rugix Ctrl based on your needs and requirements.

Hooks are placed in `/etc/rugix/hooks`. Each operation gets its own directory, for instance, `/etc/rugix/hooks/bootstrap` contains [bootstrapping hooks](#bootstrapping-hooks) and `/etc/rugix/hooks/system-commit` contains [system commit hooks](#system-update-hooks). Each directory gets a subdirectory for each stage of the respective operation. For instance, `system-commit` has a `prepare` stage. The hooks of this stage will run before performing the commit. To add a hook to the respective stage, a file with the name `<rank>-<name>` is placed in the stage directory. Here, `<rank>` is an integer and hooks with a lower rank run earlier than those with a higher rank.

For instance, you may add the following file to add a check before committing to an update:

```bash title="/etc/rugix/hooks/system-commit/prepare/10-check_system_health.sh"
#!/bin/bash

# Function to check whether a service is active.
check_service_status() {
  service_name=$1
  if systemctl is-active --quiet "$service_name"; then
    echo "Service $service_name is running."
  else
    echo "Error: Service $service_name is not running."
    exit 1
  fi
}

# Check whether the SSH server is running.
check_service_status "sshd"
```

If hooks fail, the operation is typically aborted right then and there. So, in case of a failing `system-commit/prepare` hook, the commit will not go through. A failing hook, however, that is executed after the commit, will not revert the commit.

:::tip
You can perform various checks outside of Rugix Ctrl. For instance, you can check the system health prior to trying to commit an update. While it is generally a good idea to do that, it is certainly also recommended to perform checks with hooks, as these are always guaranteed to run, no matter how the operation is triggered.
:::

Hooks are provided with the operation as the first and the stage as the second argument. This allows you to write a hook that should run at multiple stages in a single file and then symlink this file into the respective locations. As certain operations and stages may be added in the future, and those may provide further arguments, **a hook should do nothing in case of unknown arguments**.
Note that it is also fine to ignore the first two arguments and assume that a hook only runs when it should based on its path.

In the following, we document the available hooks.


## System Update Hooks

For the installation of updates, the stages of `update-install` hooks are:

- `pre-update`: Runs directly before installing an update.
- `post-update`: Runs directly after installing an update (before rebooting).

For committing to an update or rollback, the stages of `system-commit` hooks are:

- `pre-commit`: Runs directly before a commit.
- `post-commit`: Runs directly after a commit.

You can use these hooks, e.g., to prepare and trigger state migrations, if you are not using Rugix Ctrl's [State Management](./state-management.mdx) feature.


## State Management Hooks

For factory resets, the stages of `state-reset` hooks are:

- `prepare`: Runs before initiating the reset and rebooting the system.
- `pre-reset`: Runs directly before a factory reset during boot (reset can still be aborted).
- `post-reset`: Runs directly after a factory reset during boot.

As explained in the section on [State Management](./state-management.mdx), the state management functionality runs very early during the boot process, before even the init system.
For the stages running during boot, you can assume the following environment:

- `/` is mounted read-only to the respective root filesystem.
- `/sys`, `/proc`, and `/dev` are mounted.
- `/run` is mounted to a temporary, in-memory filesystem and will be passed through to the final system.
If you need to communicate anything to processes after the bootstrapping process, place it in `/run`.

In addition, the config partition is mounted read-only (usually at `/run/rugix/mounts/config`).

## Bootstrapping Hooks

For bootstrapping, the stages of `bootstrap` hooks are:

- `prepare`: Runs after mounting the config partition and determining that the system should be bootstrapped.
- `pre-layout`: Runs directly before applying the system partition layout.
- `post-layout`: Runs directly after applying the system partition layout.

All stages run during the boot process and the same considerations as for [State Management Hooks](#state-management-hooks) apply (see above).
