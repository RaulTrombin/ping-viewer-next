Place default firmware files here to be bundled with the app.

Expected structure:
- ping1d/Ping-V3.29_auto.hex
- ping2/Ping2-V1.1.0_auto.hex
- ping360/Ping360-V3.3.8_auto.hex
- utils/ping360-bootloader        (Linux/macOS) or ping360-bootloader.exe (Windows)
- utils/stm32flash                (Linux/macOS) or stm32flash.exe (Windows)

These files will be copied to the runtime directory:
<current working dir>/firmwares/

The application will only initialize/copy on first run or when the
destination is missing/empty, preserving any user modifications.


