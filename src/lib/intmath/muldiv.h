/* Copyright (c) 2003-2004, Roger Dingledine
 * Copyright (c) 2004-2006, Roger Dingledine, Nick Mathewson.
 * Copyright (c) 2007-2018, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * \file muldiv.h
 *
 * \brief Header for muldiv.c
 **/

#ifndef TOR_INTMATH_MULDIV_H
#define TOR_INTMATH_MULDIV_H

#include "lib/cc/torint.h"
#include "lib/cc/compat_compiler.h"
#include "lib/testsupport/testsupport.h"

unsigned round_to_next_multiple_of(unsigned number, unsigned divisor);
uint32_t round_uint32_to_next_multiple_of(uint32_t number, uint32_t divisor);
uint64_t round_uint64_to_next_multiple_of(uint64_t number, uint64_t divisor);

void simplify_fraction64(uint64_t *numer, uint64_t *denom);

/* Compute the CEIL of <b>a</b> divided by <b>b</b>, for nonnegative <b>a</b>
 * and positive <b>b</b>.  Works on integer types only. Not defined if a+(b-1)
 * can overflow. */
#define CEIL_DIV(a,b) (((a)+((b)-1))/(b))

uint32_t tor_mul_u32_nowrap(uint32_t a, uint32_t b);
uint64_t tor_mul_u64_nowrap(uint64_t a, uint64_t b);

#ifdef TOR_INTMATH_MULDIV_PRIVATE
STATIC int tor_mul_u64_wrap_classify(uint64_t a, uint64_t b) ATTR_CONST;
#endif /* TOR_INTMATH_MULDIV_PRIVATE */

#endif /* !defined(TOR_INTMATH_MULDIV_H) */
