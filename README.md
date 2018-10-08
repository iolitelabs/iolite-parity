# Parity-iOlite - fast, light, and robust iOlite client

[![GPLv3](https://img.shields.io/badge/license-GPL%20v3-green.svg)](https://www.gnu.org/licenses/gpl-3.0.en.html)


### Join the chat!

Get in touch with us - join our community on Telegram:
[![Telegram: iOlite Community](https://img.shields.io/badge/Telegram-iolite-brightgreen.svg)](https://t.me/iolite)

Official website: https://iolite.io

iOlite explorer: https://scan.iolite.io

Be sure to check out [our wiki](https://wiki.iolite.io) for more information.

----

## About Parity

Parity's goal is to be the fastest, lightest, and most secure Ethereum client. We are developing Parity using the sophisticated and cutting-edge Rust programming language.

iOlite-Parity is a fork of official Parity-Ethereum repository with changes made by iOlite team to serve our project's purposes. iOlite-Parity was modified to support iOlite blockchain. To get more information about iOlite blockchain, check out our whitepaper on our [official website](https://iolite.io).

iOlite-Parity is licensed under the GPLv3, and can be used for all your iOlite needs.

By default, Parity will also run a JSONRPC server on `127.0.0.1:8545` and a websockets server on `127.0.0.1:8546`. This is fully configurable and supports a number of APIs.
To build iOlite-Parity you should follow the instructions below to build from source.

----

## Build dependencies

**iOlite-Parity requires Rust version 1.27.1 to build**

We recommend installing Rust through [rustup](https://www.rustup.rs/). If you don't already have rustup, you can install it like this:

- Linux:
	```bash
	$ curl https://sh.rustup.rs -sSf | sh
	```

	Parity also requires `gcc`, `g++`, `libssl-dev`/`openssl`, `libudev-dev` and `pkg-config` packages to be installed.

- OSX:
	```bash
	$ curl https://sh.rustup.rs -sSf | sh
	```

	`clang` is required. It comes with Xcode command line tools or can be installed with homebrew.

  ```

Once you have rustup installed, then you need to install:
* [Perl](https://www.perl.org)
* [Yasm](http://yasm.tortall.net)

Make sure that these binaries are in your `PATH`. After that you should be able to build parity from source.

----

## Build from source

```bash
# download Parity code
$ git clone --single-branch -b iolite https://github.com/iolitelabs/iolite-parity.git
$ cd iolite-parity

# Set current Rust version to 1.27.1
$ rustup override set 1.27.1

# build in release mode
$ cargo build --release
```

This will produce an executable in the `./target/release` subdirectory.

Note: if cargo fails to parse manifest try:

```bash
$ ~/.cargo/bin/cargo build --release
```

Note: When compiling a crate and you receive the following error:

```
error: the crate is compiled with the panic strategy `abort` which is incompatible with this crate's strategy of `unwind`
```

Cleaning the repository will most likely solve the issue, try:

```bash
$ cargo clean
```

This will always compile the latest stable iOlite builds. 

----


## Start Parity

To start iOlite-Parity, just run

```bash
$ ./target/release/parity --config iolite_config_node.toml
```

and iOlite-Parity will begin syncing the iOlite blockchain.
