use std::fmt::Display;
use std::sync::Arc;
use crate::prelude::Entity;
use crate::plugins::ThunkContext;

/// Crate level error implemenation
/// 
#[derive(Debug, Clone)]
pub struct Error {
    /// Error category,
    /// 
    category: Category,
}

impl Error {
    /// Returns an error that indicates this error is recoverable,
    /// 
    pub fn recoverable() -> Self {
        Error { category: Category::Recoverable(None) }
    }

    /// Returns an error that indicates this error can 
    /// 
    pub fn recoverable_with(context: ThunkContext, recoverer: Entity) -> Self {
        Error { category: Category::Recoverable(Some((Arc::new(context), recoverer))) }
    }

    /// Returns an error that indicates this error is an invalid operation,
    /// 
    pub fn invalid_operation(message: &'static str) -> Self {
        Error { category: Category::InvalidOperation(message) }
    }

    /// Returns an error that indicating the previous operation is being skipped,
    /// 
    pub fn skip(reason: &'static str) -> Self {
        Error { category: Category::Skip(reason) }
    }

    /// Returns true if the error is in the skip category,
    /// 
    pub fn is_skip(&self) -> bool {
        if let Category::Skip(..) = self.category {
            true
        } else {
            false 
        }
    }

    /// Returns true if the error is recoverable,
    /// 
    pub fn is_recoverable(&self) -> bool {
        if let Category::Recoverable(..) = self.category {
            true
        } else {
            false
        }
    }

    /// Returns true if the error is an invalid operation,
    /// 
    /// An invalid operation means that the previous function cannot proceed
    /// within the current context.
    /// 
    pub fn is_invalid_operation(&self) -> bool {
        if let Category::InvalidOperation(..) = self.category {
            true
        } else {
            false
        }
    }

    /// Returns the recovery state if applicable,
    /// 
    pub fn try_get_recovery_state(&self) -> Option<(Arc<ThunkContext>, Entity)> {
        match &self.category {
            Category::Recoverable(r) => r.clone(),
            _ => None
        }
    }
}

/// Error category,
/// 
#[derive(Debug, Clone)]
enum Category {
    /// Category of errors that indicate the error can be recovered from,
    /// 
    /// Optionally, may include state that can be used for recovery. If no state is provided then the 
    /// assumption is that this intermittent and a retry will eventually succeed.
    /// 
    Recoverable(Option<(Arc<ThunkContext>, Entity)>),
    /// Caregory of errors that inidicate that the previous operation is invalid in the current context,
    /// 
    InvalidOperation(&'static str),
    /// Category of errors that indicate to the caller that the previous operation can be skipped,
    /// 
    Skip(&'static str),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime error - {:?}", self.category)
    }
}

impl std::error::Error for Error {
}

