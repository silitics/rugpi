echo "== Rugpi U-Boot Second Stage =="

echo "Bootpart: " ${bootpart}
echo "Bootargs: " ${bootargs}

load mmc 0:${bootpart} ${kernel_addr_r} ${kernel_file}
setenv kernel_comp_addr_r ${loadaddr}
setenv kernel_comp_size 0x4000000
# Try booting compressed kernel image.
booti ${kernel_addr_r} - ${fdt_addr}
# Try booting `zImage` kernel.
bootz ${kernel_addr_r} - ${fdt_addr}

echo "Error loading kernel. Rebooting..."
sleep 10
reset
