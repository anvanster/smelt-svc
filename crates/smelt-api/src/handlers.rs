// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! API request handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use smelt_core::{
    Author, AuthorType, ContextLinks, IntentRecord, IntentStatus, SmeltGraph, SqliteStorage,
};
use smelt_memory::SmeltMemory;
use smelt_validator::SmeltValidator;
use uuid::Uuid;

use crate::state::AppState;

// === Response Types ===

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub initialized: bool,
    pub database_ok: bool,
    pub graph_ok: bool,
    pub memory_ok: bool,
    pub intent_count: usize,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub suggestion: Option<String>,
}

#[derive(Serialize)]
pub struct IntentResponse {
    pub id: String,
    pub goal: String,
    pub rationale: Option<String>,
    pub status: String,
    pub author: AuthorResponse,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct AuthorResponse {
    pub name: String,
    pub email: String,
    pub author_type: String,
}

#[derive(Serialize)]
pub struct DeltaResponse {
    pub id: String,
    pub intent_id: String,
    pub timestamp: String,
    pub files_affected: usize,
    pub functions_added: usize,
    pub functions_removed: usize,
    pub functions_modified: usize,
    pub breaking_changes: usize,
}

// === Request Types ===

#[derive(Deserialize)]
pub struct CreateIntentRequest {
    pub goal: String,
    pub rationale: Option<String>,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub intent_id: Option<String>,
}

#[derive(Deserialize)]
pub struct MemorySearchRequest {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct FeedbackRequest {
    pub helpful: bool,
}

// === Handlers ===

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Get system status
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let storage = match SqliteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(_) => {
            return Json(StatusResponse {
                initialized: false,
                database_ok: false,
                graph_ok: false,
                memory_ok: false,
                intent_count: 0,
            })
        }
    };

    let intent_count = storage.list_intents(None).map(|i| i.len()).unwrap_or(0);
    let graph_ok = SmeltGraph::open(&state.graph_path).is_ok();
    let memory_ok = state.memory_path.exists();

    Json(StatusResponse {
        initialized: true,
        database_ok: true,
        graph_ok,
        memory_ok,
        intent_count,
    })
}

/// List all intents
pub async fn list_intents(State(state): State<AppState>) -> impl IntoResponse {
    let storage = match SqliteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    match storage.list_intents(None) {
        Ok(intents) => {
            let responses: Vec<IntentResponse> = intents
                .into_iter()
                .map(|i| intent_to_response(&i))
                .collect();
            Json(responses).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                suggestion: None,
            }),
        )
            .into_response(),
    }
}

/// Create a new intent
pub async fn create_intent(
    State(state): State<AppState>,
    Json(req): Json<CreateIntentRequest>,
) -> impl IntoResponse {
    let storage = match SqliteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    let mut graph = match SmeltGraph::open(&state.graph_path) {
        Ok(g) => g,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    let intent_id = Uuid::new_v4();

    // Capture baseline snapshot
    let snapshot_id = match graph.snapshot_for_intent(intent_id) {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::warn!("Failed to capture baseline snapshot: {}", e);
            None
        }
    };

    let intent = IntentRecord {
        id: intent_id,
        created_at: Utc::now(),
        author: Author {
            name: req.author_name.unwrap_or_else(|| "API User".to_string()),
            email: req
                .author_email
                .unwrap_or_else(|| "api@smelt.local".to_string()),
            author_type: AuthorType::AI,
        },
        goal: req.goal,
        rationale: req.rationale,
        constraints: Vec::new(),
        context_links: ContextLinks::default(),
        status: IntentStatus::InProgress,
        baseline_snapshot_id: snapshot_id,
    };

    match storage.store_intent(&intent) {
        Ok(_) => (StatusCode::CREATED, Json(intent_to_response(&intent))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                suggestion: None,
            }),
        )
            .into_response(),
    }
}

/// Get a specific intent
pub async fn get_intent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let storage = match SqliteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    match storage.find_intent_by_prefix(&id) {
        Ok(Some(intent)) => Json(intent_to_response(&intent)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Intent not found: {}", id),
                suggestion: Some("Use GET /api/v1/intents to list available intents.".to_string()),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                suggestion: None,
            }),
        )
            .into_response(),
    }
}

/// Get a specific delta
pub async fn get_delta(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let storage = match SqliteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid UUID format".to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    match storage.get_delta(uuid) {
        Ok(Some(delta)) => Json(delta_to_response(&delta)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Delta not found: {}", id),
                suggestion: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                suggestion: None,
            }),
        )
            .into_response(),
    }
}

/// Get delta for an intent
pub async fn get_intent_delta(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let storage = match SqliteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    // First find the intent
    let intent = match storage.find_intent_by_prefix(&id) {
        Ok(Some(i)) => i,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Intent not found: {}", id),
                    suggestion: None,
                }),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    // Get delta using the intent's ID
    match storage.get_delta(intent.id) {
        Ok(Some(delta)) => Json(delta_to_response(&delta)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "No delta found for this intent".to_string(),
                suggestion: Some("Delta is created when the intent is committed.".to_string()),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                suggestion: None,
            }),
        )
            .into_response(),
    }
}

/// Validate current changes
pub async fn validate(
    State(state): State<AppState>,
    Json(req): Json<ValidateRequest>,
) -> impl IntoResponse {
    let storage = match SqliteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    let validator = SmeltValidator::from_smelt_dir(&state.smelt_dir);

    // Get intent if specified
    let intent = if let Some(ref id) = req.intent_id {
        match storage.find_intent_by_prefix(id) {
            Ok(i) => i,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: e.to_string(),
                        suggestion: None,
                    }),
                )
                    .into_response()
            }
        }
    } else {
        None
    };

    // Create a minimal delta for validation
    let delta = smelt_core::SemanticDelta {
        id: Uuid::new_v4(),
        intent_id: intent.as_ref().map(|i| i.id).unwrap_or_else(Uuid::new_v4),
        timestamp: Utc::now(),
        from_snapshot: Uuid::nil(),
        to_snapshot: Uuid::nil(),
        changes: Vec::new(),
        impact_summary: smelt_core::ImpactSummary::default(),
    };

    let outcome = validator.validate(&delta, intent.as_ref());

    #[derive(Serialize)]
    struct ValidationResponse {
        passed: bool,
        error_count: usize,
        warning_count: usize,
        errors: Vec<ValidationIssue>,
        warnings: Vec<ValidationIssue>,
    }

    #[derive(Serialize)]
    struct ValidationIssue {
        rule: String,
        message: String,
        location: Option<String>,
        suggestion: Option<String>,
    }

    let errors: Vec<ValidationIssue> = outcome
        .errors()
        .map(|e| ValidationIssue {
            rule: e.rule.clone(),
            message: e.message.clone(),
            location: e.location.clone(),
            suggestion: e.suggestion.clone(),
        })
        .collect();

    let warnings: Vec<ValidationIssue> = outcome
        .warnings()
        .map(|w| ValidationIssue {
            rule: w.rule.clone(),
            message: w.message.clone(),
            location: w.location.clone(),
            suggestion: w.suggestion.clone(),
        })
        .collect();

    Json(ValidationResponse {
        passed: !outcome.has_errors(),
        error_count: outcome.error_count,
        warning_count: outcome.warning_count,
        errors,
        warnings,
    })
    .into_response()
}

/// Search memory for relevant episodes
pub async fn memory_search(
    State(state): State<AppState>,
    Json(req): Json<MemorySearchRequest>,
) -> impl IntoResponse {
    if !state.memory_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Memory system not available".to_string(),
                suggestion: Some("Initialize memory with 'smelt init'.".to_string()),
            }),
        )
            .into_response();
    }

    let memory = match SmeltMemory::open(&state.memory_path) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    let limit = req.limit.unwrap_or(5);

    match memory.retrieve(&req.query, limit) {
        Ok(episodes) => {
            #[derive(Serialize)]
            struct EpisodeResult {
                id: String,
                summary: String,
                task_type: String,
                outcome: String,
                utility_score: f64,
                relevance_score: f64,
            }

            let results: Vec<EpisodeResult> = episodes
                .into_iter()
                .map(|e| EpisodeResult {
                    id: e.episode.id.to_string(),
                    summary: e.episode.summary.clone(),
                    task_type: format!("{:?}", e.episode.task_type),
                    outcome: format!("{:?}", e.episode.outcome),
                    utility_score: e.score,
                    relevance_score: e.similarity,
                })
                .collect();

            Json(results).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                suggestion: None,
            }),
        )
            .into_response(),
    }
}

/// Get a specific episode
pub async fn get_episode(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !state.memory_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Memory system not available".to_string(),
                suggestion: None,
            }),
        )
            .into_response();
    }

    let memory = match SmeltMemory::open(&state.memory_path) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid UUID format".to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    match memory.get_episode(uuid) {
        Ok(Some(episode)) => {
            #[derive(Serialize)]
            struct EpisodeResponse {
                id: String,
                summary: String,
                task_type: String,
                outcome: String,
                tags: Vec<String>,
                files_modified: Vec<String>,
                created_at: String,
            }

            Json(EpisodeResponse {
                id: episode.id.to_string(),
                summary: episode.summary.clone(),
                task_type: format!("{:?}", episode.task_type),
                outcome: format!("{:?}", episode.outcome),
                tags: episode.tags.clone(),
                files_modified: episode.files_modified.clone(),
                created_at: episode.created_at.to_rfc3339(),
            })
            .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Episode not found: {}", id),
                suggestion: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                suggestion: None,
            }),
        )
            .into_response(),
    }
}

/// Submit feedback for an episode
pub async fn episode_feedback(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<FeedbackRequest>,
) -> impl IntoResponse {
    if !state.memory_path.exists() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Memory system not available".to_string(),
                suggestion: None,
            }),
        )
            .into_response();
    }

    let mut memory = match SmeltMemory::open(&state.memory_path) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid UUID format".to_string(),
                    suggestion: None,
                }),
            )
                .into_response()
        }
    };

    match memory.record_feedback(uuid, req.helpful) {
        Ok(_) => {
            #[derive(Serialize)]
            struct FeedbackResponse {
                success: bool,
                message: String,
            }

            Json(FeedbackResponse {
                success: true,
                message: format!(
                    "Feedback recorded: episode {} marked as {}",
                    id,
                    if req.helpful {
                        "helpful"
                    } else {
                        "not helpful"
                    }
                ),
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                suggestion: None,
            }),
        )
            .into_response(),
    }
}

// === Helper Functions ===

fn intent_to_response(intent: &IntentRecord) -> IntentResponse {
    IntentResponse {
        id: intent.id.to_string(),
        goal: intent.goal.clone(),
        rationale: intent.rationale.clone(),
        status: match &intent.status {
            IntentStatus::InProgress => "in_progress".to_string(),
            IntentStatus::Committed { git_sha } => format!("committed:{}", &git_sha[..8]),
            IntentStatus::Abandoned => "abandoned".to_string(),
            IntentStatus::Draft => "draft".to_string(),
            IntentStatus::PendingValidation => "pending_validation".to_string(),
            IntentStatus::Validated => "validated".to_string(),
            IntentStatus::Rejected { violations } => {
                format!("rejected:{}", violations.len())
            }
        },
        author: AuthorResponse {
            name: intent.author.name.clone(),
            email: intent.author.email.clone(),
            author_type: format!("{:?}", intent.author.author_type),
        },
        created_at: intent.created_at.to_rfc3339(),
    }
}

fn delta_to_response(delta: &smelt_core::SemanticDelta) -> DeltaResponse {
    DeltaResponse {
        id: delta.id.to_string(),
        intent_id: delta.intent_id.to_string(),
        timestamp: delta.timestamp.to_rfc3339(),
        files_affected: delta.impact_summary.files_affected,
        functions_added: delta.impact_summary.functions_added,
        functions_removed: delta.impact_summary.functions_removed,
        functions_modified: delta.impact_summary.functions_modified,
        breaking_changes: delta.impact_summary.breaking_changes,
    }
}
