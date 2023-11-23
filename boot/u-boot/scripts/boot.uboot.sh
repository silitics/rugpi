echo "== Rugpi U-Boot First Stage =="

if load mmc 0:1 ${loadaddr} bootpart.default.env; then
    env import -c ${loadaddr} ${filesize}
fi
if load mmc 0:1 ${loadaddr} boot_spare.env; then
    env import -c ${loadaddr} ${filesize}
fi
if test "${bootpart}" = ""; then
    setenv bootpart 2
fi
echo "Boot Spare: " ${boot_spare}
if test "${boot_spare}" = "1"; then
    setexpr bootpart 5 - ${bootpart}
    if load mmc 0:1 ${loadaddr} boot_spare.disabled.env; then
        save mmc 0:1 ${loadaddr} boot_spare.env ${filesize}
    else
        # If loading `boot_spare.disabled.env` fails, simply write an empty file.
        save mmc 0:1 ${loadaddr} boot_spare.env 0
    fi
fi
echo "Bootpart: " ${bootpart}

# Load boot environment and hand off to second boot stage.
if load mmc 0:${bootpart} ${loadaddr} boot.env; then
    env import -c ${loadaddr} ${filesize}
fi
if load mmc 0:${bootpart} ${loadaddr} second.scr; then
    source ${loadaddr}
fi

echo "Executing second boot stage failed. Rebooting..."
sleep 10
reset
