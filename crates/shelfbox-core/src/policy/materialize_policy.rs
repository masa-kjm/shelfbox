//! Pure eligibility rules for explicit strategy conversion.
//!
//! This policy deliberately knows nothing about paths, Git, or filesystem
//! handles. Operations collect those facts and use the selected result to
//! request one typed `MaterializationAction` from `Materializer`.

use crate::domain::materialization::MaterializationStrategy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaterializeState {
    ManagedSymlink,
    EqualRegularCopy,
    DivergedRegularCopy,
    Missing,
    Unsafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaterializeDecision {
    NoOp,
    Replace { from: MaterializationStrategy },
    RequiresSync,
    Reject,
}

pub(crate) fn decide_materialize(
    requested: MaterializationStrategy,
    state: MaterializeState,
) -> MaterializeDecision {
    match (state, requested) {
        (MaterializeState::ManagedSymlink, MaterializationStrategy::Symlink)
        | (MaterializeState::EqualRegularCopy, MaterializationStrategy::Copy)
        // A request for the strategy already observed is a no-op. In
        // particular, it does not turn `item materialize --strategy copy`
        // into an implicit content-resolution operation.
        | (MaterializeState::DivergedRegularCopy, MaterializationStrategy::Copy) => {
            MaterializeDecision::NoOp
        }
        (MaterializeState::ManagedSymlink, MaterializationStrategy::Copy) => {
            MaterializeDecision::Replace {
                from: MaterializationStrategy::Symlink,
            }
        }
        (MaterializeState::EqualRegularCopy, MaterializationStrategy::Symlink) => {
            MaterializeDecision::Replace {
                from: MaterializationStrategy::Copy,
            }
        }
        (MaterializeState::DivergedRegularCopy, MaterializationStrategy::Symlink) => {
            MaterializeDecision::RequiresSync
        }
        (MaterializeState::Missing, _) | (MaterializeState::Unsafe, _) => {
            MaterializeDecision::Reject
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_equal_copies_may_be_converted_to_symlinks() {
        assert_eq!(
            decide_materialize(
                MaterializationStrategy::Symlink,
                MaterializeState::EqualRegularCopy,
            ),
            MaterializeDecision::Replace {
                from: MaterializationStrategy::Copy,
            }
        );
        assert_eq!(
            decide_materialize(
                MaterializationStrategy::Symlink,
                MaterializeState::DivergedRegularCopy,
            ),
            MaterializeDecision::RequiresSync
        );
    }

    #[test]
    fn observed_strategy_is_a_no_op_even_when_a_copy_has_diverged() {
        assert_eq!(
            decide_materialize(
                MaterializationStrategy::Copy,
                MaterializeState::DivergedRegularCopy,
            ),
            MaterializeDecision::NoOp
        );
    }
}
