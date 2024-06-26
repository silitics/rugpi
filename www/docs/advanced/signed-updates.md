# Signed Updates

In contrast to other update solutions, Rugpi is intentionally kept simple and does not include its own signature mechanism or HTTP client. Following the UNIX philosophy, Rugpi accepts streaming updates via `stdin`. This enables the usage of proven software such as `curl` and `wget` to stream updates via HTTP and also facilitates the integration into custom update workflows. To realize verified updates, Rugpi provides an option `--check-hash` which can be used to check the SHA256 of a (streamed) update. Based upon this mechanism, signed updates can then be realized in various ways, for instance, by using GPG or OpenSSL signatures.

In a typical setup, you would create an *update manifest* which contains the hash of the update and an URL from where to download the update. You would then sign this manifest, e.g., with GPG or OpenSSL. The update workflow would first check the signature and then invoke `rugpi update` with `--check-hash` and stream in the update, e.g., with `curl` or `wget`, to it. This ensures that the update you install is indeed the one described in the signed manifest.

Here is an example:

```shell
rugpi-ctrl update install --check-hash sha256:a9627e22da964b5b6ad7c1465a79bae4d11b71a064966b37596c057de106c1a9 image.img
```

Note that this mechanism is very flexible and allows us to build on trusted tools in a simple way. You can also combine an update and the signed manifest in a `.tar` archive and process that in a streaming fashion as part of your update workflow. In the future, we may also consider adding something like that to Rugpi itself.
