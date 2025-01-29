use std::fs::File;
use std::path::PathBuf;

use clap::Parser;
use rugix_bundle::format::tags::TagNameResolver;
use rugix_bundle::source::FileSource;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Parser)]
pub enum Cmd {
    Pack(PackCmd),
    Unpack(UnpackCmd),
    Hash(HashCmd),
    Print(PrintCmd),
}

#[derive(Debug, Parser)]
pub struct PrintCmd {
    bundle: PathBuf,
}

#[derive(Debug, Parser)]
pub struct PackCmd {
    src: PathBuf,
    bundle: PathBuf,
}

#[derive(Debug, Parser)]
pub struct UnpackCmd {
    bundle: PathBuf,
    dst: PathBuf,
}

#[derive(Debug, Parser)]
pub struct HashCmd {
    bundle: PathBuf,
}

fn main() {
    let args = Args::parse();
    match args.cmd {
        Cmd::Pack(pack_cmd) => {
            rugix_bundle::builder::pack(&pack_cmd.src, &pack_cmd.bundle).unwrap()
        }
        Cmd::Unpack(_unpack_cmd) => todo!("implement unpacking"),
        Cmd::Print(print_cmd) => {
            let mut source = FileSource::from_unbuffered(File::open(&print_cmd.bundle).unwrap());
            rugix_bundle::format::stlv::pretty_print(&mut source, Some(&TagNameResolver)).unwrap();
        }
        Cmd::Hash(hash_cmd) => {
            let hash = rugix_bundle::bundle_hash(&hash_cmd.bundle).unwrap();
            println!("{hash}");
        }
    }
}

/*
<BUNDLE (6b50741c)
  <BUNDLE_HEADER (49af6433)
    BUNDLE_HASH_ALGORITHM (06ec46db) [6B] = sha512
    <PAYLOAD_ENTRY (13737992)
      PAYLOAD_ENTRY_HASH (5f6a60b1) [64B] = \x82\x91\xb9\x89yM\xdcY\x93\xc9\xa0K\xfawe\x1f\xe5_\xf5\x85\xa2E ...
      BLOCK_DEDUPLICATION (5cb80dd6) [1B] = \x01
    PAYLOAD_ENTRY (13737992)>
  BUNDLE_HEADER (49af6433)>
  <PAYLOADS (01f38fba)
    <PAYLOAD (490cafaf)
      <PAYLOAD_HEADER (0959ca75)
        <BLOCK_INDEX_FIXED (76b3d7a0)
          BLOCK_INDEX_HASH_ALGORITHM (060973c9) [6B] = sha256
          BLOCK_INDEX_FIXED_SIZE (27e5d3f2) [8B] = \x00\x00\x00\x00\x00\x00 \x00
          BLOCK_INDEX_DATA (5ce61e83) [8.0742492MiB] = \x86\x81\r\x94\xba\xf5\xcd\xd4\xa2\x93zN\x84PG\xd3\x98\xc1\xfeS\xcb ...
        BLOCK_INDEX_FIXED (76b3d7a0)>
      PAYLOAD_HEADER (0959ca75)>
      PAYLOAD_DATA (42fd641a) [1.083946228GiB] = \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00 ...
    PAYLOAD (490cafaf)>
  PAYLOADS (01f38fba)>
BUNDLE (6b50741c)>

<BUNDLE (6b50741c)
  <BUNDLE_HEADER (49af6433)
    BUNDLE_HASH_ALGORITHM (06ec46db) [6B] = sha512
    <PAYLOAD_ENTRY (13737992)
      PAYLOAD_ENTRY_HASH (5f6a60b1) [64B] = \x82\x91\xb9\x89yM\xdcY\x93\xc9\xa0K\xfawe\x1f\xe5_\xf5\x85\xa2E ...
      BLOCK_DEDUPLICATION (5cb80dd6) [1B] = \x00
    PAYLOAD_ENTRY (13737992)>
  BUNDLE_HEADER (49af6433)>
  <PAYLOADS (01f38fba)
    <PAYLOAD (490cafaf)
      <PAYLOAD_HEADER (0959ca75)
        <BLOCK_INDEX_FIXED (76b3d7a0)
          BLOCK_INDEX_HASH_ALGORITHM (060973c9) [6B] = sha256
          BLOCK_INDEX_FIXED_SIZE (27e5d3f2) [8B] = \x00\x00\x00\x00\x00\x00 \x00
          BLOCK_INDEX_DATA (5ce61e83) [8.0742492MiB] = \x86\x81\r\x94\xba\xf5\xcd\xd4\xa2\x93zN\x84PG\xd3\x98\xc1\xfeS\xcb ...
        BLOCK_INDEX_FIXED (76b3d7a0)>
      PAYLOAD_HEADER (0959ca75)>
      PAYLOAD_DATA (42fd641a) [2.0185546875GiB] = \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00 ...
    PAYLOAD (490cafaf)>
  PAYLOADS (01f38fba)>
BUNDLE (6b50741c)>



<BUNDLE (6b50741c)
  <BUNDLE_HEADER (49af6433)
    BUNDLE_MANIFEST (5cb80dd6) [172B] = {\"hash-algorithm\":\"sha512\",\"payloads\":[{\"filename\":\"cus ...
  BUNDLE_HEADER (49af6433)>
  <PAYLOADS (01f38fba)
    <PAYLOAD (490cafaf)
      <PAYLOAD_HEADER (0959ca75)
        <BLOCK_ENCODING (40ed9314)
          BLOCK_ENCODING_INDEX (76b3d7a0) [623.37KiB] = =\xefU\\\"\xc7\x9b\xe6^\x1c\x9f\xed\xb62\xcf\x0e\x05\xe7\xa8\x82 ...
          BLOCK_ENCODING_SIZES (27e5d3f2) [58.43KiB] = \x00\x04\x00\x00\x00\x04\x00\x00\x00\x04\x00\x00\x00\x01H\xc0\x00 ...
        BLOCK_ENCODING (40ed9314)>
      PAYLOAD_HEADER (0959ca75)>
      PAYLOAD_DATA (42fd641a) [1.05GiB] = \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00 ...
    PAYLOAD (490cafaf)>
  PAYLOADS (01f38fba)>
BUNDLE (6b50741c)>


<BUNDLE (6b50741c)
  <BUNDLE_HEADER (49af6433)
    BUNDLE_MANIFEST (5cb80dd6) [210B] = {\"hash-algorithm\":\"sha512\",\"payloads\":[{\"filename\":\"cus ...
  BUNDLE_HEADER (49af6433)>
  <PAYLOADS (01f38fba)
    <PAYLOAD (490cafaf)
      <PAYLOAD_HEADER (0959ca75)
        <BLOCK_ENCODING (40ed9314)
          BLOCK_ENCODING_INDEX (76b3d7a0) [623.37KiB] = =\xefU\\\"\xc7\x9b\xe6^\x1c\x9f\xed\xb62\xcf\x0e\x05\xe7\xa8\x82 ...
          BLOCK_ENCODING_SIZES (27e5d3f2) [58.43KiB] = \x00\x00\x01\x80\x00\x00\x00\xac\x00\x00\x03\xf8\x00\x00/(\x00\x00 ...
        BLOCK_ENCODING (40ed9314)>
      PAYLOAD_HEADER (0959ca75)>
      PAYLOAD_DATA (42fd641a) [298.40MiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
    PAYLOAD (490cafaf)>
  PAYLOADS (01f38fba)>
BUNDLE (6b50741c)>


<BUNDLE (6b50741c)
  <BUNDLE_HEADER (49af6433)
    BUNDLE_MANIFEST (5cb80dd6) [210B] = {\"hash-algorithm\":\"sha512\",\"payloads\":[{\"filename\":\"cus ...
  BUNDLE_HEADER (49af6433)>
  <PAYLOADS (01f38fba)
    <PAYLOAD (490cafaf)
      <PAYLOAD_HEADER (0959ca75)
        <BLOCK_ENCODING (40ed9314)
          BLOCK_ENCODING_INDEX (76b3d7a0) [472.78KiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
          BLOCK_ENCODING_SIZES (27e5d3f2) [27.89KiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
        BLOCK_ENCODING (40ed9314)>
      PAYLOAD_HEADER (0959ca75)>
      PAYLOAD_DATA (42fd641a) [298.40MiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
    PAYLOAD (490cafaf)>
  PAYLOADS (01f38fba)>
BUNDLE (6b50741c)>

<BUNDLE (6b50741c)
  <BUNDLE_HEADER (49af6433)
    BUNDLE_MANIFEST (5cb80dd6) [209B] = {\"hash-algorithm\":\"sha512\",\"payloads\":[{\"filename\":\"cus ...
  BUNDLE_HEADER (49af6433)>
  <PAYLOADS (01f38fba)
    <PAYLOAD (490cafaf)
      <PAYLOAD_HEADER (0959ca75)
        <BLOCK_ENCODING (40ed9314)
          BLOCK_ENCODING_INDEX (76b3d7a0) [1.11MiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
          BLOCK_ENCODING_SIZES (27e5d3f2) [52.90KiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
        BLOCK_ENCODING (40ed9314)>
      PAYLOAD_HEADER (0959ca75)>
      PAYLOAD_DATA (42fd641a) [364.43MiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
    PAYLOAD (490cafaf)>
  PAYLOADS (01f38fba)>
BUNDLE (6b50741c)>


<BUNDLE (6b50741c)
  <BUNDLE_HEADER (49af6433)
    BUNDLE_MANIFEST (5cb80dd6) [156B] = {\"payloads\":[{\"filename\":\"customized-arm64.img\",\"block-en ...
    <PAYLOAD_ENTRY (13737992)
      PAYLOAD_ENTRY_HASH (5f6a60b1) [64B] = \xa44\x9dQ=\x0e\xc3\xb0-\x8d\x997K\x17\xc3\xf6\xa2\xdcJ\xd9\xa9/ ...
    PAYLOAD_ENTRY (13737992)>
  BUNDLE_HEADER (49af6433)>
  <PAYLOADS (01f38fba)
    <PAYLOAD (490cafaf)
      <PAYLOAD_HEADER (0959ca75)
        <BLOCK_ENCODING (40ed9314)
          BLOCK_ENCODING_INDEX (76b3d7a0) [576.27KiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
          BLOCK_ENCODING_SIZES (27e5d3f2) [29.12KiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
        BLOCK_ENCODING (40ed9314)>
      PAYLOAD_HEADER (0959ca75)>
      PAYLOAD_DATA (42fd641a) [351.00MiB] = \xfd7zXZ\x00\x00\x04\xe6\xd6\xb4F\x02\x00!\x01\x1c\x00\x00\x00\x10 ...
    PAYLOAD (490cafaf)>
  PAYLOADS (01f38fba)>
BUNDLE (6b50741c)>
*/
