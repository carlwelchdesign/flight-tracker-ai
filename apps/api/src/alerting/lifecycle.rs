use thiserror::Error;

use crate::domain::{AlertActionKind, AlertLifecycle};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LifecycleError {
    #[error("dismiss requires a non-empty reason")]
    MissingDismissReason,
    #[error("{action:?} is not allowed while an alert is {lifecycle:?}")]
    InvalidTransition {
        lifecycle: AlertLifecycle,
        action: AlertActionKind,
    },
}

pub fn transition_lifecycle(
    lifecycle: AlertLifecycle,
    action: AlertActionKind,
    comment: Option<&str>,
) -> Result<AlertLifecycle, LifecycleError> {
    if action == AlertActionKind::Comment {
        return Ok(lifecycle);
    }
    if action == AlertActionKind::Dismiss && comment.is_none_or(|value| value.trim().is_empty()) {
        return Err(LifecycleError::MissingDismissReason);
    }

    match (lifecycle, action) {
        (AlertLifecycle::Open | AlertLifecycle::Acknowledged, AlertActionKind::Assign) => {
            Ok(lifecycle)
        }
        (AlertLifecycle::Open, AlertActionKind::Acknowledge) => Ok(AlertLifecycle::Acknowledged),
        (AlertLifecycle::Open | AlertLifecycle::Acknowledged, AlertActionKind::Dismiss) => {
            Ok(AlertLifecycle::Dismissed)
        }
        (AlertLifecycle::Open | AlertLifecycle::Acknowledged, AlertActionKind::Resolve) => {
            Ok(AlertLifecycle::Resolved)
        }
        _ => Err(LifecycleError::InvalidTransition { lifecycle, action }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_is_explicit_and_terminal_states_cannot_reopen() {
        assert_eq!(
            transition_lifecycle(AlertLifecycle::Open, AlertActionKind::Acknowledge, None),
            Ok(AlertLifecycle::Acknowledged)
        );
        assert_eq!(
            transition_lifecycle(
                AlertLifecycle::Acknowledged,
                AlertActionKind::Dismiss,
                Some("not operationally relevant")
            ),
            Ok(AlertLifecycle::Dismissed)
        );
        assert!(matches!(
            transition_lifecycle(AlertLifecycle::Resolved, AlertActionKind::Acknowledge, None),
            Err(LifecycleError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn comments_are_append_only_and_dismissal_requires_a_reason() {
        assert_eq!(
            transition_lifecycle(
                AlertLifecycle::Resolved,
                AlertActionKind::Comment,
                Some("post-event note")
            ),
            Ok(AlertLifecycle::Resolved)
        );
        assert_eq!(
            transition_lifecycle(AlertLifecycle::Open, AlertActionKind::Dismiss, Some("  ")),
            Err(LifecycleError::MissingDismissReason)
        );
    }
}
