# iso2god-rs
A tool to convert Xbox 360 ISOs into a Games-On-Demand file format

This is an optimized rewrite of https://github.com/eliecharra/iso2god-cli, with a few extra features.

```
USAGE:
    iso2god [OPTIONS] <SOURCE_ISO> <DEST_DIR>

ARGS:
    <SOURCE_ISO>    Xbox 360 ISO file to convert
    <DEST_DIR>      A folder to write resulting GOD files to

OPTIONS:
        --offline                    Do not query XboxUnity for title info
        --dry-run                    Do not convert anything, just query the title info
        --game-title <GAME_TITLE>    Set game title
    -h, --help                       Print help information
    -V, --version                    Print version information
```

