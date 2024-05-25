//! Utilities for working with the filesystem.

use std::{
    fs::{self, File},
    io::{Read, Write},
    os::fd::{AsRawFd, RawFd},
    path::Path,
};

use anyhow::Context;
use nix::{
    errno::Errno,
    fcntl::FallocateFlags,
    libc::off64_t,
    unistd::{lseek64, Whence},
};
use xscript::{run, Run};

use crate::Anyhow;

pub fn copy_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Anyhow<()> {
    let dst = dst.as_ref();
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).ok();
    };
    // TODO: Implement in Rust instead of shelling out.
    run!(["cp", "-rTp", src.as_ref(), dst])?;
    Ok(())
}

pub fn allocate_file(path: &Path, size: u64) -> Anyhow<()> {
    let file = fs::File::create(path)?;
    nix::fcntl::fallocate(
        file.as_raw_fd(),
        nix::fcntl::FallocateFlags::empty(),
        0,
        size as i64,
    )?;
    Ok(())
}

pub fn punch_hole(fd: RawFd, offset: off64_t, size: off64_t) -> Anyhow<()> {
    nix::fcntl::fallocate(
        fd,
        FallocateFlags::FALLOC_FL_PUNCH_HOLE | FallocateFlags::FALLOC_FL_KEEP_SIZE,
        offset,
        size,
    )?;
    Ok(())
}

pub fn copy_sparse(
    src: &mut File,
    dst: &mut File,
    src_offset: u64,
    dst_offset: u64,
    size: u64,
) -> Anyhow<()> {
    let mut src_offset = off64_t::try_from(src_offset).unwrap();
    let mut dst_offset = off64_t::try_from(dst_offset).unwrap();
    let src_raw_fd = src.as_raw_fd();
    let dst_raw_fd = dst.as_raw_fd();
    lseek64(src_raw_fd, src_offset, Whence::SeekSet)?;
    lseek64(dst_raw_fd, dst_offset, Whence::SeekSet)?;
    let mut total_remaining = usize::try_from(size).unwrap();
    let mut buffer = vec![0; 8192];
    while total_remaining > 0 {
        // If there is no hole, then `next_hole` points to the end of the file as there
        // always is an implicit hole at the end of any file.
        let next_hole = lseek64(src_raw_fd, src_offset, Whence::SeekHole).context("next hole")?;
        lseek64(src.as_raw_fd(), src_offset, Whence::SeekSet).context("seek set")?;
        let chunk_size = usize::try_from(next_hole - src_offset).unwrap();
        let mut chunk_remaining = chunk_size;
        while chunk_remaining > 0 && total_remaining > 0 {
            let chunk_read = buffer.len().min(chunk_remaining).min(total_remaining);
            src.read_exact(&mut buffer[..chunk_read])?;
            dst.write_all(&buffer[..chunk_read])?;
            chunk_remaining -= chunk_read;
            total_remaining -= chunk_read;
            dst_offset += chunk_read as i64;
        }
        if total_remaining > 0 {
            src_offset = match lseek64(src_raw_fd, next_hole, Whence::SeekData) {
                Ok(src_offset) => src_offset,
                Err(Errno::ENXIO) => {
                    lseek64(
                        dst_raw_fd,
                        total_remaining.try_into().unwrap(),
                        Whence::SeekCur,
                    )?;
                    break;
                }
                error => error.context("seek data")?,
            };
            let hole_size = src_offset - next_hole;
            // TODO: Punch a hole in the destination file. This requires us to consider
            // the block size of dst's filesystem.
            // punch_hole(dst_raw_fd, dst_offset, hole_size)?;
            dst_offset += hole_size;
            lseek64(dst_raw_fd, hole_size, Whence::SeekCur)?;
            total_remaining -= usize::try_from(hole_size).unwrap();
        }
    }
    Ok(())
}
