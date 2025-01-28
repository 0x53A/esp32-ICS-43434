This is a minimal project for esp32-s3

## Prerequisites

```
cargo install cargo-espflash
cargo install ldproxy
cargo install espup
espup install
```

This may be needed:

CRATE_CC_NO_DEFAULTS=1 cargo +esp build
https://github.com/esp-rs/esp-idf-svc/issues/361#issuecomment-2031361817


On macOS, need to source this:

$HOME/export-esp.sh

```
. /Users/lukas/export-esp.sh
```