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

struct ev_threadpool {
  /** Event to notice when another thread has sent a reply. */
  struct event *reply_event;
  void (*reply_cb)(threadpool_t *);
};

/** Internal: Run from the libevent mainloop when there is work to handle in
 * the reply queue handler. */
static void
reply_event_cb(evutil_socket_t sock, short events, void *arg)
{
  threadpool_t *tp = arg;
  (void) sock;
  (void) events;
  replyqueue_process(threadpool_get_replyqueue(tp));
  struct ev_threadpool *data = threadpool_get_data(tp);
  if (data && data->reply_cb)
    data->reply_cb(tp);
}

/** Register the threadpool <b>tp</b>'s reply queue with Tor's global
 * libevent mainloop. If <b>cb</b> is provided, it is run after
 * each time there is work to process from the reply queue. Return 0 on
 * success, -1 on failure.
 */
int
threadpool_register_reply_event(threadpool_t *tp,
                                void (*cb)(threadpool_t *tp))
{
  struct event_base *base = tor_libevent_get_base();
  replyqueue_t *rq = threadpool_get_replyqueue(tp);
  struct ev_threadpool *data = threadpool_get_data(tp);

  if (data) {
    tor_event_free(data->reply_event);
    tor_free(data);
  }
  data = tor_malloc(sizeof(*data));
  data->reply_event = tor_event_new(base,
                                    replyqueue_get_socket(rq),
                                    EV_READ|EV_PERSIST,
                                    reply_event_cb,
                                    tp);
  tor_assert(data->reply_event);
  data->reply_cb = cb;
  threadpool_set_data(tp, data);
  return event_add(data->reply_event, NULL);
}
