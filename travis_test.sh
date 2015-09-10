#!/usr/bin/env bash
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.


set -ex

test_crate() {
   DIR=$1; shift;
   pushd $DIR

   travis-cargo build
   travis-cargo test
   travis-cargo bench

   popd
}

# Build documentation.
pushd doc
sphinx-build -b html -W source build/html
popd

test_crate "."
test_crate "c_client"
test_crate "gui"
test_crate "subsequence_match"
test_crate "term_gui"
