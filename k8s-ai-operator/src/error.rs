// For managing errors
use thiserror::Error;

/// Enum for managing different types of errors, needed because the reconciler run function
/// needs to implement StdError
#[derive(Debug, Error)]
pub enum OperatorError {
    /// Errors reported by the kube-rs crate
    #[error("Kubernetes Example Operator Error: {source}")]
    KubeError {
        #[from]
        source: kube::Error,
    },
    // TODO: add more types of errors if needed
}

