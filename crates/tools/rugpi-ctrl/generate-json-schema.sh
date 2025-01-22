#!/bin/bash

mkdir -p generated
rm -rf generated/*
sidex generate json-schema generated/

mkdir -p ../../../schemas
cp generated/rugix_ctrl.bootstrapping.BootstrappingConfig.schema.json ../../../schemas/rugix-ctrl-bootstrapping.schema.json
cp generated/rugix_ctrl.state.StateConfig.schema.json ../../../schemas/rugix-ctrl-state.schema.json
cp generated/rugix_ctrl.system.SystemConfig.schema.json ../../../schemas/rugix-ctrl-system.schema.json