/* Copyright (c) 2001 Matej Pfajfar.
 * Copyright (c) 2001-2004, Roger Dingledine.
 * Copyright (c) 2004-2006, Roger Dingledine, Nick Mathewson.
 * Copyright (c) 2007-2019, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * @file dirauth_config.c
 * @brief Code to interpret the user's configuration of Tor's directory
 *        authority module.
 **/

#include "orconfig.h"
#include "feature/dirauth/dirauth_config.h"

#include "lib/encoding/confline.h"
#include "lib/confmgt/confmgt.h"

/* Required for dirinfo_type_t in or_options_t */
#include "core/or/or.h"
#include "app/config/config.h"

#include "feature/dircommon/voting_schedule.h"
#include "feature/stats/rephist.h"

#include "feature/dirauth/authmode.h"
#include "feature/dirauth/bwauth.h"
#include "feature/dirauth/dirauth_periodic.h"
#include "feature/dirauth/dirvote.h"
#include "feature/dirauth/guardfraction.h"

/* Copied from config.c, we will refactor later in 29211. */
#define REJECT(arg) \
  STMT_BEGIN *msg = tor_strdup(arg); return -1; STMT_END
#if defined(__GNUC__) && __GNUC__ <= 3
#define COMPLAIN(args...) \
  STMT_BEGIN log_warn(LD_CONFIG, args); STMT_END
#else
#define COMPLAIN(args, ...)                                     \
  STMT_BEGIN log_warn(LD_CONFIG, args, ##__VA_ARGS__); STMT_END
#endif /* defined(__GNUC__) && __GNUC__ <= 3 */

#define YES_IF_CHANGED_INT(opt) \
  if (!CFG_EQ_INT(old_options, new_options, opt)) return 1;

/** Scan <b>options</b> for occurrences of relative dirauth file/directory
 * paths and log a warning whenever one is found.
 *
 * Return 1 if there were relative paths; 0 otherwise.
 */
int
options_warn_about_relative_paths_dirauth(const or_options_t *options)
{
  tor_assert(options);
  int n = 0;

  n += warn_if_option_path_is_relative("V3BandwidthsFile",
                                       options->V3BandwidthsFile);
  n += warn_if_option_path_is_relative("GuardfractionFile",
                                       options->GuardfractionFile);

  return n != 0;
}

/**
 * Legacy validation/normalization function for the dirauth mode options in
 * options. Uses old_options as the previous options.
 *
 * Returns 0 on success, returns -1 and sets *msg to a newly allocated string
 * on error.
 */
int
options_validate_dirauth_mode(const or_options_t *old_options,
                              or_options_t *options,
                              char **msg)
{
  if (BUG(!options))
    return -1;

  if (BUG(!msg))
    return -1;

  if (options->AuthoritativeDir) {
    /* confirm that our address isn't broken, so we can complain now */
    uint32_t tmp;
    if (resolve_my_address(LOG_WARN, options, &tmp, NULL, NULL) < 0)
      REJECT("Failed to resolve/guess local address. See logs for details.");

    if (!options->ContactInfo && !options->TestingTorNetwork)
      REJECT("Authoritative directory servers must set ContactInfo");
    if (!options->RecommendedClientVersions)
      options->RecommendedClientVersions =
        config_lines_dup(options->RecommendedVersions);
    if (!options->RecommendedServerVersions)
      options->RecommendedServerVersions =
        config_lines_dup(options->RecommendedVersions);
    if (options->VersioningAuthoritativeDir &&
        (!options->RecommendedClientVersions ||
         !options->RecommendedServerVersions))
      REJECT("Versioning authoritative dir servers must set "
             "Recommended*Versions.");

    char *t;
    /* Call these functions to produce warnings only. */
    t = format_recommended_version_list(options->RecommendedClientVersions, 1);
    tor_free(t);
    t = format_recommended_version_list(options->RecommendedServerVersions, 1);
    tor_free(t);

    if (options->UseEntryGuards) {
      log_info(LD_CONFIG, "Authoritative directory servers can't set "
               "UseEntryGuards. Disabling.");
      options->UseEntryGuards = 0;
    }
    if (!options->DownloadExtraInfo && authdir_mode_v3(options)) {
      log_info(LD_CONFIG, "Authoritative directories always try to download "
               "extra-info documents. Setting DownloadExtraInfo.");
      options->DownloadExtraInfo = 1;
    }
    if (!(options->BridgeAuthoritativeDir ||
          options->V3AuthoritativeDir))
      REJECT("AuthoritativeDir is set, but none of "
             "(Bridge/V3)AuthoritativeDir is set.");

    /* If we have a v3bandwidthsfile and it's broken, complain on startup */
    if (options->V3BandwidthsFile && !old_options) {
      dirserv_read_measured_bandwidths(options->V3BandwidthsFile, NULL, NULL,
                                       NULL);
    }
    /* same for guardfraction file */
    if (options->GuardfractionFile && !old_options) {
      dirserv_read_guardfraction_file(options->GuardfractionFile, NULL);
    }

    if (!options->DirPort_set)
      REJECT("Running as authoritative directory, but no DirPort set.");

    if (!options->ORPort_set)
      REJECT("Running as authoritative directory, but no ORPort set.");

    if (options->ClientOnly)
      REJECT("Running as authoritative directory, but ClientOnly also set.");
  }

  /* 31851: the tests expect us to validate these options, even when we are
   * not in authority mode. */
  if (options->MinUptimeHidServDirectoryV2 < 0) {
    log_warn(LD_CONFIG, "MinUptimeHidServDirectoryV2 option must be at "
             "least 0 seconds. Changing to 0.");
    options->MinUptimeHidServDirectoryV2 = 0;
  }

  return 0;
}

/**
 * Legacy validation/normalization function for the dirauth bandwidth options
 * in options. Uses old_options as the previous options.
 *
 * Returns 0 on success, returns -1 and sets *msg to a newly allocated string
 * on error.
 */
int
options_validate_dirauth_bandwidth(const or_options_t *old_options,
                                   or_options_t *options,
                                   char **msg)
{
  (void)old_options;

  if (BUG(!options))
    return -1;

  if (BUG(!msg))
    return -1;

  /* 31851: the tests expect us to validate these options, even when we are
   * not in authority mode. */
  if (ensure_bandwidth_cap(&options->AuthDirFastGuarantee,
                           "AuthDirFastGuarantee", msg) < 0)
    return -1;
  if (ensure_bandwidth_cap(&options->AuthDirGuardBWGuarantee,
                           "AuthDirGuardBWGuarantee", msg) < 0)
    return -1;

  return 0;
}

/**
 * Legacy validation/normalization function for the dirauth schedule options
 * in options. Uses old_options as the previous options.
 *
 * Returns 0 on success, returns -1 and sets *msg to a newly allocated string
 * on error.
 */
int
options_validate_dirauth_schedule(const or_options_t *old_options,
                                  or_options_t *options,
                                  char **msg)
{
  (void)old_options;

  if (BUG(!options))
    return -1;

  if (BUG(!msg))
    return -1;

  if (options->V3AuthVoteDelay + options->V3AuthDistDelay >=
      options->V3AuthVotingInterval/2) {
    REJECT("V3AuthVoteDelay plus V3AuthDistDelay must be less than half "
           "V3AuthVotingInterval");
  }

  if (options->V3AuthVoteDelay < MIN_VOTE_SECONDS) {
    if (options->TestingTorNetwork) {
      if (options->V3AuthVoteDelay < MIN_VOTE_SECONDS_TESTING) {
        REJECT("V3AuthVoteDelay is way too low.");
      } else {
        COMPLAIN("V3AuthVoteDelay is very low. "
                 "This may lead to failure to vote for a consensus.");
      }
    } else {
      REJECT("V3AuthVoteDelay is way too low.");
    }
  }

  if (options->V3AuthDistDelay < MIN_DIST_SECONDS) {
    if (options->TestingTorNetwork) {
      if (options->V3AuthDistDelay < MIN_DIST_SECONDS_TESTING) {
        REJECT("V3AuthDistDelay is way too low.");
      } else {
        COMPLAIN("V3AuthDistDelay is very low. "
                 "This may lead to missing votes in a consensus.");
      }
    } else {
      REJECT("V3AuthDistDelay is way too low.");
    }
  }

  if (options->V3AuthNIntervalsValid < 2)
    REJECT("V3AuthNIntervalsValid must be at least 2.");

  if (options->V3AuthVotingInterval < MIN_VOTE_INTERVAL) {
    if (options->TestingTorNetwork) {
      if (options->V3AuthVotingInterval < MIN_VOTE_INTERVAL_TESTING) {
        REJECT("V3AuthVotingInterval is insanely low.");
      } else {
        COMPLAIN("V3AuthVotingInterval is very low. "
                 "This may lead to failure to synchronise for a consensus.");
      }
    } else {
      REJECT("V3AuthVotingInterval is insanely low.");
    }
  } else if (options->V3AuthVotingInterval > 24*60*60) {
    REJECT("V3AuthVotingInterval is insanely high.");
  } else if (((24*60*60) % options->V3AuthVotingInterval) != 0) {
    COMPLAIN("V3AuthVotingInterval does not divide evenly into 24 hours.");
  }

  return 0;
}

/**
 * Legacy validation/normalization function for the dirauth testing options
 * in options. Uses old_options as the previous options.
 *
 * Returns 0 on success, returns -1 and sets *msg to a newly allocated string
 * on error.
 */
int
options_validate_dirauth_testing(const or_options_t *old_options,
                                 or_options_t *options,
                                 char **msg)
{
  (void)old_options;

  if (BUG(!options))
    return -1;

  if (BUG(!msg))
    return -1;

  if (options->TestingV3AuthInitialVotingInterval
      < MIN_VOTE_INTERVAL_TESTING_INITIAL) {
    REJECT("TestingV3AuthInitialVotingInterval is insanely low.");
  } else if (((30*60) % options->TestingV3AuthInitialVotingInterval) != 0) {
    REJECT("TestingV3AuthInitialVotingInterval does not divide evenly into "
           "30 minutes.");
  }

  if (options->TestingV3AuthInitialVoteDelay < MIN_VOTE_SECONDS_TESTING) {
    REJECT("TestingV3AuthInitialVoteDelay is way too low.");
  }

  if (options->TestingV3AuthInitialDistDelay < MIN_DIST_SECONDS_TESTING) {
    REJECT("TestingV3AuthInitialDistDelay is way too low.");
  }

  if (options->TestingV3AuthInitialVoteDelay +
      options->TestingV3AuthInitialDistDelay >=
      options->TestingV3AuthInitialVotingInterval) {
    REJECT("TestingV3AuthInitialVoteDelay plus TestingV3AuthInitialDistDelay "
           "must be less than TestingV3AuthInitialVotingInterval");
  }

  if (options->TestingV3AuthVotingStartOffset >
      MIN(options->TestingV3AuthInitialVotingInterval,
          options->V3AuthVotingInterval)) {
    REJECT("TestingV3AuthVotingStartOffset is higher than the voting "
           "interval.");
  } else if (options->TestingV3AuthVotingStartOffset < 0) {
    REJECT("TestingV3AuthVotingStartOffset must be non-negative.");
  }

  if (options->TestingAuthDirTimeToLearnReachability < 0) {
    REJECT("TestingAuthDirTimeToLearnReachability must be non-negative.");
  } else if (options->TestingAuthDirTimeToLearnReachability > 2*60*60) {
    COMPLAIN("TestingAuthDirTimeToLearnReachability is insanely high.");
  }

  return 0;
}

/**
 * Return true if changing the configuration from <b>old</b> to <b>new</b>
 * affects the timing of the voting subsystem
 */
static int
options_transition_affects_dirauth_timing(const or_options_t *old_options,
                                          const or_options_t *new_options)
{
  tor_assert(old_options);
  tor_assert(new_options);

  if (authdir_mode_v3(old_options) != authdir_mode_v3(new_options))
    return 1;
  if (! authdir_mode_v3(new_options))
    return 0;
  YES_IF_CHANGED_INT(V3AuthVotingInterval);
  YES_IF_CHANGED_INT(V3AuthVoteDelay);
  YES_IF_CHANGED_INT(V3AuthDistDelay);
  YES_IF_CHANGED_INT(TestingV3AuthInitialVotingInterval);
  YES_IF_CHANGED_INT(TestingV3AuthInitialVoteDelay);
  YES_IF_CHANGED_INT(TestingV3AuthInitialDistDelay);
  YES_IF_CHANGED_INT(TestingV3AuthVotingStartOffset);

  return 0;
}

/** Fetch the active option list, and take dirauth actions based on it. All of
 * the things we do should survive being done repeatedly.  If present,
 * <b>old_options</b> contains the previous value of the options.
 *
 * Return 0 if all goes well, return -1 if it's time to die.
 *
 * Note: We haven't moved all the "act on new configuration" logic
 * into the options_act* functions yet.  Some is still in do_hup() and other
 * places.
 */
int
options_act_dirauth(const or_options_t *old_options)
{
  const or_options_t *options = get_options();

  /* We may need to reschedule some dirauth stuff if our status changed. */
  if (old_options) {
    if (options_transition_affects_dirauth_timing(old_options, options)) {
      voting_schedule_recalculate_timing(options, time(NULL));
      reschedule_dirvote(options);
    }
  }

  return 0;
}

/** Fetch the active option list, and take dirauth mtbf actions based on it.
 * All of the things we do should survive being done repeatedly.  If present,
 * <b>old_options</b> contains the previous value of the options.
 *
 * Must be called immediately after a successful or_state_load().
 *
 * Return 0 if all goes well, return -1 if it's time to die.
 *
 * Note: We haven't moved all the "act on new configuration" logic
 * into the options_act* functions yet.  Some is still in do_hup() and other
 * places.
 */
int
options_act_dirauth_mtbf(const or_options_t *old_options)
{
  (void)old_options;

  const or_options_t *options = get_options();
  int running_tor = options->command == CMD_RUN_TOR;

  /* Load dirauth state */
  if (running_tor) {
    rep_hist_load_mtbf_data(time(NULL));
  }

  return 0;
}

/** Fetch the active option list, and take dirauth statistics actions based
 * on it. All of the things we do should survive being done repeatedly. If
 * present, <b>old_options</b> contains the previous value of the options.
 *
 * Sets <b>*print_notice_out</b> if we enabled stats, and need to print
 * a stats log using options_act_relay_stats_msg().
 *
 * Return 0 if all goes well, return -1 if it's time to die.
 *
 * Note: We haven't moved all the "act on new configuration" logic
 * into the options_act* functions yet.  Some is still in do_hup() and other
 * places.
 */
int
options_act_dirauth_stats(const or_options_t *old_options,
                          bool *print_notice_out)
{
  if (BUG(!print_notice_out))
    return -1;

  const or_options_t *options = get_options();

  if (options->BridgeAuthoritativeDir) {
    time_t now = time(NULL);
    int print_notice = 0;

    if ((!old_options || !old_options->BridgeAuthoritativeDir) &&
        options->BridgeAuthoritativeDir) {
      rep_hist_desc_stats_init(now);
      print_notice = 1;
    }
    if (print_notice)
      *print_notice_out = 1;
  }

  /* If we used to have statistics enabled but we just disabled them,
     stop gathering them.  */
  if (old_options && old_options->BridgeAuthoritativeDir &&
      !options->BridgeAuthoritativeDir)
    rep_hist_desc_stats_term();

  return 0;
}
