import binascii
import struct

cmdline = "console=serial0,115200 console=tty1 rootfstype=ext4 fsck.repair=yes rootwait panic=60 root=PARTUUID=2ddb0742-05 init=/usr/bin/rugpi-ctrl"
environ = [f"cmdline={cmdline}".encode("ascii")]

data = b"\0".join(environ) + b"\n"

crc = binascii.crc32(data)

with open("boot.env", "wb") as env_file:
    env_file.write(struct.pack("<I", crc))
    env_file.write(data)
