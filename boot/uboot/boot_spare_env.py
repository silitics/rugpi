import binascii
import struct

environ = [b"boot_spare=1"]

data = b"\0".join(environ) + b"\n"

crc = binascii.crc32(data)

with open("boot_spare.env", "wb") as env_file:
    env_file.write(struct.pack("<I", crc))
    env_file.write(data)
