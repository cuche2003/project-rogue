use crate::Direction;

/// Player intent sent by a frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Move(Direction),
    Wait,
    Descend,
    Restart,
}

/// Why a command was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    GameOver,
    Blocked,
    InvalidTarget,
    NoStairs,
}

/// Result of dispatching one command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    Applied,
    Rejected(CommandError),
}
