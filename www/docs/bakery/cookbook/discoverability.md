# Discoverability

Sometimes an embedded device should be automatically discoverable in a network.
For instance, automatic discovery allows service technicians to scan for devices and users to find and access devices more easily.

To facility discovery, [Avahi](https://www.avahi.org/) can be used.
To install Avahi, you can use the [Avahi Recipe](https://github.com/silitics/rugpi-extra/tree/main/recipes/avahi) available in the [`rugpi-extra` repository](https://github.com/silitics/rugpi-extra/tree/main).

Installing Avahi makes it possible to reach the system under the local domain name `<hostname>.local` in the local network via [Multicast DNS](https://datatracker.ietf.org/doc/html/rfc6762).
As a result, it is easy to access the Raspberry Pi simply by entering its hostname followed by `.local`, e.g., in the navigation bar of a browser.
In addition, the recipe allows making SSH, SFTP, and an HTTP interface discoverable via [DNS-SD](https://datatracker.ietf.org/doc/html/rfc6763).
To this end, use the recipe's parameters.
