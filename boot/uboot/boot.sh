echo Rugpi Boot Script
echo =================


##############################################################################
# Load Rugpi environment files.
##############################################################################

if load mmc 0:1 ${loadaddr} default.env; then
    env import -c ${loadaddr} ${filesize}
fi
if load mmc 0:1 ${loadaddr} boot_spare.env; then
    env import -c ${loadaddr} ${filesize}
fi


##############################################################################
# Set `bootpart` to the active partition set (default or spare).
##############################################################################

if test ${bootpart} = ""; then
    # If `bootpart` is not set, boot from the second partition.
    setenv bootpart 2
fi

# If `boot_spare` is set, boot from the spare partition set.
if test ${boot_spare} = 1; then
    setexpr bootpart 5 - ${bootpart}
    # Next time, boot from the default partition set again.
    setenv boot_spare 0
    env export -c ${loadaddr} boot_spare
    save mmc 0:1 ${loadaddr} default.env ${filesize}
fi


##############################################################################
# Load environment file `boot.env` from active partition set.
##############################################################################

if load mmc 0:${bootpart} ${loadaddr} boot.env; then
    env import -c ${loadaddr} ${filesize}
fi


##############################################################################
# Print information and boot kernel.
##############################################################################

echo Bootpart: ${bootpart}
echo Cmdline: ${cmdline}

load mmc 0:${bootpart} ${kernel_addr_r} ${kernel_file}
setenv kernel_comp_addr_r ${loadaddr}
setenv kernel_comp_size 0x4000000
setenv bootargs ${cmdline}
# Try booting `zImage`.
booti ${kernel_addr_r} - ${fdt_addr}
# Try booting `uImage`.
bootm ${kernel_addr_r} - ${fdt_addr}

echo "Error loading kernel... Rebooting in 10 seconds."
sleep 10
reset
