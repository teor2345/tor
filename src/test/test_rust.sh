#!/bin/sh
# Test all the Rust crates we're using

crates=tor_util

exitcode=0

# check that CARGO_HOME is not relative (it will be interpreted
# relative to $abs_top_builddir/src/rust instead of $abs_top_builddir)
if [ -n "$CARGO_HOME" ] && [ "${CARGO_HOME:1:1}" != / ]; then
    echo "Tor's build system only supports absolute path (or unset) CARGO_HOME."
fi

for crate in $crates; do
    cd "${abs_top_srcdir:-../../..}/src/rust/${crate}"
    CARGO_TARGET_DIR="${abs_top_builddir:-../../..}/src/rust/target" "${CARGO:-cargo}" test ${CARGO_ONLINE-"--frozen"} || exitcode=1
done

exit $exitcode
