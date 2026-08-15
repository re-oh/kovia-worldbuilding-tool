use bevy::prelude::*;

/// Implemented beside the domain data it changes. History stores these
/// actions without knowing whether they belong to terrain, layers, features,
/// or a future plugin.
pub trait UndoAction: Send + Sync + 'static {
    fn undo(&self, world: &mut World) -> Result<(), String>;
    fn redo(&self, world: &mut World) -> Result<(), String>;
}

#[derive(Default, Resource)]
pub struct EditHistory {
    undo: Vec<Box<dyn UndoAction>>,
    redo: Vec<Box<dyn UndoAction>>,
}

impl EditHistory {
    pub fn record(&mut self, action: impl UndoAction) {
        self.undo.push(Box::new(action));
        self.redo.clear();
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct Undo;

#[derive(Debug, Clone, Copy, Event)]
pub struct Redo;

#[derive(Debug, Clone, Event)]
pub enum HistoryOutcome {
    Undid,
    Redid,
    NothingToUndo,
    NothingToRedo,
    Failed(String),
}

/// Generic chronology only. It imports no terrain, layer, feature, editor, or
/// renderer type. Domain plugins optionally submit their own `UndoAction`s.
pub struct HistoryPlugin;

impl Plugin for HistoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditHistory>()
            .add_observer(request_undo)
            .add_observer(request_redo);
    }
}

fn request_undo(_request: On<Undo>, mut commands: Commands) {
    commands.queue(ApplyHistoryStep::Undo);
}

fn request_redo(_request: On<Redo>, mut commands: Commands) {
    commands.queue(ApplyHistoryStep::Redo);
}

#[derive(Clone, Copy)]
enum ApplyHistoryStep {
    Undo,
    Redo,
}

impl Command for ApplyHistoryStep {
    type Out = ();

    fn apply(self, world: &mut World) {
        let mut history = world.remove_resource::<EditHistory>().unwrap_or_default();
        let (action, empty_outcome) = match self {
            Self::Undo => (history.undo.pop(), HistoryOutcome::NothingToUndo),
            Self::Redo => (history.redo.pop(), HistoryOutcome::NothingToRedo),
        };
        let Some(action) = action else {
            world.insert_resource(history);
            world.trigger(empty_outcome);
            return;
        };

        let result = match self {
            Self::Undo => action.undo(world),
            Self::Redo => action.redo(world),
        };
        let outcome = match (self, result) {
            (Self::Undo, Ok(())) => {
                history.redo.push(action);
                HistoryOutcome::Undid
            }
            (Self::Redo, Ok(())) => {
                history.undo.push(action);
                HistoryOutcome::Redid
            }
            (Self::Undo, Err(error)) => {
                history.undo.push(action);
                HistoryOutcome::Failed(error)
            }
            (Self::Redo, Err(error)) => {
                history.redo.push(action);
                HistoryOutcome::Failed(error)
            }
        };
        world.insert_resource(history);
        world.trigger(outcome);
    }
}
