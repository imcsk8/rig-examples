// Methods for managing k8s resources
//use kube::Resource;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
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
    Resource,
    api::{ListParams, Patch, PatchParams}
};

// For managing errors
use thiserror::Error;

// Logging macros
use log::{debug, info};

// The k8s Pod structure
use k8s_openapi::api::core::v1::Pod;

// Kubernetes configuration objects
//use kube::Config;

// Configuration for the controller
use kube::runtime::watcher::Config;

// For managing iteration of k8s objects
//use futures_util::stream::stream::StreamExt;
use futures_util::StreamExt;

// Our errors
use crate::error::*;
use kube::runtime::controller::Error as KubeContError;

// Context
use crate::context::*;

// CRD
//use crate::crd::{AiOperator, AiOperatorResource, AiOperatorAction, create_crd};
use crate::crd::{AiOperator, AiOperatorAction, create_crd};

// For hasing
use sha2::{Digest, Sha256};

// For logging
use pretty_env_logger;

// Functions that perform actions
use aioperator::apply;

pub mod context;
pub mod crd;
pub mod error;
pub mod aioperator;
pub mod finalizer;

#[tokio::main]
async fn main() -> Result <(), AiOperatorError> {
    pretty_env_logger::init_timed();
    info!("Starting operator");
    // Load the client
    let kc: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG file");
    debug!("---- Before creating crd ---");
    create_crd(kc.clone()).await;
    debug!("---- After creating crd ---");
    println!("Starting AiOperator...");
    // Get the API client
    let api: Api<AiOperator> = Api::all(kc.clone());
    let context: Arc<ContextData> = Arc::new(ContextData::new(kc.clone()));
    // Control loop
    run_controller(api.clone(), context).await;

    Ok(())
}

/// Check reconciliation data
async fn reconcile(aiop: Arc<AiOperator>, context: Arc<ContextData>
) -> Result<Action, AiOperatorError> {
    let client: Client = context.client.clone(); // The `Client` is shared -> a clone from the reference is obtained

    info!("Status: {:?}", aiop.status);

    // The resource of `AiOperator` kind is required to have a namespace set. However, it is not guaranteed
    // the resource will have a `namespace` set. Therefore, the `namespace` field on object's metadata
    // is optional and Rust forces the programmer to check for it's existence first.
    let namespace: String = match aiop.namespace() {
        None => {
            // If there is no namespace to deploy to defined, reconciliation ends with an error immediately.
            return Err(AiOperatorError::UserInputError(
                "Expected AiOperator resource to be namespaced. Can't deploy to an unknown namespace."
                    .to_owned(),
            ));
        }
        Some(namespace) => namespace,
    };

    let name = aiop.name_any();
    info!("AiOperator Resource name: {}", name);
    info!("AiOperator namespace: {}", namespace);
    // Reconcile every 10 seconds
    //Ok(Action::requeue(Duration::from_secs(10)))

    // Performs action as decided by the `determine_action` function.
    return match determine_action(&aiop, client.clone()).await.unwrap() {
        AiOperatorAction::Create | AiOperatorAction::Update => {
                finalizer::add(client.clone(), &name, &namespace).await?;
                info!("En CREATE | UPDATE");
                // STATUS
                /*let mut annotations: BTreeMap<String, String> = BTreeMap::new();
                let new_state_hash = create_hash(&meta.name.as_ref().unwrap(),
                    aiop.spec.prompt.clone());
                annotations.insert("state_hash".to_owned(), new_state_hash.clone());*/
                let new_status = apply(client.clone(), &name, aiop.clone(), &namespace).await?;
                let aioperators: Api<AiOperator> = Api::namespaced(client.clone(), &namespace);
                let patch_params = PatchParams {
                    field_manager: Some("AiOperator".to_string()),
                    ..PatchParams::default()
                };
                // Update the status in a scoped block
                {
                    let inner_nc = AiOperator {
                        metadata: aiop.meta().clone(),
                        spec: aiop.spec.clone(),
                        status: Some(new_status.clone())
                    };
                    let aiop_mutex: Arc<Mutex<AiOperator>> = Arc::new(Mutex::new(inner_nc));
                    let mut locked_aiop = aiop_mutex.lock().unwrap();
                    locked_aiop.status = Some(new_status);
                }
                let _result = aioperators
                    .patch(
                        aiop.meta().name.clone().unwrap().as_str(),
                        &patch_params,
                        &Patch::Apply(&*aiop)
                    )
                .await?;
                info!("---------- DESPUES DE PATCH");
            //};
            //Ok(Action::requeue(Duration::from_secs(60)))
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        AiOperatorAction::Delete => {
            // Once the deployment is successfully removed, remove the finalizer to make it possible
            // for Kubernetes to delete the `AiOperator` resource.
            finalizer::delete(client, &name, &namespace).await?;
            info!("AiOperator resource: {} deleted", name);
            //Ok(Action::await_change()) // Makes no sense to delete after a successful delete, as the resource is gone
            Ok(Action::requeue(Duration::from_secs(60)))
        }
        // The resource is already in desired state, do nothing and re-check after 10 seconds
        AiOperatorAction::NoOp => Ok(Action::requeue(Duration::from_secs(10))),
    };

}

/// Acctions taken when reonciliation fails
fn on_error(pod: Arc<AiOperator>, error: &AiOperatorError, _context: Arc<ContextData>
) -> Action {
    eprintln!("Error: {:?}", error);
    info!("Error: {:?}", error);
    Action::requeue(Duration::from_secs(5))
}

/// Control loop
async fn run_controller(api: Api<AiOperator>, context: Arc<ContextData>) {
    Controller::new(api.clone(), Config::default())
        .run(reconcile, on_error, context)
        .for_each(|reconciliation_result| async move {
            //check_reconciliation_result(reconciliation_result);
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
}

/// Resources arrives into reconciliation queue in a certain state. This function looks at
/// the state of given `AiOperator` resource and decides which actions needs to be performed.
/// The finite set of possible actions is represented by the `AiOperatorAction` enum.
///
/// # Arguments
/// - `echo`: A reference to `AiOperator` being reconciled to decide next action upon.
/// TODO: check for more resource status
/// meta() -> https://docs.rs/kube/0.88.1/kube/core/struct.ObjectMeta.html
async fn determine_action(aiop: &AiOperator, client: Client) ->
    Result<AiOperatorAction, String> {

    let updating = is_update(&aiop, client).await?;
    if updating {
        info!("Update {:?}", updating);
    } else {
        info!("Not an update {:?}", updating);
    }

    //info!("AiOperator? {:?}", &aiop);
    let aiop_meta = aiop.meta();
    let name = match &aiop_meta.name {
        Some(n) => n,
        None    => "N/A",
    };
    return if aiop_meta.deletion_timestamp.is_some() {
        info!("Deleting: {}", name);
        Ok(AiOperatorAction::Delete)
    } else if aiop_meta.finalizers
        .as_ref()
        .map_or(true, |finalizers| finalizers.is_empty())
    {
        info!("Creating: {}", name);
        Ok(AiOperatorAction::Create)

    } else if updating {
        Ok(AiOperatorAction::Update)
    } else {
        info!("Nothing to do for: {}", name);
        Ok(AiOperatorAction::NoOp)
    };
}

async fn is_update(aiop: &AiOperator, client: Client) ->
    Result<bool, String> {

    let meta = aiop.meta();
    //let current_state_hash = meta.annotations.unwrap().get("state_hash");


    /*let anon = get_annotations(client.clone(), aiop.metadata.namespace.clone().unwrap()).await?;
    info!("-------------- WTF!!: {:?}", anon);*/


    let annotations = match get_annotations(client.clone(),
        aiop.metadata.namespace.clone().unwrap()).await {
        //Ok(a) => a,
        Ok(a) => {
            info!("----------- ANNOTATION: {:?}", a);
            a
        },
        _     => {
            // No annotations means that the object is being created
            info!("Is creating");
            return Ok(false);
        },
    };

    // Create a hash to keep track of the changes
    let new_state_hash = create_hash(&meta.name.as_ref().unwrap(),
        aiop.spec.prompt.clone());

    let current_state_hash: String = annotations.get("state_hash")
        .unwrap_or(&String::from("N/A"))
        .to_string(); //TODO: use other default
    info!("--- NEW STATE HASH: {:?}", new_state_hash);
    info!("--- CURRENT STATE HASH: {:?}", current_state_hash);

    // If the state hashes are different we're updating
    if current_state_hash != new_state_hash {
        return Ok(true);
    }
    Ok(true)
}

/// Creates a sha256 hash from the given attributes
pub fn create_hash(name: &str,
    prompt: String,
) -> String {
    let state_string = format!("{}-{}",
        name,
        prompt.to_string(),
    );
    let mut hasher = Sha256::new();
    hasher.update(state_string.as_bytes());
    hasher.finalize()
      .iter()
      .map(|byte| format!("{:02x}", byte))
      .collect::<String>()
}

async fn get_annotations(client: Client, namespace: String) ->
    Result <BTreeMap<std::string::String, std::string::String>, String> {
    let list_params = ListParams::default();
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    let mut deployment = deployments.list(&list_params).await.unwrap();
    //info!("ITEMS: {}", &deployment.items);
    // We only have one deployment
    let my_deployment = deployment.items.pop();
    if my_deployment.is_none() {
        return Err("No annotations".to_string());
    }
    match &my_deployment.unwrap().metadata.annotations {
        Some(a) =>  {
            Ok(a.clone())
        },
        None => Err("There are no annotations!!!".to_string())
    }
}

