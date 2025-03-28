use rig::{completion::Prompt, providers::openai};
use dotenv::dotenv;
//use std::env;

#[tokio::main]
async fn main() {
	dotenv().ok();
    // Create OpenAI client and agent.
    // This requires the `OPENAI_API_KEY` environment variable to be set.
    let openai_client = openai::Client::from_env();

    let gpt4 = openai_client.agent("gpt-3.5-turbo").build();

    // Prompt the model and print its response
    let response = gpt4
        .prompt("You are an expert Rust AI agent programmer a hello world \
				program using the rig crate")
        .await
        .expect("Failed to prompt GPT-4");

    println!("GPT-4: {response}");
}
