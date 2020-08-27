# nanodns-rs
[![Gitpod ready-to-code](https://img.shields.io/badge/Gitpod-ready--to--code-blue?logo=gitpod)](https://gitpod.io/from-referrer/)

An ultra-minimal authorative DNS server written in Rust, inspired by https://github.com/floe/nanodns.

```
nanodns 0.1.0
Gero Posmyk-Leinemann <gero.posmyk-leinemann@typefox.io>
ultra-minimal authorative DNS server

USAGE:
    nanodns-rs [FLAGS] <addr> [path-to-names.conf]

ARGS:
    <addr>                  The local socket address to bind to. ex.: 127.0.0.1:53
    <path-to-names.conf>    The path to the name file to use [default: ./names.conf]

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    Controls the log level. ex.: -v,  -vv or -vvv
    -V, --version    Prints version information
```

```
./nanodns-rs 127.0.0.1:20053 ./names.conf -v 
 2020-08-27T09:18:24.195Z INFO  nanodns_rs::dns > zone contents:
[("my.domain.com", (Name { is_fqdn: true, labels: [my, domain, com] }, 1.2.3.4))]
 2020-08-27T09:18:24.197Z INFO  nanodns_rs      > listening for DNS queries on 127.0.0.1:20053...
```

# Build
```
cargo build
```
