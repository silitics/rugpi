#![no_main]

use libfuzzer_sys::fuzz_target;

use byte_calc::NumBytes;

fuzz_target!(|data: &[u8]| {
    let _ = NumBytes::parse_ascii(data);
});
