# Stratum
Workspace containing a full stratum client, server and proxy in Rust.

What is the point of this?
Every stratum is different, and so consider this like an http library. The point is not to write your API for you, but to make it easier for you to write an API. 

This is that for stratums. It TODO

## How is this workspace organized?
```
    stratum
          └── server  # A generic stratum implementation 
          └── proxy   # A TCP proxy that will hold and load balance connections
```

## Development

### Opening and Using Documentation

To run all of the rustdocs associated with stratum, it is recommended to run the following command:
`cargo doc --open --document-private-items --no-deps`

This will compile the documentation for every module in this crate, including private code. Important to note that it will not
compile the dependency documenation. If you would like to have that documentation compiled, then drop the `--no-deps` flag from the above 
command.

@todo link to the tests readme

## Testing

Due to the nature of some of the tests in this library (Mainly signal testing), cargo test will not work out of the box unless we skip those steps.

To Mitigate that process, we use a next-generation test runner called cargo-nextest

The quickest way to install is to run the following command: 
`cargo install cargo-nextest`

Further installation methods can be found at the official website [Nextest](https://nexte.st/index.html). 

After installing, to run our test suite simply run:
`cargo nextest run`

### Test Coverage

Testing coverage is generated automatically on every Pull Request - In order to generate the coverage data locally you first need to follow the setup instructions here: https://github.com/taiki-e/cargo-llvm-cov

This library provides us with source-based coverage data which is an improved method of detecting testing coverage.

Once you have install llvm-cov, run the following command for the coverage data to be generated to stdout (further configuration can be found in their docs):

`cargo llvm-cov nextest`


### Testing for Memory Leaks

We currently use MacOS's new tooling `Instruments` for testing memory leaks. Along with this we use `cargo instruments`.

In order to install cargo instruments, you need to install from a separate branch until the master branch has been updated. To do this, run: `cargo install --branch update-cargo --git https://github.com/cmyr/cargo-instruments.git`.
