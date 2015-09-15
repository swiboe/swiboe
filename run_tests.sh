#!/usr/bin/env bash
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.


set -ex

CARGO="$(which travis-cargo || which cargo)"

test_crate() {
   DIR=$1; shift;
   pushd $DIR > /dev/null

   $CARGO build

   # Only run tests if there is a rust file that contains #[test].
   FOUND=$(find . -name '*.rs' -print0 | \
      xargs -0 grep -q '^\s*#\[test\]' \
      && echo yes || echo no)
   if [[ $FOUND = "yes" ]]; then
      $CARGO test;
   fi

   # Only run benchmarks if there is a rust file that contains #[bench].
   FOUND=$(find . -name '*.rs' -print0 | \
      xargs -0 grep -q '^\s*#\[bench\]' \
      && echo yes || echo no)
   if [[ $FOUND = "yes" ]]; then
      $CARGO bench
   fi

   popd > /dev/null
}

# Build documentation.
pushd doc > /dev/null
sphinx-build -b html -W source build/html
popd > /dev/null

test_crate "."
test_crate "c_client"
test_crate "gui"
test_crate "subsequence_match"
test_crate "term_gui"
