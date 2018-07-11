/* Copyright (c) 2003, Roger Dingledine
 * Copyright (c) 2004-2006, Roger Dingledine, Nick Mathewson.
 * Copyright (c) 2007-2018, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * \file compress_buf.c
 *
 * \brief Working with compressed data in buffers.
 **/

#include "lib/cc/compat_compiler.h"
#include "lib/container/buffers.h"
#include "lib/compress/compress.h"
#include "lib/log/util_bug.h"

#ifdef PARANOIA
/** Helper: If PARANOIA is defined, assert that the buffer in local variable
 * <b>buf</b> is well-formed. */
#define check() STMT_BEGIN buf_assert_ok(buf); STMT_END
#else
#define check() STMT_NIL
#endif /* defined(PARANOIA) */

/** Compress or uncompress the <b>data_len</b> bytes in <b>data</b> using the
 * compression state <b>state</b>, appending the result to <b>buf</b>.  If
 * <b>done</b> is true, flush the data in the state and finish the
 * compression/uncompression.  Return -1 on failure, 0 on success. */
int
buf_add_compress(buf_t *buf, tor_compress_state_t *state,
                 const char *data, size_t data_len,
                 const int done)
{
  char tmp[256], *next;
  size_t old_avail, avail;
  int over = 0;

  do {
    next = tmp;
    avail = old_avail = sizeof tmp;
    switch (tor_compress_process(state, &next, &avail,
                                 &data, &data_len, done)) {
      case TOR_COMPRESS_DONE:
        over = 1;
        break;
      case TOR_COMPRESS_ERROR:
        return -1;
      case TOR_COMPRESS_OK:
        if (data_len == 0) {
          tor_assert_nonfatal(!done);
          over = 1;
        }
        break;
      case TOR_COMPRESS_BUFFER_FULL:
        if (data_len == 0 && !done) {
          /* We've consumed all the input data, though, so there's no
           * point in forging ahead right now. */
          over = 1;
        }
        break;
    }
    buf_add(buf, tmp, old_avail - avail);

  } while (!over);
  check();
  return 0;
}
