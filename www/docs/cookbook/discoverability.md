# Discoverability

Sometimes an embedded device should be automatically discoverable in a network.
For instance, automatic discovery allows service technicians to scan for devices and users to find and access devices more easily.

To facility discovery, [Avahi](https://www.avahi.org/) can be used.
To install Avahi, you may start with the [Avahi Cookbook Recipe](https://github.com/silitics/rugpi/tree/main/cookbook/x-avahi-daemon).

By default, the recipe makes SSH, SFTP, and an HTTP interface discoverable via [DNS-SD](https://datatracker.ietf.org/doc/html/rfc6763).
In addition, it allows the resolution of the domain name `<hostname>.local` in the local network via [Multicast DNS](https://datatracker.ietf.org/doc/html/rfc6762).
As a result, it is easy to access the Raspberry Pi simply by entering its hostname followed by `.local`, e.g., in the navigation bar of a browser.