/* Copyright (c) 2016-2018, The Tor Project, Inc. */
/* See LICENSE for licensing information */

#define CONSDIFF_PRIVATE

#include "orconfig.h"
#include "core/or/or.h"
#include "feature/dircommon/consdiff.h"

#include "test/fuzz/fuzzing.h"

static int
mock_consensus_compute_digest_(const char *c, size_t l, consensus_digest_t *d)
{
  (void)c;
  (void)l;
  memset(d->sha3_256, 3, sizeof(d->sha3_256));
  return 0;
}

int
fuzz_init(void)
{
  MOCK(consensus_compute_digest, mock_consensus_compute_digest_);
  MOCK(consensus_compute_digest_as_signed, mock_consensus_compute_digest_);
  return 0;
}

int
fuzz_cleanup(void)
{
  UNMOCK(consensus_compute_digest);
  UNMOCK(consensus_compute_digest_as_signed);
  return 0;
}

int
fuzz_main(const uint8_t *stdin_buf, size_t data_size)
{
#define SEP "=====\n"
#define SEPLEN strlen(SEP)
  const uint8_t *separator = tor_memmem(stdin_buf, data_size, SEP, SEPLEN);
  if (! separator)
    return 0;
  size_t c1_len = separator - stdin_buf;
  const char *c1 = (const char*)stdin_buf;
  size_t c2_len = data_size - c1_len - SEPLEN;
  const char *c2 = (const char*)separator + SEPLEN;

  char *c3 = consensus_diff_generate(c1, c1_len, c2, c2_len);

  if (c3) {
    char *c4 = consensus_diff_apply(c1, c1_len, c3, strlen(c3));
    tor_assert(c4);
    if (strcmp(c2, c4)) {
      printf("%s\n", escaped(c1));
      printf("%s\n", escaped(c2));
      printf("%s\n", escaped(c3));
      printf("%s\n", escaped(c4));
    }
    tor_assert(! strcmp(c2, c4));
    tor_free(c3);
    tor_free(c4);
  }

  return 0;
}

