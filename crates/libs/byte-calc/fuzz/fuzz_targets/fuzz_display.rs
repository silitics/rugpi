#![no_main]

use libfuzzer_sys::fuzz_target;

use byte_calc::NumBytes;

fuzz_target!(|data: &[u8]| {
    if let Ok(num) = NumBytes::parse_ascii(data) {
        num.to_string();
    }
});
