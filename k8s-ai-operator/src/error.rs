// For managing errors
use thiserror::Error;

/// Enum for managing different types of errors, needed because the reconciler run function
/// needs to implement StdError
#[derive(Debug, Error)]
pub enum AiOperatorError {
    /// Errors reported by the kube-rs crate
    #[error("Kubernetes Example Operator Error: {source}")]
    KubeError {
        #[from]
        source: kube::Error,
    },
    /// Error in user input or AiOperator resource definition, typically missing fields.
    #[error("Invalid AiOperator CRD: {0}")]
    UserInputError(String),
}

