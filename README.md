# S5Late
Tethered iPod bootrom/DFU exploit.

Currently only supports Nano 7G, to support Nano 6G offsets need to be updated.

# Running
- Put your iPod into DFU mode.
- Then run:
```
./S5Late hax

[2024-12-15T23:11:34Z INFO  S5Late] We found an Apple Device in DFU mode.
[2024-12-15T23:11:34Z INFO  S5Late] Succesfully exploited the device!
```

If the exploit ran succesfully now you can load unencrypted images without signature by running:

```
./S5Late load <image_path>

[2025-02-03T09:58:27Z INFO  S5Late] We found an Apple Device in DFU mode.
[2025-02-03T09:58:27Z INFO  S5Late::apple_device] Adding IMG header to the binary.
[2025-02-03T09:58:27Z INFO  S5Late] Image (<image_path>) loaded to the device.
```

*Note:* A minimal header expected by the bootrom is prepended to the binary before loading.

# Vulnerability
This exploits a vulnerability in DFU_DNLOAD packet parsing code, where no check exists for the total amount of bytes received. This means we can keep sending bytes until we overwrite some pointers at the end of SRAM.

# Thanks
- q3k - for general help
- CUB3D - for ipod_sun
