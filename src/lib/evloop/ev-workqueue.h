/* Copyright (c) 2013-2018, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * \file ev-workqueue.h
 * \brief Header for ev-workqueue.c
 **/

#ifndef TOR_EV_WORKQUEUE_H
#define TOR_EV_WORKQUEUE_H

#include "lib/evloop/workqueue.h"

int tor_event_register_replyqueue(replyqueue_t *rq,
                                  void (*cb)(void *), void *arg);

#endif /* !defined(TOR_EV_WORKQUEUE_H) */
