#!/bin/bash

mkdir -p generated
rm -rf generated/*
sidex generate json-schema generated/

mkdir -p ../../../schemas
cp generated/rugix_bakery.projects.ProjectConfig.schema.json ../../../schemas/rugix-bakery-project.schema.json
cp generated/rugix_bakery.layers.LayerConfig.schema.json ../../../schemas/rugix-bakery-layer.schema.json
cp generated/rugix_bakery.recipes.RecipeConfig.schema.json ../../../schemas/rugix-bakery-recipe.schema.json
cp generated/rugix_bakery.tests.TestConfig.schema.json ../../../schemas/rugix-bakery-test.schema.json
cp generated/rugix_bakery.repositories.RepositoryConfig.schema.json ../../../schemas/rugix-bakery-repository.schema.json