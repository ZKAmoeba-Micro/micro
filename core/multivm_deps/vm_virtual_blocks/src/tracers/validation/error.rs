use crate::Halt;
use micro_types::vm_trace::ViolatedValidationRule;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum ValidationError {
    FailedTx(Halt),
    ViolatedRule(ViolatedValidationRule),
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedTx(revert_reason) => {
                write!(f, "Validation revert: {}", revert_reason)
            }
            Self::ViolatedRule(rule) => {
                write!(f, "Violated validation rules: {}", rule)
            }
        }
    }
}
