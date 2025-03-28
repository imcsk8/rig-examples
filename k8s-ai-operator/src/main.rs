// Methods for managing k8s resources
//use kube::Resource;
use kube::ResourceExt;

// To handle asynchronous networking
use tokio::time::Duration;

// Kubernetes OpenAPI "objects"
use k8s_openapi::api::apps::v1::{Deployment};

// Wrappers for the kubernetes functionalities
use kube::{
    // The client communicates with the k8s API
    client::Client,
    // Represents the results of the reconciliation attempt
    runtime::controller::Action,
    // A controller is an infinite loop that gets a stream of objects to
    // be reconciled
    runtime::Controller,
    // Wrappers for the k8s API interaction
    Api,
    api::{ListParams, Patch, PatchParams}
};

// For managing errors
use thiserror::Error;

// Logging macros
use log::{info};

// Thread safe atomic reference counters for pointers
use std::sync::Arc;


// The k8s Pod structure
use k8s_openapi::api::core::v1::Pod;

// Kubernetes configuration objects
//use kube::Config;

// Configuration for the controller
use kube::runtime::watcher::Config;

// For managing iteration of k8s objects
//use futures_util::stream::stream::StreamExt;
use futures_util::StreamExt;

use kube::runtime::controller::Error as KubeContError;

// For logging
use pretty_env_logger;

/// ContextData just wraps `kube::client::Client` so it can be added implementations
/// Context injected with each `reconcile` and `on_error` method invocation.
struct ContextData {
    /// Kubernetes client to make Kubernetes API requests with. Required for K8S resource management.
    client: Client,
}

/// Enum for managing different types of errors, needed because the reconciler run function
/// needs to implement StdError
#[derive(Debug, Error)]
pub enum ExampleError {
    /// Errors reported by the kube-rs crate
    #[error("Kubernetes Example Operator Error: {source}")]
    KubeError {
        #[from]
        source: kube::Error,
    },
    // TODO: add more types of errors if needed
}

#[tokio::main]
async fn main() -> Result <(), ExampleError> {
    pretty_env_logger::init_timed();
    info!("Starting operator");
    // Load the client
    let kc: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG file");
    println!("Hello, world!");
    // Get the API client
    let api: Api<Pod> = Api::all(kc.clone());
    let context: Arc<ContextData> = Arc::new(ContextData::new(kc.clone()));

    // Instance of a controller
    Controller::new(api.clone(), Config::default())
        .run(reconcile, on_error, context)
        .for_each(|reconciliation_result| async move {
            match reconciliation_result {
                Ok(r) => {
                    info!("Reconciliation successful. Resource: {:?}", r);
                },
                Err(e) => {
                    match e {
                        KubeContError::ReconcilerFailed(err, obj) => {
                            info!("Reconciliation error!! {:?}",
                                err);

                        },
                        _ => {},
                    }
                }
            }
        }).await;

    Ok(())
}

/// Check reconciliation data
async fn reconcile(pod: Arc<Pod>, context: Arc<ContextData>
) -> Result<Action, ExampleError> {
    info!("Status: {:?}", pod.status);
    let name = pod.name_any();
    info!("Resource name: {}", name);
    // Reconcile every 10 seconds
    Ok(Action::requeue(Duration::from_secs(10)))
}

/// Acctions taken when reonciliation fails
fn on_error(pod: Arc<Pod>, error: &ExampleError, _context: Arc<ContextData>
) -> Action {
    eprintln!("Error: {:?}", error);
    info!("Error: {:?}", error);
    Action::requeue(Duration::from_secs(5))
}

/// Context data handler
impl ContextData {
    /// Constructs a new instance of ContextData.
    ///
    /// # Arguments:
    /// - `client`: A Kubernetes client to make Kubernetes REST API requests with. Resources
    /// will be created and deleted with this client.
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }
}
