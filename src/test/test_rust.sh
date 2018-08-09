#!/bin/sh
# Test all Rust crates

set -e

export LSAN_OPTIONS=suppressions=${abs_top_srcdir}/src/test/rust_supp.txt

# check that CARGO_HOME is not relative (it will be interpreted
# relative to $abs_top_builddir/src/rust instead of $abs_top_builddir)
if [ -n "$$CARGO_HOME" ] && [ "$${CARGO_HOME:1:1}" != / ]; then
    echo "Tor's build system only supports absolute path (or unset) CARGO_HOME."
fi

for cargo_toml_dir in "${abs_top_srcdir:-../../..}"/src/rust/*; do
    if [ -e "${cargo_toml_dir}/Cargo.toml" ]; then
	    cd "$(abs_top_builddir)/src/rust" && \
	    CARGO_TARGET_DIR="${abs_top_builddir:-../../..}/src/rust/target" \
	        "${CARGO:-cargo}" test ${CARGO_ONLINE-"--frozen"} \
	        ${EXTRA_CARGO_OPTIONS} \
	        --manifest-path "${cargo_toml_dir}/Cargo.toml" || exitcode=1
    fi
done

exit $exitcode
