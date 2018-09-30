/* Copyright (c) 2013-2018, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * \file ev-workqueue.h
 * \brief Header for ev-workqueue.c
 **/

#ifndef TOR_EV_WORKQUEUE_H
#define TOR_EV_WORKQUEUE_H

#include "lib/evloop/workqueue.h"

int threadpool_register_reply_event(threadpool_t *tp,
                                    void (*cb)(threadpool_t *tp));

#endif /* !defined(TOR_EV_WORKQUEUE_H) */
