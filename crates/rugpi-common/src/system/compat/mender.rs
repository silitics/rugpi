//! Compatibility layer for Mender.

/*
    Grub Notes:

    Variables:
        bootcount => number of attempts of booting the upgrade
        upgrade_available => 1 = update, 0 = no update
        mender_boot_part => boot partition

    try_boot_next(entry):
        if entry != default:
            bootcount = 0
            upgrade_available = 1
            mender_boot_part = (root partition of the slot to boot)
        else:
            bootcount = 0
            upgrade_available = 0
            mender_boot_part = (root partition of the slot to boot)

    get_default()
        if upgrade_available == 1:
            invert mender_boot_part
        else:
            mender_boot part

    set_default(entry):
        bootcount = 0
        upgrade_available = 0
        mender_boot_part = (root partition of the slot to boot)
*/
