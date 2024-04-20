# brun

Watch for changes on a remote git branch and run a command.

## Why

Have you ever had to run some heavy tests on some remote machine, like running a 
webserver on some hardware that you don't have locally but you still want to 
develop the code locally?

In this situation, you can install `brun` in the remote server and point it to
a branch reference on Github with a `command` to run, and `brun` will watch for 
any file changes in that branch, and will re-run `command` with the new changes.

## Installation

To install it, clone the repo and run:

```sh
cargo install --path . 
```

This will build the CLI and install it in `$HOME/.cargo/bin`. You should make
sure that you have that installed


## Usage

To run the command, you can just call it with: 

```sh
brun -- echo 'Something changed!'
```

This will listen for changes in the current checked out branch, pull the changes
as they happen in the remote, and run the given command.
