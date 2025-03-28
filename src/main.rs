// src/main.rs
mod config;
mod error;
mod gemini;
mod ntfy;

use crate::config::Config;
use crate::error::AppError;
use actix_web::{App, HttpResponse, HttpServer, Responder, ResponseError, web};
use reqwest::Client;

use std::sync::Arc;
use tracing_subscriber::fmt::format::FmtSpan;

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        tracing::error!(error = %self, "Handler error occurred");
        match self {
            AppError::Config(_) => HttpResponse::InternalServerError().json("Configuration Error"),
            AppError::Reqwest(_) => HttpResponse::BadGateway().json("Upstream Service Error"),
            AppError::Serde(_) => HttpResponse::InternalServerError().json("Data Processing Error"),
            AppError::GeminiApi(_) => HttpResponse::BadGateway().json("Gemini API Error"),
            AppError::Ntfy(_) => HttpResponse::BadGateway().json("Notification Service Error"),
            AppError::Io(_) => HttpResponse::InternalServerError().json("IO Error"),
            AppError::ParseError(_) => {
                HttpResponse::InternalServerError().json("Content Parsing Error")
            }
            AppError::Internal(_) => {
                HttpResponse::InternalServerError().json("Internal Server Error")
            }
        }
    }
}

struct AppState {
    http_client: Client,
    config: Config,
}

const SOLUTION_DELAY: &str = "10m";

#[tracing::instrument(skip(app_state), fields(job_id = "cloud_scheduler_trigger"))]
async fn handle_fermi_request(
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, AppError> {
    tracing::info!("Received request to generate and send Fermi problem");

    // 1. Generate Problem and Solution
    let fermi_estimation =
        gemini::generate_fermi_problem_and_solution(&app_state.http_client, &app_state.config)
            .await?;

    // 2. Send Problem Immediately
    tracing::info!("Sending problem notification");
    ntfy::send_notification(
        &app_state.http_client,
        &app_state.config,
        "Problem: ",               // Title prefix
        &fermi_estimation.problem, // Body
        None,                      // No delay
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to send problem notification");
        e
    })?;

    // 3. Send Solution with Delay
    tracing::info!(
        "Scheduling solution notification with delay: {}",
        SOLUTION_DELAY
    );
    ntfy::send_notification(
        &app_state.http_client,
        &app_state.config,
        "Solution: ",               // Title prefix
        &fermi_estimation.solution, // Body
        Some(SOLUTION_DELAY),       // Apply delay
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to schedule solution notification");
        e
    })?;

    tracing::info!(
        "Successfully processed Fermi problem request (problem sent, solution scheduled)"
    );
    Ok(HttpResponse::Ok().body(format!(
        "Fermi problem sent, solution scheduled for {} delay.",
        SOLUTION_DELAY
    )))
}

// --- Health Check ---
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

// --- Main Function ---
#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .json()
        .init();

    tracing::info!("Starting Fermi Notifier Service");

    let config = Config::from_env()?;
    tracing::info!(port = config.port, ntfy_topic = %config.ntfy_topic, "Configuration loaded");

    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()?;

    let bind_port = config.port;

    let app_state = Arc::new(AppState {
        http_client,
        config,
    });

    tracing::info!("Starting HTTP server on port {}", app_state.config.port);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone())) // Clone Arc for each worker
            .route("/", web::post().to(handle_fermi_request))
            .route("/healthz", web::get().to(health_check))
    })
    .bind(("0.0.0.0", bind_port))?
    .run()
    .await?;

    tracing::info!("Server stopped");
    Ok(())
}
