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

**TODO**: Documentation
