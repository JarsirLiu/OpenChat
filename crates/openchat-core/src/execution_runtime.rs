use std::{future::Future, pin::Pin};

use crate::{turn_control::ActiveTurnHandle, SessionRuntime, TurnPlan};

pub type TurnExecutionFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub trait TurnExecution: Send + Sync {
    fn run_turn(
        &self,
        plan: TurnPlan,
        session_runtime: SessionRuntime,
        active_turn: ActiveTurnHandle,
    ) -> TurnExecutionFuture;
}
