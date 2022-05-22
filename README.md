# Stratum
Workspace containing a full stratum client, server and proxy in Rust.

What is the point of this?
Every stratum is different, and so consider this like an http library. The point is not to write your API for you, but to make it easier for you to write an API. 

This is that for stratums. It TODO

## How is this workspace organized?
```
    stratum
          └── client  # A client implementation of the stratum protocol
          └── proxy   # A TCP proxy that will hold and load balance connections
          └── server  # A generic stratum implementation 
          └── types   # The collection of types required for Stratum.
```

## Development

### Opening and Using Documentation

To run all of the rustdocs associated with stratum, it is recommended to run the following command:
`cargo doc --open --document-private-items --no-deps`

This will compile the documentation for every module in this crate, including private code. Important to note that it will not
compile the dependency documenation. If you would like to have that documentation compiled, then drop the `--no-deps` flag from the above 
command.

@todo link to the tests readme
### Testing for Memory Leaks

We currently use MacOS's new tooling `Instruments` for testing memory leaks. Along with this we use `cargo instruments`.

In order to install cargo instruments, you need to install from a separate branch until the master branch has been updated. To do this, run: `cargo install --branch update-cargo --git https://github.com/cmyr/cargo-instruments.git`.
