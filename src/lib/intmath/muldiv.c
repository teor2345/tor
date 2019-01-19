/* Copyright (c) 2003-2004, Roger Dingledine
 * Copyright (c) 2004-2006, Roger Dingledine, Nick Mathewson.
 * Copyright (c) 2007-2018, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * \file muldiv.c
 *
 * \brief Integer math related to multiplication, division, and rounding.
 **/

#define TOR_INTMATH_MULDIV_PRIVATE
#include "lib/intmath/muldiv.h"

#include "lib/intmath/bits.h"
#include "lib/err/torerr.h"

#include <stdlib.h>

/** Return the lowest x such that x is at least <b>number</b>, and x modulo
 * <b>divisor</b> == 0.  If no such x can be expressed as an unsigned, return
 * UINT_MAX. Asserts if divisor is zero. */
unsigned
round_to_next_multiple_of(unsigned number, unsigned divisor)
{
  raw_assert(divisor > 0);
  if (UINT_MAX - divisor + 1 < number)
    return UINT_MAX;
  number += divisor - 1;
  number -= number % divisor;
  return number;
}

/** Return the lowest x such that x is at least <b>number</b>, and x modulo
 * <b>divisor</b> == 0. If no such x can be expressed as a uint32_t, return
 * UINT32_MAX. Asserts if divisor is zero. */
uint32_t
round_uint32_to_next_multiple_of(uint32_t number, uint32_t divisor)
{
  raw_assert(divisor > 0);
  if (UINT32_MAX - divisor + 1 < number)
    return UINT32_MAX;

  number += divisor - 1;
  number -= number % divisor;
  return number;
}

/** Return the lowest x such that x is at least <b>number</b>, and x modulo
 * <b>divisor</b> == 0. If no such x can be expressed as a uint64_t, return
 * UINT64_MAX. Asserts if divisor is zero. */
uint64_t
round_uint64_to_next_multiple_of(uint64_t number, uint64_t divisor)
{
  raw_assert(divisor > 0);
  if (UINT64_MAX - divisor + 1 < number)
    return UINT64_MAX;
  number += divisor - 1;
  number -= number % divisor;
  return number;
}

/* Helper: return greatest common divisor of a,b */
static uint64_t
gcd64(uint64_t a, uint64_t b)
{
  while (b) {
    uint64_t t = b;
    b = a % b;
    a = t;
  }
  return a;
}

/* Given a fraction *<b>numer</b> / *<b>denom</b>, simplify it.
 * Requires that the denominator is greater than 0. */
void
simplify_fraction64(uint64_t *numer, uint64_t *denom)
{
  raw_assert(denom);
  uint64_t gcd = gcd64(*numer, *denom);
  *numer /= gcd;
  *denom /= gcd;
}

/* Helper: safely multiply two uint32_t's, capping at UINT32_MAX rather
 * than overflow.
 * Uses 64-bit multiplication, rather than division, because division can
 * be expensive on some architectures. */
uint32_t
tor_mul_u32_nowrap(uint32_t a, uint32_t b)
{
  /* a*b > UINT32_MAX check, without division or overflow */
  uint64_t ab = a * b;
  if (PREDICT_UNLIKELY(ab > (uint64_t)UINT32_MAX)) {
    return UINT32_MAX;
  } else {
    return (uint32_t)ab;
  }
}

/* Helper: check if multiplying two uint64_t's could overflow.
 * Does not use division, which is expensive on some architectures.
 *
 * Returns 1 when a*b definitely does overflow.
 * Returns 0 when a*b may or may not overflow.
 * Returns -1 when a*b definitely does not overflow.
 */
STATIC int
tor_mul_u64_wrap_classify(uint64_t a, uint64_t b)
{
  /* tor_log2(0) incorrectly returns 0.
   * So we deal with these degenerate cases here:
   *   0 * n = 0
   *   1 * n = n
   */
  if (a < 2 || b < 2)
    return -1;

  /* tor_log2() returns floor(log2()), and 0 <= log2(a) - floor(log2(a)) < 1.
   * Therefore, the possible error in this log-based check is:
   *   2^n * 2^[0,1) * 2^m * 2^[0,1) = 2^(n+m) * 2^[0,2)
   * After applying log2():
   *   n + [0,1) + m + [0,1) = n + m + [0,2)
   */
  int log_ab_lower_bound = tor_log2(a) + tor_log2(b);
  if (log_ab_lower_bound >= 64) {
    /* For example:
     *  2^32 * 2^32 = 2^64 overflows,
     *  all the cross-products of [2^32, 2^33) also overflow. */
    return 1;
  } else if (log_ab_lower_bound <= 62) {
    /* For example:
     *  (2^32 - 1) * (2^32 - 1) = 2^64 - 2*2^32 + 1 does not overflow,
     *  all the cross-products of [2^31, 2^32) also do not overflow. */
    return -1;
  } else {
    /* For example:
     *  (2^32 - 1) * 2^32 = 2^64 - 2^32 does not overflow, and
     *  (2^32 - 1) * (2^32 + 1) = 2^64 - 1 does not overflow, but
     *  (2^32 - 1) * (2^32 + 2) = 2^64 + 2^32 - 2 does overflow. */
    return 0;
  }
}

/* Helper: safely multiply two uint64_t's, capping at UINT64_MAX rather
 * than overflow.
 * May use 64-bit division, which is expensive on some architectures. */
uint64_t
tor_mul_u64_nowrap(uint64_t a, uint64_t b)
{
  /* Approximate a*b > UINT64_MAX check, without division or overflow.
   * If you have fast division, and your compiler optimises tor_log2() badly,
   * you can skip this check. */
  int wrap_class = tor_mul_u64_wrap_classify(a, b);
  if (PREDICT_LIKELY(wrap_class < 0)) {
    return a*b;
  } else if (PREDICT_UNLIKELY(wrap_class > 0)) {
    return UINT64_MAX;
  }

  /* Accurate a*b > UINT64_MAX check, without overflow. */
  if (PREDICT_UNLIKELY(a > UINT64_MAX / b)) {
    return UINT64_MAX;
  } else {
    return a*b;
  }
}
