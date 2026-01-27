# Smelt REST API Reference

The Smelt API server provides HTTP endpoints for programmatic access to Smelt functionality.

## Running the Server

```bash
cargo run -p smelt-api -- --port 3000 --smelt-dir /path/to/.smelt
```

Default: `http://localhost:3000`

## Endpoints

### Health Check

#### GET /health

Check if the server is running.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

---

### Intents

#### GET /api/v1/intents

List all intents.

**Query Parameters:**
- `status` (optional): Filter by status (Draft, InProgress, Committed, etc.)

**Response:**
```json
{
  "intents": [
    {
      "id": "abc12345-...",
      "goal": "Add user authentication",
      "status": "InProgress",
      "author": {
        "name": "Developer",
        "email": "dev@example.com",
        "author_type": "Human"
      },
      "created_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

#### POST /api/v1/intents

Create a new intent.

**Request Body:**
```json
{
  "goal": "Add user authentication",
  "rationale": "Security requirement",
  "constraints": [
    {
      "name": "max_files",
      "value": "10",
      "required": true
    }
  ]
}
```

**Response:**
```json
{
  "id": "abc12345-...",
  "goal": "Add user authentication",
  "status": "Draft",
  "created_at": "2024-01-15T10:30:00Z"
}
```

#### GET /api/v1/intents/:id

Get intent details.

**Response:**
```json
{
  "id": "abc12345-...",
  "goal": "Add user authentication",
  "rationale": "Security requirement",
  "author": {
    "name": "Developer",
    "email": "dev@example.com",
    "author_type": "Human"
  },
  "constraints": [...],
  "context_links": {
    "issues": ["https://github.com/..."],
    "pull_requests": [],
    "documentation": []
  },
  "status": "InProgress",
  "created_at": "2024-01-15T10:30:00Z"
}
```

#### PATCH /api/v1/intents/:id/status

Update intent status.

**Request Body:**
```json
{
  "status": "Committed",
  "git_sha": "1a2b3c4d..."
}
```

---

### Deltas

#### GET /api/v1/deltas/:id

Get semantic delta details.

**Response:**
```json
{
  "id": "def67890-...",
  "intent_id": "abc12345-...",
  "timestamp": "2024-01-15T11:00:00Z",
  "changes": [
    {
      "type": "FunctionAdded",
      "name": "authenticate",
      "file": "src/auth.rs",
      "signature": "fn authenticate(user: &str, pass: &str) -> bool",
      "is_public": true
    }
  ],
  "impact_summary": {
    "files_affected": 2,
    "functions_added": 3,
    "functions_removed": 0,
    "functions_modified": 1,
    "types_added": 1,
    "breaking_changes": 0,
    "complexity_delta": 5
  }
}
```

#### GET /api/v1/intents/:id/deltas

Get all deltas for an intent.

**Response:**
```json
{
  "deltas": [...]
}
```

---

### Validation

#### POST /api/v1/validate

Validate a semantic delta.

**Request Body:**
```json
{
  "delta_id": "def67890-...",
  "intent_id": "abc12345-...",
  "strict": false
}
```

**Response:**
```json
{
  "passed": true,
  "violations": [],
  "error_count": 0,
  "warning_count": 1,
  "info_count": 2
}
```

**Violation structure:**
```json
{
  "rule": "complexity",
  "severity": "warning",
  "message": "Function complexity increased by 8 (threshold: 10)",
  "location": {
    "file": "src/auth.rs",
    "line": 45
  }
}
```

---

### Memory

#### POST /api/v1/memory/search

Search for relevant episodes.

**Request Body:**
```json
{
  "query": "authentication JWT tokens",
  "limit": 5
}
```

**Response:**
```json
{
  "episodes": [
    {
      "id": "ghi34567-...",
      "summary": "Implemented JWT token validation",
      "task_type": "feature",
      "outcome": "Success",
      "similarity": 0.85,
      "score": 0.78,
      "files_modified": ["src/auth.rs", "src/jwt.rs"],
      "tags": ["auth", "jwt", "security"]
    }
  ]
}
```

#### POST /api/v1/memory/feedback

Record feedback for an episode.

**Request Body:**
```json
{
  "episode_id": "ghi34567-...",
  "helpful": true
}
```

**Response:**
```json
{
  "success": true,
  "helpful_count": 5,
  "feedback_count": 6
}
```

#### GET /api/v1/memory/stats

Get memory statistics.

**Response:**
```json
{
  "total_episodes": 47,
  "helpful_count": 32,
  "average_utility": 0.72,
  "episodes_by_type": {
    "feature": 20,
    "bugfix": 15,
    "refactor": 8,
    "docs": 4
  }
}
```

---

### Status

#### GET /api/v1/status

Get repository and Smelt status.

**Response:**
```json
{
  "initialized": true,
  "repository": "my-project",
  "active_intent": {
    "id": "abc12345-...",
    "goal": "Add user authentication"
  },
  "changed_files": ["src/auth.rs", "src/main.rs"],
  "indexing": {
    "status": "complete",
    "files_indexed": 150
  }
}
```

---

## Error Responses

All endpoints return errors in a consistent format:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Intent not found: abc12345",
    "suggestion": "Check the intent ID or list available intents"
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `NOT_INITIALIZED` | 400 | Smelt not initialized in repository |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_FAILED` | 422 | Validation errors in request |
| `CONFLICT` | 409 | Resource already exists |
| `INTERNAL_ERROR` | 500 | Server error |

---

## Authentication

The API currently does not require authentication. For production deployments, consider adding:
- API key authentication
- JWT tokens
- OAuth2 integration

---

## Rate Limiting

No rate limiting is currently implemented. Consider adding limits for production use.

---

## CORS

The server includes CORS headers allowing requests from any origin. Configure appropriately for production.
