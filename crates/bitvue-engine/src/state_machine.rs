//! State Machine - Unified state management pattern
//!
//! This module provides a state machine pattern for managing application state,
//! with support for transitions, guards, and actions.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

// =============================================================================
// State Machine Core Types
// =============================================================================

/// State identifier
pub type StateId = String;

/// Event identifier
pub type EventId = String;

/// Transition result
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionResult<S> {
    /// Transition succeeded
    Success(S),
    /// Transition failed with reason
    Failed { current: S, reason: String },
    /// Transition ignored (no matching transition)
    Ignored(S),
}

impl<S> TransitionResult<S> {
    /// Check if transition succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Check if transition failed
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Check if transition was ignored
    pub fn is_ignored(&self) -> bool {
        matches!(self, Self::Ignored(_))
    }

    /// Get the state (current or new)
    pub fn state(&self) -> &S {
        match self {
            Self::Success(s) => s,
            Self::Failed { current: s, .. } => s,
            Self::Ignored(s) => s,
        }
    }
}

/// Guard function - returns true if transition should proceed
pub type GuardFn<S, E> = Arc<dyn Fn(&S, &E) -> bool + Send + Sync>;

/// Action function - executed during transition
pub type ActionFn<S, E> = Arc<dyn Fn(&S, &E, &S) + Send + Sync>;

/// Transition definition
pub struct Transition<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Target state
    pub target: StateId,
    /// Optional guard function
    pub guard: Option<GuardFn<S, E>>,
    /// Optional action to execute on transition
    pub action: Option<ActionFn<S, E>>,
}

impl<S, E> Transition<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Create a new transition
    pub fn new(target: impl Into<StateId>) -> Self {
        Self {
            target: target.into(),
            guard: None,
            action: None,
        }
    }

    /// Add a guard to the transition
    pub fn with_guard(mut self, guard: GuardFn<S, E>) -> Self {
        self.guard = Some(guard);
        self
    }

    /// Add an action to the transition
    pub fn with_action(mut self, action: ActionFn<S, E>) -> Self {
        self.action = Some(action);
        self
    }
}

// =============================================================================
// State Definition
// =============================================================================

/// State definition with entry/exit actions
pub struct State<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// State identifier
    pub id: StateId,
    /// Optional entry action
    pub on_entry: Option<ActionFn<S, E>>,
    /// Optional exit action
    pub on_exit: Option<ActionFn<S, E>>,
    /// Parent state (for hierarchical state machines)
    pub parent: Option<StateId>,
}

impl<S, E> State<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Create a new state
    pub fn new(id: impl Into<StateId>) -> Self {
        Self {
            id: id.into(),
            on_entry: None,
            on_exit: None,
            parent: None,
        }
    }

    /// Add an entry action
    pub fn on_entry(mut self, action: ActionFn<S, E>) -> Self {
        self.on_entry = Some(action);
        self
    }

    /// Add an exit action
    pub fn on_exit(mut self, action: ActionFn<S, E>) -> Self {
        self.on_exit = Some(action);
        self
    }

    /// Set parent state
    pub fn with_parent(mut self, parent: impl Into<StateId>) -> Self {
        self.parent = Some(parent.into());
        self
    }
}

// =============================================================================
// State Machine
// =============================================================================

/// State machine for managing state transitions
pub struct StateMachine<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Current state
    current: S,
    /// State definitions
    states: HashMap<StateId, State<S, E>>,
    /// Transitions: (state, event) -> transition
    transitions: HashMap<(StateId, EventId), Transition<S, E>>,
    /// Current state ID
    current_id: StateId,
}

impl<S, E> StateMachine<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Create a new state machine
    pub fn new(initial: S, initial_id: impl Into<StateId>) -> Self {
        Self {
            current: initial,
            states: HashMap::new(),
            transitions: HashMap::new(),
            current_id: initial_id.into(),
        }
    }

    /// Add a state definition
    pub fn add_state(&mut self, state: State<S, E>) -> &mut Self {
        let id = state.id.clone();
        self.states.insert(id, state);
        self
    }

    /// Add a transition
    pub fn add_transition(
        &mut self,
        from: impl Into<StateId>,
        on: impl Into<EventId>,
        transition: Transition<S, E>,
    ) -> &mut Self {
        let from_id = from.into();
        let event_id = on.into();
        self.transitions.insert((from_id, event_id), transition);
        self
    }

    /// Get the current state
    pub fn current(&self) -> &S {
        &self.current
    }

    /// Get the current state ID
    pub fn current_id(&self) -> &str {
        &self.current_id
    }

    /// Check if a transition is possible
    pub fn can_transition(&self, event: &E) -> bool {
        let event_id = self.event_id(event);
        let key = (self.current_id.clone(), event_id);
        self.transitions.contains_key(&key)
    }

    /// Process an event and attempt transition
    pub fn process(&mut self, event: E) -> TransitionResult<S> {
        let event_id = self.event_id(&event);

        // Find matching transition
        let transition = match self.transitions.get(&(self.current_id.clone(), event_id)) {
            Some(t) => t,
            None => return TransitionResult::Ignored(self.current.clone()),
        };

        // Check guard
        if let Some(guard) = &transition.guard {
            if !guard(&self.current, &event) {
                return TransitionResult::Failed {
                    current: self.current.clone(),
                    reason: "Guard condition failed".to_string(),
                };
            }
        }

        // Get target state definition
        let target_state = match self.states.get(&transition.target) {
            Some(s) => s,
            None => {
                return TransitionResult::Failed {
                    current: self.current.clone(),
                    reason: format!("Target state '{}' not found", transition.target),
                };
            }
        };

        // Execute exit action of current state
        if let Some(current_state) = self.states.get(&self.current_id) {
            if let Some(exit_action) = &current_state.on_exit {
                exit_action(&self.current, &event, &self.current);
            }
        }

        // Store old state for action
        let old_state = self.current.clone();
        let _old_id = self.current_id.clone();

        // Update current state (caller will update the actual state value)
        self.current_id = transition.target.clone();

        // Execute transition action
        if let Some(action) = &transition.action {
            action(&old_state, &event, &self.current);
        }

        // Execute entry action of new state
        if let Some(entry_action) = &target_state.on_entry {
            entry_action(&old_state, &event, &self.current);
        }

        TransitionResult::Success(self.current.clone())
    }

    /// Process an event and update state with a function
    pub fn process_with<F>(&mut self, event: E, state_fn: F) -> TransitionResult<S>
    where
        F: FnOnce(&S, &E, &StateId) -> S,
    {
        let old_state = self.current.clone();
        let event_clone = event.clone();
        let result = self.process(event);

        match result {
            TransitionResult::Success(_) => {
                // Update state with the provided function
                self.current = state_fn(&old_state, &event_clone, &self.current_id);
                TransitionResult::Success(self.current.clone())
            }
            _ => result,
        }
    }

    /// Get event ID from event (default implementation uses type name)
    fn event_id(&self, _event: &E) -> EventId {
        std::any::type_name::<E>().to_string()
    }

    /// Get all state IDs
    pub fn state_ids(&self) -> Vec<StateId> {
        self.states.keys().cloned().collect()
    }
}

impl<S, E> fmt::Debug for StateMachine<S, E>
where
    S: Clone + Send + Sync + fmt::Debug + 'static,
    E: Clone + Send + Sync + fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateMachine")
            .field("current_id", &self.current_id)
            .field("current", &self.current)
            .field("state_count", &self.states.len())
            .field("transition_count", &self.transitions.len())
            .finish()
    }
}

// =============================================================================
// Builder for State Machine
// =============================================================================

/// Builder for creating state machines
pub struct StateMachineBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    initial: S,
    initial_id: StateId,
    states: Vec<State<S, E>>,
    transitions: Vec<(StateId, EventId, Transition<S, E>)>,
}

impl<S, E> StateMachineBuilder<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Create a new builder
    pub fn new(initial: S, initial_id: impl Into<StateId>) -> Self {
        Self {
            initial,
            initial_id: initial_id.into(),
            states: Vec::new(),
            transitions: Vec::new(),
        }
    }

    /// Add a state
    pub fn state(mut self, state: State<S, E>) -> Self {
        self.states.push(state);
        self
    }

    /// Add a transition
    pub fn transition(
        mut self,
        from: impl Into<StateId>,
        on: impl Into<EventId>,
        to: impl Into<StateId>,
    ) -> Self {
        self.transitions
            .push((from.into(), on.into(), Transition::new(to.into())));
        self
    }

    /// Add a transition with guard
    pub fn transition_with_guard(
        mut self,
        from: impl Into<StateId>,
        on: impl Into<EventId>,
        to: impl Into<StateId>,
        guard: GuardFn<S, E>,
    ) -> Self {
        let mut t = Transition::new(to.into());
        t.guard = Some(guard);
        self.transitions.push((from.into(), on.into(), t));
        self
    }

    /// Build the state machine
    pub fn build(self) -> StateMachine<S, E> {
        let mut sm = StateMachine::new(self.initial, self.initial_id);

        for state in self.states {
            sm.add_state(state);
        }

        for (from, on, transition) in self.transitions {
            sm.add_transition(from, on, transition);
        }

        sm
    }
}

// =============================================================================
// Example Application States
// =============================================================================

/// Application state example
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Idle,
    Loading,
    Ready,
    Error,
}

/// Application event example
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    Start,
    LoadComplete,
    LoadFailed,
    Reset,
}

/// Example state machine for application lifecycle
pub type AppStateMachine = StateMachine<AppState, AppEvent>;

/// Create an application state machine
pub fn create_app_state_machine() -> AppStateMachine {
    StateMachineBuilder::new(AppState::Idle, "idle")
        // States
        .state(State::new("idle"))
        .state(State::new("loading"))
        .state(State::new("ready"))
        .state(State::new("error"))
        // Transitions
        .transition("idle", "Start", "loading")
        .transition("loading", "LoadComplete", "ready")
        .transition("loading", "LoadFailed", "error")
        .transition("error", "Reset", "idle")
        .transition("ready", "Start", "loading")
        .build()
}

// =============================================================================
// Player State Machine Example
// =============================================================================

/// Player state example
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
    Seeking,
    Buffering,
}

/// Player event example
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerEvent {
    Play,
    Pause,
    Stop,
    Seek,
    SeekComplete,
    BufferStart,
    BufferEnd,
}

/// Create a player state machine
pub fn create_player_state_machine() -> StateMachine<PlayerState, PlayerEvent> {
    StateMachineBuilder::new(PlayerState::Stopped, "stopped")
        .state(State::new("stopped"))
        .state(State::new("playing"))
        .state(State::new("paused"))
        .state(State::new("seeking"))
        .state(State::new("buffering"))
        // Transitions
        .transition("stopped", "Play", "playing")
        .transition("stopped", "Seek", "seeking")
        .transition("playing", "Pause", "paused")
        .transition("playing", "Stop", "stopped")
        .transition("playing", "Seek", "seeking")
        .transition("playing", "BufferStart", "buffering")
        .transition("paused", "Play", "playing")
        .transition("paused", "Stop", "stopped")
        .transition("paused", "Seek", "seeking")
        .transition("seeking", "SeekComplete", "playing")
        .transition("seeking", "BufferStart", "buffering")
        .transition("buffering", "BufferEnd", "playing")
        .build()
}

// =============================================================================
// Workspace State Machine Example
// =============================================================================

/// Workspace state example
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceState {
    Empty,
    SingleStream,
    DualStream,
    Compare,
}

/// Workspace event example
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceEvent {
    OpenFirstStream,
    OpenSecondStream,
    SwitchToSingle,
    SwitchToDual,
    SwitchToCompare,
    CloseStream,
}

/// Create a workspace state machine
pub fn create_workspace_state_machine() -> StateMachine<WorkspaceState, WorkspaceEvent> {
    StateMachineBuilder::new(WorkspaceState::Empty, "empty")
        .state(State::new("empty"))
        .state(State::new("single"))
        .state(State::new("dual"))
        .state(State::new("compare"))
        // Transitions
        .transition("empty", "OpenFirstStream", "single")
        .transition("single", "SwitchToDual", "dual")
        .transition("single", "CloseStream", "empty")
        .transition("dual", "SwitchToSingle", "single")
        .transition("dual", "SwitchToCompare", "compare")
        .transition("dual", "CloseStream", "single")
        .transition("compare", "SwitchToDual", "dual")
        .transition("compare", "SwitchToSingle", "single")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_creation() {
        let sm: StateMachine<&str, String> = StateMachine::new("idle", "idle");
        assert_eq!(sm.current(), &"idle");
        assert_eq!(sm.current_id(), "idle");
    }

    #[test]
    fn test_state_machine_add_state() {
        let mut sm: StateMachine<&str, String> = StateMachine::new("idle", "idle");
        sm.add_state(State::new("idle"));
        sm.add_state(State::new("loading"));

        assert_eq!(sm.state_ids().len(), 2);
    }

    #[test]
    fn test_state_machine_add_transition() {
        let mut sm = StateMachine::new("idle", "idle");
        sm.add_state(State::new("idle"));
        sm.add_state(State::new("loading"));
        sm.add_transition("idle", "Start", Transition::new("loading"));

        let event = AppEvent::Start; // This won't match due to event_id
        let result = sm.process(event);
        // Should be ignored since event type doesn't match
        assert!(result.is_ignored());
    }

    #[test]
    fn test_app_state_machine() {
        let sm = create_app_state_machine();
        assert_eq!(sm.current(), &AppState::Idle);
        assert_eq!(sm.current_id(), "idle");
    }

    #[test]
    fn test_player_state_machine() {
        let sm = create_player_state_machine();
        assert_eq!(sm.current(), &PlayerState::Stopped);
        assert_eq!(sm.current_id(), "stopped");
    }

    #[test]
    fn test_workspace_state_machine() {
        let sm = create_workspace_state_machine();
        assert_eq!(sm.current(), &WorkspaceState::Empty);
        assert_eq!(sm.current_id(), "empty");
    }

    #[test]
    fn test_transition_result_success() {
        let result: TransitionResult<&str> = TransitionResult::Success("new_state");
        assert!(result.is_success());
        assert!(!result.is_failed());
        assert!(!result.is_ignored());
        assert_eq!(result.state(), &"new_state");
    }

    #[test]
    fn test_transition_result_failed() {
        let result: TransitionResult<&str> = TransitionResult::Failed {
            current: "old_state",
            reason: "Guard failed".to_string(),
        };
        assert!(!result.is_success());
        assert!(result.is_failed());
        assert!(!result.is_ignored());
        assert_eq!(result.state(), &"old_state");
    }

    #[test]
    fn test_transition_result_ignored() {
        let result: TransitionResult<&str> = TransitionResult::Ignored("current_state");
        assert!(!result.is_success());
        assert!(!result.is_failed());
        assert!(result.is_ignored());
        assert_eq!(result.state(), &"current_state");
    }

    #[test]
    fn test_state_with_entry_exit() {
        let state = State::<&str, &str>::new("test")
            .on_entry(Arc::new(|_, _, _| {}))
            .on_exit(Arc::new(|_, _, _| {}));

        assert_eq!(state.id, "test");
        assert!(state.on_entry.is_some());
        assert!(state.on_exit.is_some());
    }

    #[test]
    fn test_state_with_parent() {
        let state = State::<&str, &str>::new("child").with_parent("parent");
        assert_eq!(state.parent, Some("parent".to_string()));
    }

    #[test]
    fn test_state_machine_debug() {
        let sm: StateMachine<&str, String> = StateMachine::new("test", "test");
        let debug_str = format!("{:?}", sm);
        assert!(debug_str.contains("StateMachine"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_builder_pattern() {
        let sm: StateMachine<&str, String> = StateMachineBuilder::new("initial", "initial")
            .state(State::new("initial"))
            .state(State::new("next"))
            .transition("initial", "Go", "next")
            .build();

        assert_eq!(sm.current(), &"initial");
        assert_eq!(sm.state_ids().len(), 2);
    }
}
