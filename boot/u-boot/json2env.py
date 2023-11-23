#!/usr/bin/env python3

import typing as t

import binascii
import json
import pathlib
import struct
import sys


def encode_env(environ: t.Mapping[str, str]) -> bytes:
    data = (
        b"\0".join(f"{key}={value}".encode("ascii") for key, value in environ.items())
        + b"\0"
    )
    crc = binascii.crc32(data)
    return struct.pack("<I", crc) + data


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
