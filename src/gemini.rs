use crate::config::Config;
use crate::error::{AppError, AppResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::instrument;

const GEMINI_API_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"; // Using Flash for speed/cost
const PROBLEM_MARKER: &str = "**Problem:**";
const SOLUTION_MARKER: &str = "**Solution:**";
const SEPARATOR_MARKER: &str = "---SOLUTION_SEPARATOR---";

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Deserialize, Debug)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize, Debug)]
struct ResponsePart {
    text: String,
}

#[derive(Debug, Clone)]
pub struct FermiEstimation {
    pub problem: String,
    pub solution: String,
}

fn create_prompt() -> String {
    format!(
        "Generate a unique and interesting Fermi estimation problem suitable for a quick mental challenge. Ensure it's a different type of problem than common examples like piano tuners or jellybeans. \
        Provide the problem statement clearly, starting exactly with \"{PROBLEM_MARKER}\". \
        Then, insert a line containing only \"{SEPARATOR_MARKER}\". \
        Finally, provide a brief, step-by-step estimation outlining the assumptions and calculation, and state the final approximate answer, starting exactly with \"{SOLUTION_MARKER}\". \
        Do not include any text before the {PROBLEM_MARKER} or after the solution ends.",
        PROBLEM_MARKER = PROBLEM_MARKER,
        SEPARATOR_MARKER = SEPARATOR_MARKER,
        SOLUTION_MARKER = SOLUTION_MARKER
    )
}

#[instrument(skip(client, config), fields(prompt_len = create_prompt().len()))]
pub async fn generate_fermi_problem_and_solution(
    client: &Client,
    config: &Config,
) -> AppResult<FermiEstimation> {
    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: create_prompt(),
            }],
        }],
        generation_config: GenerationConfig {
            max_output_tokens: 5000,
            temperature: 1.8, // Balance creativity and predictability
        },
    };

    let url = format!("{}?key={}", GEMINI_API_URL, config.gemini_api_key);

    tracing::info!("Sending request to Gemini API");

    let response = client.post(&url).json(&request_body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        tracing::error!(status = %status, error_body = %error_text, "Gemini API request failed");
        return Err(AppError::GeminiApi(format!(
            "API request failed with status {}: {}",
            status, error_text
        )));
    }

    let response_body: GeminiResponse = response.json().await?;
    let generated_text = response_body
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.trim().to_string())
        .ok_or_else(|| {
            AppError::GeminiApi("No text content found in Gemini response".to_string())
        })?;

    tracing::debug!(generated_text = %generated_text, "Full Gemini output");
    let parts: Vec<&str> = generated_text.split(SEPARATOR_MARKER).collect();
    if parts.len() != 2 {
        tracing::error!(received_parts = parts.len(), generated_text = %generated_text, "Failed to split generated text by separator");
        return Err(AppError::ParseError(format!(
            "Expected 2 parts, found {}",
            parts.len()
        )));
    }

    let problem_part = parts[0].trim();
    let solution_part = parts[1].trim();

    if !problem_part.starts_with(PROBLEM_MARKER) || !solution_part.starts_with(SOLUTION_MARKER) {
        tracing::warn!("Problem or solution part missing expected marker");
        return Err(AppError::ParseError(
            "Missing expected marker(s)".to_string(),
        ));
    }

    let clean_problem = problem_part
        .strip_prefix(PROBLEM_MARKER)
        .unwrap_or(problem_part)
        .trim()
        .to_string();

    let clean_solution = solution_part
        .strip_prefix(SOLUTION_MARKER)
        .unwrap_or(solution_part)
        .trim()
        .to_string();

    if clean_problem.is_empty() || clean_solution.is_empty() {
        tracing::error!("Parsed problem or solution is empty after cleaning");
        return Err(AppError::ParseError(
            "Parsed problem or solution empty.".to_string(),
        ));
    }

    tracing::info!("Successfully parsed Fermi problem and solution");
    Ok(FermiEstimation {
        problem: clean_problem,
        solution: clean_solution,
    })
}
