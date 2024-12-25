# Integration Testing

:::warning
**Work in progress!** See https://github.com/silitics/rugpi/issues/41.
:::

Embedded Linux systems are inherently complex, with numerous interconnected components working together. To ensure that all parts of a system work together seamlessly, Rugpi Bakery includes an integration testing framework designed to validate system images as a complete unit. This framework uses virtual machines to execute comprehensive _test workflows_. By catching integration errors early, it minimizes the need for costly and time-consuming testing on physical hardware.

Test workflows are placed in the `tests` directory of your Rugpi Bakery project. Each workflow consists of a TOML file describing the test to be conducted.

- TODO: We probably need some sort of testing matrix to test different configurations/images.

```shell
./run-bakery test
```