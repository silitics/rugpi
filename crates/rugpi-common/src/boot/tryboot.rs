/// The autoboot configuration for system `A`.
pub const AUTOBOOT_A: &str = "[all]
tryboot_a_b=1
boot_partition=2
[tryboot]
boot_partition=3";

/// The autoboot configuration for system `B`.
pub const AUTOBOOT_B: &str = "[all]
tryboot_a_b=1
boot_partition=3
[tryboot]
boot_partition=2";
