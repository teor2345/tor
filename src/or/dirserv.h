/* Copyright (c) 2001 Matej Pfajfar.
 * Copyright (c) 2001-2004, Roger Dingledine.
 * Copyright (c) 2004-2006, Roger Dingledine, Nick Mathewson.
 * Copyright (c) 2007-2018, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * \file dirserv.h
 * \brief Header file for dirserv.c.
 **/

#ifndef TOR_DIRSERV_H
#define TOR_DIRSERV_H

#include "lib/testsupport/testsupport.h"

/** What fraction (1 over this number) of the relay ID space do we
 * (as a directory authority) launch connections to at each reachability
 * test? */
#define REACHABILITY_MODULO_PER_TEST 128

/** How often (in seconds) do we launch reachability tests? */
#define REACHABILITY_TEST_INTERVAL 10

/** How many seconds apart are the reachability tests for a given relay? */
#define REACHABILITY_TEST_CYCLE_PERIOD \
  (REACHABILITY_TEST_INTERVAL*REACHABILITY_MODULO_PER_TEST)

/** Maximum length of an exit policy summary. */
#define MAX_EXITPOLICY_SUMMARY_LEN 1000

/** Maximum allowable length of a version line in a networkstatus. */
#define MAX_V_LINE_LEN 128

/** Maximum allowable length of bandwidth headers in a bandwidth file */
#define MAX_BW_FILE_HEADERS_LEN 50

/** Terminatore that separates bandwidth file headers from bandwidth file
 * relay lines */
#define BW_FILE_TERMINATOR "=====\n"

/** Ways to convert a spoolable_resource_t to a bunch of bytes. */
typedef enum dir_spool_source_t {
    DIR_SPOOL_SERVER_BY_DIGEST=1, DIR_SPOOL_SERVER_BY_FP,
    DIR_SPOOL_EXTRA_BY_DIGEST, DIR_SPOOL_EXTRA_BY_FP,
    DIR_SPOOL_MICRODESC,
    DIR_SPOOL_NETWORKSTATUS,
    DIR_SPOOL_CONSENSUS_CACHE_ENTRY,
} dir_spool_source_t;
#define dir_spool_source_bitfield_t ENUM_BF(dir_spool_source_t)

/** Object to remember the identity of an object that we are spooling,
 * or about to spool, in response to a directory request.
 *
 * (Why do we spool?  Because some directory responses are very large,
 * and we don't want to just shove the complete answer into the output
 * buffer: that would take a ridiculous amount of RAM.)
 *
 * If the spooled resource is relatively small (like microdescriptors,
 * descriptors, etc), we look them up by ID as needed, and add the whole
 * thing onto the output buffer at once.  If the spooled reseource is
 * big (like networkstatus documents), we reference-count it, and add it
 * a few K at a time.
 */
typedef struct spooled_resource_t {
  /**
   * If true, we add the entire object to the outbuf.  If false,
   * we spool the object a few K at a time.
   */
  unsigned spool_eagerly : 1;
  /**
   * Tells us what kind of object to get, and how to look it up.
   */
  dir_spool_source_bitfield_t spool_source : 7;
  /**
   * Tells us the specific object to spool.
   */
  uint8_t digest[DIGEST256_LEN];
  /**
   * A large object that we're spooling. Holds a reference count.  Only
   * used when spool_eagerly is false.
   */
  struct cached_dir_t *cached_dir_ref;
  /**
   * A different kind of large object that we might be spooling. Also
   * reference-counted.  Also only used when spool_eagerly is false.
   */
  struct consensus_cache_entry_t *consensus_cache_entry;
  const uint8_t *cce_body;
  size_t cce_len;
  /**
   * The current offset into cached_dir or cce_body. Only used when
   * spool_eagerly is false */
  off_t cached_dir_offset;
} spooled_resource_t;

#ifdef DIRSERV_PRIVATE
typedef struct measured_bw_line_t {
  char node_id[DIGEST_LEN];
  char node_hex[MAX_HEX_NICKNAME_LEN+1];
  long int bw_kb;
} measured_bw_line_t;
#endif /* defined(DIRSERV_PRIVATE) */

int connection_dirserv_flushed_some(dir_connection_t *conn);

int dirserv_add_own_fingerprint(crypto_pk_t *pk);
int dirserv_load_fingerprint_file(void);
void dirserv_free_fingerprint_list(void);
enum was_router_added_t dirserv_add_multiple_descriptors(
                                     const char *desc, uint8_t purpose,
                                     const char *source,
                                     const char **msg);
enum was_router_added_t dirserv_add_descriptor(routerinfo_t *ri,
                                               const char **msg,
                                               const char *source);
void dirserv_set_router_is_running(routerinfo_t *router, time_t now);
int list_server_status_v1(smartlist_t *routers, char **router_status_out,
                          int for_controller);
char *dirserv_get_flag_thresholds_line(void);
void dirserv_compute_bridge_flag_thresholds(void);

int directory_fetches_from_authorities(const or_options_t *options);
int directory_fetches_dir_info_early(const or_options_t *options);
int directory_fetches_dir_info_later(const or_options_t *options);
int directory_caches_unknown_auth_certs(const or_options_t *options);
int directory_caches_dir_info(const or_options_t *options);
int directory_permits_begindir_requests(const or_options_t *options);
int directory_too_idle_to_fetch_descriptors(const or_options_t *options,
                                            time_t now);

cached_dir_t *dirserv_get_consensus(const char *flavor_name);
void dirserv_set_cached_consensus_networkstatus(const char *consensus,
                                              const char *flavor_name,
                                              const common_digests_t *digests,
                                              const uint8_t *sha3_as_signed,
                                              time_t published);
void dirserv_clear_old_networkstatuses(time_t cutoff);
int dirserv_get_routerdesc_spool(smartlist_t *spools_out, const char *key,
                                 dir_spool_source_t source,
                                 int conn_is_encrypted,
                                 const char **msg_out);
int dirserv_get_routerdescs(smartlist_t *descs_out, const char *key,
                            const char **msg);
void dirserv_orconn_tls_done(const tor_addr_t *addr,
                             uint16_t or_port,
                             const char *digest_rcvd,
                             const ed25519_public_key_t *ed_id_rcvd);
int dirserv_should_launch_reachability_test(const routerinfo_t *ri,
                                            const routerinfo_t *ri_old);
void dirserv_single_reachability_test(time_t now, routerinfo_t *router);
void dirserv_test_reachability(time_t now);
int authdir_wants_to_reject_router(routerinfo_t *ri, const char **msg,
                                   int complain,
                                   int *valid_out);
uint32_t dirserv_router_get_status(const routerinfo_t *router,
                                   const char **msg,
                                   int severity);
void dirserv_set_node_flags_from_authoritative_status(node_t *node,
                                                      uint32_t authstatus);

int dirserv_would_reject_router(const routerstatus_t *rs);
char *routerstatus_format_entry(
                              const routerstatus_t *rs,
                              const char *version,
                              const char *protocols,
                              routerstatus_format_type_t format,
                              int consensus_method,
                              const vote_routerstatus_t *vrs);
void dirserv_free_all(void);
void cached_dir_decref(cached_dir_t *d);
cached_dir_t *new_cached_dir(char *s, time_t published);

int validate_recommended_package_line(const char *line);
int dirserv_query_measured_bw_cache_kb(const char *node_id,
                                       long *bw_out,
                                       time_t *as_of_out);
void dirserv_clear_measured_bw_cache(void);
int dirserv_has_measured_bw(const char *node_id);
int dirserv_get_measured_bw_cache_size(void);
void dirserv_count_measured_bws(const smartlist_t *routers);
int running_long_enough_to_decide_unreachable(void);
void dirserv_compute_performance_thresholds(digestmap_t *omit_as_sybil);

#ifdef DIRSERV_PRIVATE

STATIC void dirserv_set_routerstatus_testing(routerstatus_t *rs);

/* Put the MAX_MEASUREMENT_AGE #define here so unit tests can see it */
#define MAX_MEASUREMENT_AGE (3*24*60*60) /* 3 days */

STATIC int measured_bw_line_parse(measured_bw_line_t *out, const char *line,
                                  int line_is_after_headers);

STATIC int measured_bw_line_apply(measured_bw_line_t *parsed_line,
                           smartlist_t *routerstatuses);

STATIC void dirserv_cache_measured_bw(const measured_bw_line_t *parsed_line,
                               time_t as_of);
STATIC void dirserv_expire_measured_bw_cache(time_t now);

STATIC int
dirserv_read_guardfraction_file_from_str(const char *guardfraction_file_str,
                                      smartlist_t *vote_routerstatuses);
#endif /* defined(DIRSERV_PRIVATE) */

int dirserv_read_measured_bandwidths(const char *from_file,
                                     smartlist_t *routerstatuses,
                                     smartlist_t *bw_file_headers);

int dirserv_read_guardfraction_file(const char *fname,
                                 smartlist_t *vote_routerstatuses);

spooled_resource_t *spooled_resource_new(dir_spool_source_t source,
                                         const uint8_t *digest,
                                         size_t digestlen);
spooled_resource_t *spooled_resource_new_from_cache_entry(
                                      struct consensus_cache_entry_t *entry);
void spooled_resource_free_(spooled_resource_t *spooled);
#define spooled_resource_free(sp) \
  FREE_AND_NULL(spooled_resource_t, spooled_resource_free_, (sp))
void dirserv_spool_remove_missing_and_guess_size(dir_connection_t *conn,
                                                 time_t cutoff,
                                                 int compression,
                                                 size_t *size_out,
                                                 int *n_expired_out);
void dirserv_spool_sort(dir_connection_t *conn);
void dir_conn_clear_spool(dir_connection_t *conn);

#endif /* !defined(TOR_DIRSERV_H) */

