use crate::crd::AiOperator;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Error};
use serde_json::{json, Value};
use log::info;

/// Adds a finalizer record into an `AiOperator` kind of resource. If the finalizer already exists,
/// this action has no effect.
///
/// # Arguments:
/// - `client` - Kubernetes client to modify the `Echo` resource with.
/// - `name` - Name of the `AiOperator` resource to modify. Existence is not verified
/// - `namespace` - Namespace where the `Echo` resource with given `name` resides.
///
/// Note: Does not check for resource's existence for simplicity.
pub async fn add(client: Client, name: &str, namespace: &str) -> Result<AiOperator, Error> {
    let api: Api<AiOperator> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": ["aioperator/finalizer"]
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}

/// Removes all finalizers from an `Nextcloud` resource. If there are no finalizers already, this
/// action has no effect.
///
/// # Arguments:
/// - `client` - Kubernetes client to modify the `AiOperator` resource with.
/// - `name` - Name of the `AiOperator` resource to modify. Existence is not verified
/// - `namespace` - Namespace where the `Nextcloud` resource with given `name` resides.
///
/// Note: Does not check for resource's existence for simplicity.
//pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<Nextcloud, Error> {
pub async fn delete(client: Client, name: &str, namespace: &str) -> Result <(), Error> {
    let api: Api<AiOperator> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": null
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    let ret = api.patch(name, &PatchParams::default(), &patch).await?;
    info!("---- AFTER DELETE COMMAND: {:?}", ret);

    Ok(())
}
