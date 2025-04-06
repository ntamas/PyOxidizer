#!/usr/bin/env bash

set -exo pipefail

yum install -y libatomic   # see https://github.com/rust-lang/cargo/issues/13546
curl https://sh.rustup.rs -sSf | sh -s -- -y
