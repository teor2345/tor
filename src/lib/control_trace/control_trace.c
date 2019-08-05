/* Copyright (c) 2001 Matej Pfajfar.
 * Copyright (c) 2001-2004, Roger Dingledine.
 * Copyright (c) 2004-2006, Roger Dingledine, Nick Mathewson.
 * Copyright (c) 2007-2019, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * @file control_trace.c
 * @brief Handling code for control message trace debug logging, at a lower
 *   level than the controller logging code.
 *
 * Sometimes we want to log control message traces as debug logs.  But since
 * debug logs become controller log events, they can't use the logging code
 * directly to report their errors. (If they did, we would modify the control
 * messages while debugging, and potentially introduce infinite loops between
 * the controller and logging code.)
 *
 * As a workaround, the logging code provides this module with a set of raw
 * fds to be used for reporting debug messages from the Tor controller code.
 *
 * Control message traces are not currently sent as syslogs, android logs,
 * or to any callback-based log destinations. While it is technically possible
 * to send controller traces via non-control logging functions, restricting
 * control traces to file descriptors allows us to re-use the torrerr/raw_log
 * code. It also simplifies the control trace-specific code.
 **/

#include "orconfig.h"

#include <stdlib.h>
#include <string.h>

#define CONTROL_TRACE_PRIVATE
#include "lib/control_trace/control_trace.h"
#include "lib/err/raw_log.h"
#include "lib/malloc/malloc.h"
#include "lib/string/printf.h"

/* The prefix string for every control trace log */
#define CONTROL_TRACE_PREFIX "Control Trace "

/** Array of fds that we use to log control message debug traces.
 * Unlike crashes, which can happen at any time, we won't log any control
 * message debug traces until logging has been initialised. */
static int *control_safe_log_fds = NULL;
/** The number of elements used in control_safe_log_fds */
static int n_control_safe_log_fds = 0;

/** Given a list of string arguments ending with a NULL, writes them
 * to our control message debug trace logs. */
STATIC void
tor_log_debug_control_safe(const char *m, ...)
{
  va_list ap;
  va_start(ap, m);
  tor_log_raw_ap(control_safe_log_fds, n_control_safe_log_fds, false, m, ap);
  va_end(ap);
}

/** Format a control connection pointer <b>conn</b> into an identifier for the
 * connection, and return it as a newly allocated string.
 *
 * The returned string must be freed using tor_free(). */
static char *
tor_control_conn_to_str_dup(const control_connection_t *conn)
{
  char *conn_fmt = NULL;
  int rv = 0;

  rv = tor_asprintf(&conn_fmt, "Conn: %p", conn);
  if (rv < 0 || (size_t)rv < strlen("Conn: 0xffffffff")) {
    tor_free(conn_fmt);
    return tor_strdup("Conn Formatting Error");
  } else {
    return conn_fmt;
  }
}

/** Log a trace of a control message to the control-safe logs.
 *
 * Called when tor sends the control channel <b>conn</b> an event message
 * <b>msg</b> of <b>type</b>.
 *
 * The pointer value of <b>conn</b> is used as an identifier in the logs.
 *
 * Low-level interface: use one of the typed functions/macros, rather than
 * using this function directly. */
void
tor_log_debug_control_safe_message(const control_connection_t *conn,
                                   const char *type, const char *msg)
{
  if (n_control_safe_log_fds) {
    char *conn_fmt = tor_control_conn_to_str_dup(conn);
    tor_log_debug_control_safe(CONTROL_TRACE_PREFIX,
                               conn_fmt, ", ",
                               type, ": ",
                               "Content: '", msg, "'.");
    tor_free(conn_fmt);
  }
}

/** Log a trace of a control command to the control-safe logs.
 *
 * Called when the control channel <b>conn</b> sends tor a command string
 * <b>cmd</b>, with arguments <b>args</b>.
 *
 * The pointer value of <b>conn</b> is used as an identifier in the logs. */
void
tor_log_debug_control_safe_command(const control_connection_t *conn,
                                   const char *cmd, const char *args)
{
  if (n_control_safe_log_fds) {
    char *conn_fmt = tor_control_conn_to_str_dup(conn);
    tor_log_debug_control_safe(CONTROL_TRACE_PREFIX,
                               conn_fmt, ", ",
                               "Command: '", cmd, "', ",
                               "Arguments: '", args, "'.");
    tor_free(conn_fmt);
  }
}

/** Set *<b>out</b> to a pointer to an array of the fds that we use to log
 * control message debug traces. Return the number of elements in the array.
 * If the number of elemements in the array is zero, *<b>out</b> may be NULL.
 */
int
tor_log_get_control_safe_debug_fds(const int **out)
{
  *out = control_safe_log_fds;
  return n_control_safe_log_fds;
}

/**
 * Update the list of fds that that we use to log control message debug traces.
 */
void
tor_log_set_control_safe_debug_fds(const int *fds, int n)
{
  tor_free(control_safe_log_fds);
  size_t byte_len = n * sizeof(int);
  if (byte_len > 0) {
    control_safe_log_fds = tor_malloc(byte_len);
    memcpy(control_safe_log_fds, fds, byte_len);
  } else {
    control_safe_log_fds = NULL;
  }
  n_control_safe_log_fds = n;
}

/**
 * Reset the list of emergency error fds to its default.
 */
void
tor_log_reset_control_safe_debug_fds(void)
{
  tor_log_set_control_safe_debug_fds(NULL, 0);
}
