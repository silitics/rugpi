//! Raspberry Pi-specific functionality.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::Path;

use nix::fcntl;
use nix::libc::{c_char, c_int};
use nix::sys::stat;

/// Check whether the device is a Raspberry Pi.
pub fn is_raspberry_pi() -> bool {
    // We simply check whether the VCIO device for communicating with Raspberry Pi's
    // firmware exists. If the VCIO device does not exist, the device may still be a
    // Raspberry Pi, however, we would be unable to interact with its firmware.
    Vcio::exists()
}

/// Retrieve the tryboot flag from Raspberry Pi's firmware.
pub fn get_tryboot_flag() -> io::Result<bool> {
    let vcio = Vcio::open()?;
    Ok(get_reboot_flags(&vcio)? & 1 != 0)
}

/// Set the tryboot flag via Raspberry Pi's firmware.
///
/// This function is based on the implementation of Raspberry Pi's Linux driver:
/// <https://github.com/raspberrypi/linux/blob/085e8b4e0e1268ab82245e3433fb33399720b7ff/drivers/firmware/raspberrypi.c#L191>
pub fn set_tryboot_flag(tryboot: bool) -> io::Result<()> {
    let vcio = Vcio::open()?;
    if tryboot {
        set_reboot_flags(&vcio, 1)?;
    } else {
        set_reboot_flags(&vcio, 0)?;
    }
    Ok(())
}

/// Request tag for retrieving reboot flags.
const RPI_FIRMWARE_GET_REBOOT_FLAGS: u32 = 0x00030064_u32;
/// Request tag for setting reboot flags.
const RPI_FIRMWARE_SET_REBOOT_FLAGS: u32 = 0x00038064_u32;

/// Status code indicating a request.
const RPI_FIRMWARE_STATUS_REQUEST: u32 = 0x00000000_u32;
/// Status code indicating success.
const RPI_FIRMWARE_STATUS_SUCCESS: u32 = 0x80000000_u32;

/// Offset of the status code in the buffer.
const BUFFER_STATUS_OFFSET: usize = 1;

/// Offset of the reboot flags in the buffer.
const BUFFER_REBOOT_FLAGS_OFFSET: usize = 5;

/// Retrieve the current reboot flags from Raspberry Pi's firmware.
fn get_reboot_flags(vcio: &Vcio) -> io::Result<u32> {
    let mut buffer = encode_reboot_flags_request(RPI_FIRMWARE_GET_REBOOT_FLAGS, 0);
    unsafe {
        // SAFETY: Buffer is valid as required by the property interface.
        vcio.ioctl_property(&mut buffer)?;
    }
    if buffer[BUFFER_STATUS_OFFSET] != RPI_FIRMWARE_STATUS_SUCCESS {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Unable to retrieve reboot flags from Raspberry Pi's firmware (0x{:08X}).",
                buffer[BUFFER_STATUS_OFFSET]
            ),
        ));
    }
    Ok(buffer[BUFFER_REBOOT_FLAGS_OFFSET])
}

/// Set the current reboot flags via Raspberry Pi's firmware.
fn set_reboot_flags(vcio: &Vcio, flags: u32) -> io::Result<u32> {
    let mut buffer = encode_reboot_flags_request(RPI_FIRMWARE_SET_REBOOT_FLAGS, flags);
    unsafe {
        // SAFETY: Buffer is valid as required by the property interface.
        vcio.ioctl_property(&mut buffer)?;
    }
    if buffer[BUFFER_STATUS_OFFSET] != RPI_FIRMWARE_STATUS_SUCCESS {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Unable to set reboot flags via Raspberry Pi's firmware (0x{:08X}).",
                buffer[BUFFER_STATUS_OFFSET]
            ),
        ));
    }
    Ok(buffer[BUFFER_REBOOT_FLAGS_OFFSET])
}

/// Encode a request for retrieving or setting reboot flags.
fn encode_reboot_flags_request(tag: u32, flags: u32) -> [u32; 7] {
    [
        7 * 4,                       // Size of the buffer in bytes.
        RPI_FIRMWARE_STATUS_REQUEST, // Status code.
        tag,                         // Tag of the request.
        4,                           // Size of the value buffer in bytes.
        0,                           // Tag request code.
        flags,                       // Reboot flags.
        0,                           // End tag.
    ]
}

/// Path to the VCIO device for communicating with Raspberry Pi's firmware.
const VCIO_PATH: &str = "/dev/vcio";

/// Handle to the VCIO device.
#[derive(Debug)]
struct Vcio {
    /// Underlying file descriptor.
    fd: OwnedFd,
}

impl Vcio {
    /// Check whether the VCIO device exists.
    pub fn exists() -> bool {
        Path::new(VCIO_PATH).exists()
    }

    /// Open a handle to the VCIO device.
    pub fn open() -> io::Result<Self> {
        let flags = fcntl::OFlag::O_NONBLOCK;
        let mode = stat::Mode::empty();
        let fd = fcntl::open(VCIO_PATH, flags, mode)?;
        Ok(Self {
            fd: unsafe {
                // SAFETY: We own the file descriptor.
                OwnedFd::from_raw_fd(fd)
            },
        })
    }

    /// Perform an `ioctl` call to the VCIO property interface using the provided buffer.
    ///
    /// # Safety
    ///
    /// The provided `buffer` must be valid as required by the property interface.
    pub unsafe fn ioctl_property(&self, buffer: &mut [u32]) -> io::Result<c_int> {
        // Violating this safety precondition will most likely cause UB.
        assert!(
            buffer[0] <= (buffer.len() * 4) as u32,
            "Invalid buffer size. Buffer is smaller than indicated."
        );

        /// The `ioctl` identifier of the property interface.
        const IOCTL_IDENTIFIER: u8 = 100;
        /// The `ioctl` sequence number of the property interface.
        const IOCTL_SEQ_PROPERTY: u8 = 0;

        // We have to cast to `c_int` to make this work on 32-bit and 64-bit.
        const IOCTL_REQUEST_CODE: c_int = nix::request_code_readwrite!(
            IOCTL_IDENTIFIER,
            IOCTL_SEQ_PROPERTY,
            std::mem::size_of::<*mut c_char>()
        ) as c_int;

        // We have to use `ioctl_readwrite_bad` here because the code is computed with
        // `*mut c_char` but the actual type needs to be `c_char`.
        nix::ioctl_readwrite_bad! {
            /// Raw `ioctl` call.
            ioctl_property,
            IOCTL_REQUEST_CODE,
            c_char
        };

        Ok(ioctl_property(
            self.fd.as_raw_fd(),
            buffer.as_mut_ptr() as *mut c_char,
        )?)
    }
}
