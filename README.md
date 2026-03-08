# kattangalsec ectf hsm design

# build
cargo build --release --target thumbv6m-none-eabi 

# flash
uvx ectf hw /dev/ttyACM0 flash --name firmware ./target/thumbv6m-none-eabi/release/ectf-hsm-rust
