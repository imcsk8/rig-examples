use std::sync::{Arc};
use crate::AiOperator;
use crate::crd::{AiOperatorStatus};
use crate::error::{AiOperatorError};
use log::{info};
use kube::Client;
use rig::{completion::Prompt, providers::openai};
use dotenv::dotenv;


/// Apply the changes
pub async fn apply(
    client: Client,
    name: &str,
    aiop: Arc<AiOperator>,
    namespace: &str,
) -> Result<AiOperatorStatus, AiOperatorError> {
    let mut global_state_hash = "HASH".to_string();

    info!("Applying changes!!");
    info!("Calling OpenAI API: {}", aiop.spec.prompt.to_string());
    let answer = send_prompt(aiop.spec.prompt.to_string()).await;
    //info!("Result API: {}", aiop.spec.answer.to_string());
    info!("Result API: {}", answer);


    // TODO check if we need a success object
    Ok(AiOperatorStatus {
        installed: false,
        configured: 0,
        maintenance: false,
        waiting: false,
        last_backup: "N/A".to_string(),
        state_hash: global_state_hash,
    })
}

async fn send_prompt(aiop_prompt: String) -> String {
	dotenv().ok();
    // Create OpenAI client and agent.
    // This requires the `OPENAI_API_KEY` environment variable to be set.
    let openai_client = openai::Client::from_env();

    let gpt4 = openai_client.agent("gpt-3.5-turbo").build();

    // Prompt the model and print its response
    let response = gpt4
        .prompt(aiop_prompt)
        .await
        .expect("Failed to prompt GPT-4");

    info!("GPT Response: {response}");
    response
}
