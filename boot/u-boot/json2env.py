#!/usr/bin/env python3

"""
Python utility for converting JSON files to U-Boot environments with CRC32 checksums.
"""

import typing as t

import binascii
import json
import pathlib
import struct
import sys


def encode_env(environ: t.Mapping[str, str]) -> bytes:
    """
    Encode the given environment.
    """
    data = (
        b"\0".join(f"{key}={value}".encode("ascii") for key, value in environ.items())
        + b"\0"
    )
    checksum = binascii.crc32(data)
    return struct.pack("<I", checksum) + data


def main():
    if len(sys.argv) != 3:
        print("usage: json2env.py <json> <env>")
        sys.exit(1)
    json_file = pathlib.Path(sys.argv[1])
    env_file = pathlib.Path(sys.argv[2])
    environ = json.loads(json_file.read_text())
    env_file.write_bytes(encode_env(environ))


if __name__ == "__main__":
    main()
