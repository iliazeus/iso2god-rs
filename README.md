# iso2god-rs
A tool to convert Xbox 360 and original Xbox ISOs into an Xbox 360 compatible Games-On-Demand file format

This is an optimized rewrite of https://github.com/eliecharra/iso2god-cli, with a few extra features.

```
Usage: iso2god [OPTIONS] <SOURCE_ISO> <DEST_DIR>

Arguments:
  <SOURCE_ISO>  ISO file to convert
  <DEST_DIR>    A folder to write resulting GOD files to

Options:
      --dry-run             Do not convert anything, just print the title info
      --game-title <TITLE>  Set game title
      --trim                Trim off unused space from the ISO image
  -j, --num-threads <N>     Number of worker threads to use
  -h, --help                Print help
  -V, --version             Print version
```
