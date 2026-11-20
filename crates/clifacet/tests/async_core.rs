#![cfg(feature = "async")]
#![allow(
    clippy::pedantic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value,
    clippy::unused_self,
    clippy::unused_async_trait_impl,
    clippy::manual_let_else
)]

use std::cell::RefCell;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use clifacet::r#async::{
    AsyncCommand, AsyncHandler, AsyncPlan, AsyncPlanned, AsyncPlannedWith, Cancel,
    LocalAsyncHandler, ProgressEvent, RunCx,
};
use clifacet::failure::{Failure, FailureKind};

/// Minimal single-step poll helper for testing futures without external runtimes.
fn poll_ready<F: Future>(fut: F) -> Poll<F::Output> {
    let mut pinned = pin!(fut);
    pinned
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
}

// ---------------------------------------------------------------------------
// Context and Command definitions
// ---------------------------------------------------------------------------

#[derive(Default)]
struct TestDatabase {
    records: Mutex<Vec<String>>,
}

struct InsertRecord;

impl AsyncHandler<TestDatabase> for InsertRecord {
    type Input = String;
    type Output = usize;

    async fn handle(
        &self,
        input: Self::Input,
        db: &TestDatabase,
        run: &RunCx,
    ) -> Result<Self::Output, Failure> {
        if run.is_cancelled() {
            return Err(Failure::cancelled("operation cancelled before insert"));
        }

        run.message("validating record");
        if input.is_empty() {
            return Err(Failure::validation("record cannot be empty"));
        }

        run.message("inserting into database");
        let mut guard = db.records.lock().expect("db lock poisoned");
        guard.push(input);
        run.advance(1);

        Ok(guard.len())
    }
}

struct ComputeChecksum;

impl AsyncCommand for ComputeChecksum {
    type Input = Vec<u8>;
    type Output = u64;

    async fn execute(&self, input: Self::Input, run: &RunCx) -> Result<Self::Output, Failure> {
        run.report_total(Some(input.len() as u64));
        let mut sum = 0u64;
        for (idx, byte) in input.iter().enumerate() {
            if run.is_cancelled() {
                return Err(Failure::cancelled(format!(
                    "checksum calculation aborted at byte {idx}"
                )));
            }
            sum = sum.wrapping_add(u64::from(*byte));
            run.advance(1);
        }
        run.message("checksum complete");
        Ok(sum)
    }
}

struct ServiceMigration;

impl AsyncPlanned for ServiceMigration {
    type Input = String;
    type Output = String;

    async fn plan(&self, service: Self::Input) -> Result<AsyncPlan<'_, Self::Output>, Failure> {
        if service == "forbidden" {
            return Err(Failure::validation("migration forbidden for this service"));
        }

        let step1 = format!("backup schema for {service}");
        let step2 = format!("apply v2 migrations for {service}");
        let step3 = format!("verify health of {service}");

        let plan =
            AsyncPlan::new(move || async move { Ok(format!("migration completed for {service}")) })
                .step(step1)
                .step(step2)
                .step(step3);

        Ok(plan)
    }
}

struct ContextualMigration;

impl AsyncPlannedWith<TestDatabase> for ContextualMigration {
    type Input = String;
    type Output = usize;

    async fn plan<'a>(
        &'a self,
        table: Self::Input,
        db: &'a TestDatabase,
    ) -> Result<AsyncPlan<'a, Self::Output>, Failure> {
        let step = format!("migrate table {table}");
        let plan = AsyncPlan::new(move || async move {
            let mut guard = db.records.lock().expect("db lock");
            guard.push(format!("migrated:{table}"));
            Ok(guard.len())
        })
        .step(step);

        Ok(plan)
    }
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

#[test]
fn async_handler_executes_with_context_and_reports_progress() {
    let db = TestDatabase::default();
    let handler = InsertRecord;
    let (run_cx, sink) = RunCx::for_test();

    let poll = poll_ready(handler.handle("record-1".to_string(), &db, &run_cx));
    match poll {
        Poll::Ready(Ok(count)) => assert_eq!(count, 1),
        other => panic!("expected Poll::Ready(Ok(1)), got {other:?}"),
    }

    assert_eq!(sink.current(), 1);
    assert_eq!(
        sink.last_message().as_deref(),
        Some("inserting into database")
    );
    assert_eq!(
        sink.events(),
        vec![
            ProgressEvent::Message("validating record".to_string()),
            ProgressEvent::Message("inserting into database".to_string()),
            ProgressEvent::Advance(1),
        ]
    );

    let second_poll = poll_ready(handler.handle("record-2".to_string(), &db, &run_cx));
    match second_poll {
        Poll::Ready(Ok(count)) => assert_eq!(count, 2),
        other => panic!("expected Poll::Ready(Ok(2)), got {other:?}"),
    }
}

#[test]
fn async_handler_fails_fast_on_validation() {
    let db = TestDatabase::default();
    let handler = InsertRecord;
    let (run_cx, _) = RunCx::for_test();

    let poll = poll_ready(handler.handle(String::new(), &db, &run_cx));
    match poll {
        Poll::Ready(Err(failure)) => {
            assert_eq!(failure.kind(), FailureKind::Validation);
            assert!(failure.message().contains("cannot be empty"));
        }
        other => panic!("expected validation failure, got {other:?}"),
    }
}

#[test]
fn async_command_blanket_implements_async_handler() {
    let cmd = ComputeChecksum;
    let (run_cx, sink) = RunCx::for_test();

    // Call as AsyncCommand directly
    let poll1 = poll_ready(cmd.execute(vec![10, 20, 30], &run_cx));
    match poll1 {
        Poll::Ready(Ok(sum)) => assert_eq!(sum, 60),
        other => panic!("expected ready sum 60, got {other:?}"),
    }
    assert_eq!(sink.total(), Some(3));
    assert_eq!(sink.current(), 3);

    // Call through blanket AsyncHandler<Cx> implementation
    sink.clear();
    let poll2 = poll_ready(AsyncHandler::<()>::handle(
        &cmd,
        vec![1, 2, 3, 4],
        &(),
        &run_cx,
    ));
    match poll2 {
        Poll::Ready(Ok(sum)) => assert_eq!(sum, 10),
        other => panic!("expected ready sum 10, got {other:?}"),
    }
    assert_eq!(sink.total(), Some(4));
    assert_eq!(sink.current(), 4);
}

#[test]
fn cooperative_cancellation_aborts_and_preserves_partial_state() {
    let cmd = ComputeChecksum;
    let (run_cx, sink) = RunCx::for_test();

    // Trip cancellation before executing
    run_cx.request_cancel();
    assert!(run_cx.is_cancelled());

    let poll = poll_ready(cmd.execute(vec![1, 2, 3], &run_cx));
    match poll {
        Poll::Ready(Err(failure)) => {
            assert_eq!(failure.kind(), FailureKind::Cancelled);
            assert!(failure.message().contains("aborted at byte 0"));
        }
        other => panic!("expected cancelled failure, got {other:?}"),
    }

    assert_eq!(sink.current(), 0);
}

#[test]
fn local_async_handler_supports_unsent_types() {
    struct UnsentState {
        log: Rc<RefCell<Vec<String>>>,
    }

    struct UnsentHandler {
        state: UnsentState,
    }

    impl LocalAsyncHandler<()> for UnsentHandler {
        type Input = String;
        type Output = usize;

        async fn handle_local(
            &self,
            input: Self::Input,
            _cx: &(),
            _run: &RunCx,
        ) -> Result<Self::Output, Failure> {
            self.state.log.borrow_mut().push(input);
            Ok(self.state.log.borrow().len())
        }
    }

    let shared_log = Rc::new(RefCell::new(Vec::new()));
    let handler = UnsentHandler {
        state: UnsentState {
            log: shared_log.clone(),
        },
    };
    let run_cx = RunCx::empty();

    let poll = poll_ready(handler.handle_local("local-step-1".to_string(), &(), &run_cx));
    match poll {
        Poll::Ready(Ok(len)) => assert_eq!(len, 1),
        other => panic!("expected ready 1, got {other:?}"),
    }
    assert_eq!(*shared_log.borrow(), vec!["local-step-1"]);
}

#[test]
fn async_planned_describes_steps_without_executing_work() {
    let migration = ServiceMigration;

    let plan_poll = poll_ready(migration.plan("payment-gateway".to_string()));
    let plan = match plan_poll {
        Poll::Ready(Ok(plan)) => plan,
        other => panic!("expected ready plan, got {other:?}"),
    };

    assert_eq!(
        plan.steps_slice(),
        &[
            "backup schema for payment-gateway",
            "apply v2 migrations for payment-gateway",
            "verify health of payment-gateway",
        ]
    );
    assert_eq!(
        plan.describe(),
        "backup schema for payment-gateway\napply v2 migrations for payment-gateway\nverify health of payment-gateway"
    );

    // Map output and execute
    let mapped = plan.map(|result| format!("STATUS: {result}"));
    let exec_poll = poll_ready(mapped.execute());
    match exec_poll {
        Poll::Ready(Ok(msg)) => {
            assert_eq!(msg, "STATUS: migration completed for payment-gateway");
        }
        other => panic!("expected execution result, got {other:?}"),
    }
}

#[test]
fn async_planned_executes_as_async_command() {
    let migration = ServiceMigration;
    let run_cx = RunCx::empty();

    // AsyncPlanned blanket implements AsyncCommand
    let exec_poll = poll_ready(migration.execute("billing-api".to_string(), &run_cx));
    match exec_poll {
        Poll::Ready(Ok(msg)) => assert_eq!(msg, "migration completed for billing-api"),
        other => panic!("expected ready message, got {other:?}"),
    }
}

#[test]
fn async_planned_with_context_builds_and_executes_plan() {
    let db = TestDatabase::default();
    let migration = ContextualMigration;

    let plan_poll = poll_ready(migration.plan("users".to_string(), &db));
    let plan = match plan_poll {
        Poll::Ready(Ok(p)) => p,
        other => panic!("expected ready plan, got {other:?}"),
    };

    assert_eq!(plan.describe(), "migrate table users");
    assert!(db.records.lock().unwrap().is_empty());

    let exec_poll = poll_ready(plan.execute());
    match exec_poll {
        Poll::Ready(Ok(count)) => assert_eq!(count, 1),
        other => panic!("expected ready 1, got {other:?}"),
    }
    assert_eq!(*db.records.lock().unwrap(), vec!["migrated:users"]);
}

#[test]
fn cancel_token_waking_and_thread_safety() {
    let cancel = Cancel::new();
    assert!(!cancel.is_cancelled());
    assert_eq!(format!("{cancel}"), "active");

    let cancel_worker = cancel.clone();
    let woke = Arc::new(AtomicBool::new(false));
    let woke_clone = woke.clone();

    // Custom waker to verify waking
    struct FlagWaker(Arc<AtomicBool>);
    impl std::task::Wake for FlagWaker {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let waker = Waker::from(Arc::new(FlagWaker(woke_clone)));
    let mut fut = cancel_worker.cancelled();

    // Poll while uncancelled -> should register waker and return Pending
    let mut pinned_fut = pin!(&mut fut);
    let poll1 = pinned_fut.as_mut().poll(&mut Context::from_waker(&waker));
    assert!(poll1.is_pending());
    assert!(!woke.load(Ordering::SeqCst));

    // Cancel from another handle
    cancel.cancel();
    assert!(cancel.is_cancelled());
    assert_eq!(format!("{cancel}"), "cancelled");
    assert!(woke.load(Ordering::SeqCst));

    // Second poll should now be Ready
    let poll2 = pinned_fut.as_mut().poll(&mut Context::from_waker(&waker));
    assert!(poll2.is_ready());

    // Subsequent cancellations are idempotent
    cancel.cancel();
    assert!(cancel.is_cancelled());
}
