#!/bin/bash
set -e

CONFIG_FILE="/home/astroanax/dev/csc/ectf/2026-ectf-insecure-example/mspm0_openocd.cfg"
ELF_FILE="./target/thumbv6m-none-eabi/release/ectf-hsm-rust"

openocd -f "$CONFIG_FILE" -c "init; halt" &
OPENOCD_PID=$!
sleep 2

echo "$OPENOCD_PID"

arm-none-eabi-gdb \
    -ex "file $ELF_FILE" \
    -ex "target remote localhost:3333" \
    -ex "monitor reset halt" \
    -ex "load" \
    -ex "break main" \
    -ex "break uart_listener" \
    -ex "info break" \
    -ex "continue"

kill $OPENOCD_PID 2>/dev/null || true
