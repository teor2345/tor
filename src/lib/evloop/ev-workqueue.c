/* copyright (c) 2013-2018, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * \file ev-workqueue.c
 *
 * \brief Provides integration between worker threads, implemented in
 * workqueue.c, and the global libevent mainloop in Tor.
 */

#include "lib/evloop/ev-workqueue.h"
#include "lib/evloop/compat_libevent.h"
#include "lib/log/util_bug.h"
#include <event2/event.h>

struct evwq_event {
  /** Event to notice when another thread has sent a reply. */
  struct event *reply_event;
  void (*reply_cb)(void *);
  void *arg;
};

/** Internal: Run from the libevent mainloop when there is work to handle in
 * the reply queue handler. */
static void
reply_event_cb(evutil_socket_t sock, short events, void *arg)
{
  replyqueue_t *rq = arg;
  (void) sock;
  (void) events;
  replyqueue_process(rq);
  struct evwq_event *data = replyqueue_get_data(rq);
  if (data && data->reply_cb)
    data->reply_cb(data->arg);
}

/** Register the replyqueue <b>rq</b> with Tor's global
 * libevent mainloop. If <b>cb</b> is provided, it is run after
 * each time there is work to process from the reply queue with the
 * argument <b>arg</b>. Return 0 on success, -1 on failure.
 */
int
tor_event_register_replyqueue(replyqueue_t *rq, void (*cb)(void *), void *arg)
{
  struct event_base *base = tor_libevent_get_base();
  struct evwq_event *data = replyqueue_get_data(rq);

  if (data) {
    tor_event_free(data->reply_event);
    tor_free(data);
  }
  data = tor_malloc(sizeof(*data));
  data->reply_event = tor_event_new(base,
                                    replyqueue_get_socket(rq),
                                    EV_READ|EV_PERSIST,
                                    reply_event_cb,
                                    rq);
  tor_assert(data->reply_event);
  data->reply_cb = cb;
  data->arg = arg;
  replyqueue_set_data(rq, data);
  return event_add(data->reply_event, NULL);
}
