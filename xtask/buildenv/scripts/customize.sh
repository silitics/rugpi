#!/bin/bash

root_dir=$1;

# We remove those files as they are not needed and would cause the build to be non-deterministic.
rm "${root_dir}/etc/hostname"
rm "${root_dir}/etc/resolv.conf"
