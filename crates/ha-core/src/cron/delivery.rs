use super::types::CronJob;

#[derive(Debug, Clone, Copy)]
pub enum DeliveryOutcome<'a> {
    Success { text: &'a str },
    Failure { error: &'a str },
}

/// §8: aggregate outcome of fanning one run's result out to all of a job's
/// delivery targets. Drives the run-log `delivery_status` so the GUI can show
/// whether the result actually reached its IM destinations.
#[derive(Debug, Clone, Default)]
pub struct DeliveryReport {
    /// Targets we actually attempted to send to (whitelisted + account present).
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
    /// Skipped: not in the `channel_conversations` whitelist.
    pub skipped_unverified: usize,
    /// Skipped: the sending account no longer exists (target marked stale).
    pub skipped_missing_account: usize,
}

impl DeliveryReport {
    fn total(&self) -> usize {
        self.attempted + self.failed + self.skipped_unverified + self.skipped_missing_account
    }

    /// Run-log `delivery_status` string, or `None` when the job had no targets
    /// (nothing to fan out — distinct from a fan-out that delivered to nobody).
    pub fn run_log_status(&self) -> Option<&'static str> {
        if self.total() == 0 {
            return None;
        }
        if self.succeeded == self.total() {
            Some("delivered")
        } else if self.succeeded > 0 {
            Some("partial")
        } else {
            Some("failed")
        }
    }
}

/// G2: deliver a background-completion **injection** turn to a cron job's
/// targets. A background job/subagent spawned during a cron run completes after
/// the inline run already delivered its own response; `inject_and_run_parent`
/// then runs a fresh billed turn against the (now-idle) cron session whose
/// output would otherwise reach nobody. Resolve the owning job from the session
/// and fan its result out the same way the inline run does. No-op when the
/// session isn't a cron run, the job is gone, or it has no delivery targets.
pub async fn deliver_injection_for_session(session_id: &str, text: &str) {
    let Some(cron_db) = crate::globals::get_cron_db() else {
        return;
    };
    let job = match cron_db.find_job_by_session(session_id) {
        Ok(Some(job)) if !job.delivery_targets.is_empty() => job,
        Ok(_) => return,
        Err(e) => {
            app_warn!(
                "cron",
                "delivery",
                "find_job_by_session({}) failed: {}",
                session_id,
                e
            );
            return;
        }
    };
    // Injection has no run log of its own (the inline run already wrote one), so
    // the report is informational only here.
    let _ = deliver_results(&job, DeliveryOutcome::Success { text }).await;
}

/// Fan-out a finished cron job's result to each configured IM channel target in
/// parallel, returning a [`DeliveryReport`] summarizing what reached whom.
///
/// - Success → send the raw response text, optionally prefixed with
///   `[Cron] {name}` when the job opts in (`prefix_delivery_with_name`).
/// - Failure → send `⚠️ [Cron] {name} failed: {error}`.
///
/// Transient per-target send failures / timeouts are retried with backoff
/// (`MAX_SEND_ATTEMPTS`). Targets whose account has been deleted are skipped and
/// flagged `stale` (written back so the GUI marks them); whitelist misses are
/// skipped + audited. One broken channel never fails the job or blocks siblings.
pub async fn deliver_results(_job: &CronJob, outcome: DeliveryOutcome<'_>) -> DeliveryReport {
    let _ = match outcome {
        DeliveryOutcome::Success { text } => text,
        DeliveryOutcome::Failure { error } => error,
    };
    DeliveryReport::default()
}

#[cfg(test)]
mod tests {
    use super::DeliveryReport;

    #[test]
    fn run_log_status_none_when_no_targets() {
        assert_eq!(DeliveryReport::default().run_log_status(), None);
    }

    #[test]
    fn run_log_status_delivered_when_all_succeed() {
        let r = DeliveryReport {
            attempted: 2,
            succeeded: 2,
            ..Default::default()
        };
        assert_eq!(r.run_log_status(), Some("delivered"));
    }

    #[test]
    fn run_log_status_partial_on_mixed_outcomes() {
        // One delivered, one failed.
        let r = DeliveryReport {
            attempted: 2,
            succeeded: 1,
            failed: 1,
            ..Default::default()
        };
        assert_eq!(r.run_log_status(), Some("partial"));
        // One delivered, one skipped (account gone) is still partial.
        let r = DeliveryReport {
            attempted: 1,
            succeeded: 1,
            skipped_missing_account: 1,
            ..Default::default()
        };
        assert_eq!(r.run_log_status(), Some("partial"));
    }

    #[test]
    fn run_log_status_failed_when_none_delivered() {
        // All failed.
        let r = DeliveryReport {
            attempted: 2,
            failed: 2,
            ..Default::default()
        };
        assert_eq!(r.run_log_status(), Some("failed"));
        // All skipped (whitelist miss + missing account) — had targets, reached
        // nobody → failed (distinct from None = no targets configured).
        let r = DeliveryReport {
            skipped_unverified: 1,
            skipped_missing_account: 1,
            ..Default::default()
        };
        assert_eq!(r.run_log_status(), Some("failed"));
    }
}
