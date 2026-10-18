//! Caller gates (`docs/BATCH.md` §3.12, "not via MCP").
//!
//! Two steps reach outside the batch: `script.external` (runs a program) and `script.code` with
//! `allowIo: true` (JSF without the sandbox). The CLI and the UI keep the opt-ins in the step; an
//! **MCP caller** (an agent) may never run them, whatever the step says.
//!
//! The caller is known in two ways:
//! - [`with_caller`] marks the current thread for the duration of a call (fp-mcp wraps every tool
//!   call, so `batch`, `run_command batch.run`, `run_script` → `fp.run("batch.run")` and the bridge
//!   into the UI are all covered). [`Recipe::validate`](crate::Recipe::validate),
//!   [`pipeline::preview`](crate::pipeline::preview) and [`executor::run`](crate::executor::run)
//!   read it.
//! - [`RunOptions::caller`](crate::RunOptions::caller) carries it into background jobs
//!   ([`executor::spawn`](crate::executor::spawn) runs on another thread).

use crate::error::{BatchError, Result};
use crate::recipe::Recipe;
use crate::step::{Issue, Severity};
use crate::steps::script_extra::{SCRIPT_CODE, SCRIPT_EXTERNAL};
use serde_json::Value;
use std::cell::Cell;

/// Who asked for the batch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Caller {
    /// UI, CLI, JSF of the user: the opt-ins in the steps apply.
    #[default]
    Local,
    /// An agent over MCP: `script.external` and `script.code` with `allowIo` are refused.
    Mcp,
}

thread_local! {
    static CALLER: Cell<Caller> = const { Cell::new(Caller::Local) };
}

/// Runs `f` with the current thread marked as `caller` (restored afterwards, also on panic).
pub fn with_caller<R>(caller: Caller, f: impl FnOnce() -> R) -> R {
    struct Reset(Caller);
    impl Drop for Reset {
        fn drop(&mut self) {
            CALLER.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(CALLER.with(|c| c.replace(caller)));
    f()
}

/// Caller of the current thread.
pub fn current_caller() -> Caller {
    CALLER.with(Cell::get)
}

/// The stricter of `caller` and the current thread's caller.
pub fn effective(caller: Caller) -> Caller {
    if caller == Caller::Mcp || current_caller() == Caller::Mcp {
        Caller::Mcp
    } else {
        Caller::Local
    }
}

/// Is the enabled step `id` with `params` refused for `caller`? Returns the message key.
fn refused(id: &str, params: &Value, caller: Caller) -> Option<&'static str> {
    if caller != Caller::Mcp {
        return None;
    }
    match id {
        SCRIPT_EXTERNAL => Some("batch-issue-mcp-external"),
        SCRIPT_CODE
            if params
                .get("allowIo")
                .and_then(Value::as_bool)
                .unwrap_or(false) =>
        {
            Some("batch-issue-mcp-allow-io")
        }
        _ => None,
    }
}

/// Validation issues (errors) for the steps `caller` may not run.
pub fn caller_issues(recipe: &Recipe, caller: Caller) -> Vec<Issue> {
    recipe
        .enabled_steps()
        .filter_map(|(i, s)| {
            refused(&s.id, &s.params, caller).map(|key| {
                Issue::step(
                    i,
                    &s.id,
                    Severity::Error,
                    "refusedForMcp",
                    fp_i18n::tr!(key, index = (i + 1) as u64),
                )
            })
        })
        .collect()
}

/// Error listing every offending step when `caller` may not run `recipe`.
pub fn check(recipe: &Recipe, caller: Caller) -> Result<()> {
    let issues = caller_issues(recipe, caller);
    if issues.is_empty() {
        return Ok(());
    }
    let steps: Vec<String> = issues.into_iter().map(|i| i.message).collect();
    Err(BatchError::Invalid(fp_i18n::tr!(
        "batch-err-mcp-refused",
        steps = steps.join("; ")
    )))
}
