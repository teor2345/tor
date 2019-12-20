/* Copyright (c) 2013-2019, The Tor Project, Inc. */
/* See LICENSE for licensing information */

#ifndef TOR_BACKTRACE_H
#define TOR_BACKTRACE_H

/**
 * \file backtrace.h
 *
 * \brief Header for backtrace.c
 **/

#include "orconfig.h"
#include "lib/cc/compat_compiler.h"
#include "lib/cc/torint.h"
#include "lib/defs/logging_types.h"

typedef void (*tor_log_fn)(int, log_domain_mask_t, const char *fmt, ...)
  CHECK_PRINTF(3,4);

void log_backtrace_impl(int severity, log_domain_mask_t domain,
                        const char *msg,
                        tor_log_fn logger);
int configure_backtrace_handler(const char *tor_version);
void clean_up_backtrace_handler(void);
void dump_stack_symbols_to_error_fds(void);
const char *get_tor_backtrace_version(void);

#define log_backtrace(sev, dom, msg) \
  log_backtrace_impl((sev), (dom), (msg), tor_log)

#ifdef EXPOSE_CLEAN_BACKTRACE
#if defined(HAVE_EXECINFO_H) && defined(HAVE_BACKTRACE) && \
  defined(HAVE_BACKTRACE_SYMBOLS_FD) && defined(HAVE_SIGACTION)

/* It's hard to find a definition that satisfies all our OSes. */
#ifdef HAVE_CYGWIN_SIGNAL_H
#include <cygwin/signal.h>
#elif defined(HAVE_SYS_UCONTEXT_H)
#include <sys/ucontext.h>
#elif defined(HAVE_UCONTEXT_H)
#include <ucontext.h>
#endif /* defined(HAVE_CYGWIN_SIGNAL_H) || ... */

void clean_backtrace(void **stack, size_t depth, const ucontext_t *ctx);
#endif
#endif /* defined(EXPOSE_CLEAN_BACKTRACE) */

#endif /* !defined(TOR_BACKTRACE_H) */
