---
sidebar_position: 4
---

# CI/CD Integration

You can run Rugpi Bakery as part of a CI/CD pipeline, enabling a modern development workflow.
Please be aware that building an image is a rather resource-heavy process and may quickly consume your CI minutes.

## GitHub Actions

Here is an example for using Rugpi Bakery with GitHub Actions:

```yaml
jobs:  
  bake-image:
    name: Bake Image
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install QEMU
        run: docker run --privileged --rm tonistiigi/binfmt --install all
      
      - name: Bake Image
        run: ./run-bakery bake image customized

      - name: Upload Image
        uses: actions/upload-artifact@v4
        with:
          name: customized.img
          path: build/customized/system.img
```

## GitLab CI/CD

To run Rugpi Bakery in GitLab CI/CD it needs to be configured such that it is able to start Docker containers.
If you are using the Docker-based GitLab Runner you must configure it in privileged mode.
For details, we refer to [GitLab's documentation](https://docs.gitlab.com/ee/ci/docker/using_docker_build.html#use-docker-in-docker).