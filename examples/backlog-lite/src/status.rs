//! Where a task can be, spelled every way a person types it.

use std::fmt;

/// Where a task can be in this project's configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Not started.
    ToDo,
    /// Being worked on.
    InProgress,
    /// Finished.
    Done,
}

impl Status {
    /// Every status, in the order the tool lists them.
    pub const ALL: [Self; 3] = [Self::ToDo, Self::InProgress, Self::Done];

    /// The spelling Backlog.md itself accepts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ToDo => "To Do",
            Self::InProgress => "In Progress",
            Self::Done => "Done",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for Status {
    type Err = String;

    /// Accepts `todo`, `to-do`, `To Do` and `inprogress` alike, and says what it knows.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalised: String = value
            .chars()
            .filter(|character| character.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
        match normalised.as_str() {
            "todo" => Ok(Self::ToDo),
            "inprogress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            _ => Err(format!(
                "`{value}` is not a status; expected one of: To Do, In Progress, Done"
            )),
        }
    }
}

/// The `value_parser` the leaves that take a status share.
pub(crate) fn parse_status(value: &str) -> Result<Status, String> {
    value.parse()
}
