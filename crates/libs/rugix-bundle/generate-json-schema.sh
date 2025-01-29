#!/bin/bash

mkdir -p generated
rm -rf generated/*
sidex generate json-schema generated/

mkdir -p ../../../schemas
cp generated/rugix_bundle.manifest.BundleManifest.schema.json ../../../schemas/rugix-bundle-manifest.schema.json