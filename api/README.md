# REST API with Axum

A production-ready REST API built with Rust and Axum, featuring JWT authentication, CRUD operations, and in-memory data storage.

## Features

- JWT Authentication - Secure token-based auth with bcrypt password hashing
- Post Management - Full CRUD operations on blog posts
- User Management - User registration, login, and profile access
- Pagination - Efficient data retrieval with page and limit parameters
- Input Validation - Request validation using the validator crate
- Error Handling - Comprehensive error handling with proper HTTP status codes
- CORS Support - Cross-origin resource sharing enabled
- Structured Logging - Request/response logging with tracing

## Tech Stack

- **Framework**: [Axum](https://github.com/tokio-rs/axum) - Ergonomic web framework built on Tokio
- **Runtime**: [Tokio](https://tokio.rs/) - Async runtime for Rust
- **Authentication**: JWT with [jsonwebtoken](https://docs.rs/jsonwebtoken/)
- **Password Hashing**: [bcrypt](https://docs.rs/bcrypt/)
- **Validation**: [validator](https://docs.rs/validator/)
- **Serialization**: [serde](https://serde.rs/) & [serde_json](https://docs.rs/serde_json/)
- **Logging**: [tracing](https://docs.rs/tracing/)

## Project Structure

```
.
├── auth.rs              # JWT token creation and validation
├── dto/                 # Data Transfer Objects
│   ├── mod.rs
│   ├── requests.rs      # Request payloads
│   └── responses.rs     # Response structures
├── errors.rs            # Custom error types and HTTP responses
├── handlers/            # Route handlers
│   ├── health.rs        # Health check endpoint
│   ├── mod.rs
│   ├── post_handlers.rs # Post CRUD operations
│   └── user_handlers.rs # User authentication & profile
├── main.rs              # Application entry point
├── models/              # Domain models
│   ├── mod.rs
│   ├── post_model.rs    # Post entity
│   └── user_model.rs    # User entity
└── states.rs            # Application state (in-memory storage)
```

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd <project-directory>
```

2. Build the project:
```bash
cargo build --release
```

3. Run the server:
```bash
cargo run
```

The API will start on `http://localhost:3000`

## API Endpoints

### Health

| Method | Endpoint | Description | Auth Required |
|--------|----------|-------------|---------------|
| GET | `/health` | Health check | No |

### Authentication

| Method | Endpoint | Description | Auth Required |
|--------|----------|-------------|---------------|
| POST | `/auth/signup` | Register new user | No |
| POST | `/auth/login` | Login user | No |

### Users

| Method | Endpoint | Description | Auth Required |
|--------|----------|-------------|---------------|
| GET | `/users/me` | Get current user profile | Yes |

### Posts

| Method | Endpoint | Description | Auth Required |
|--------|----------|-------------|---------------|
| GET | `/posts` | Get all posts (paginated) | No |
| GET | `/posts/:id` | Get post by ID | No |
| POST | `/posts` | Create new post | Yes |
| DELETE | `/posts/:id` | Delete post (owner only) | Yes |

## Usage Examples

### 1. Register a New User

```bash
curl -X POST http://localhost:3000/auth/signup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "username": "johndoe",
    "password": "securepassword123"
  }'
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "username": "johndoe",
    "created_at": 1699564800
  }
}
```

### 2. Login

```bash
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "securepassword123"
  }'
```

### 3. Get Current User (Protected)

```bash
curl http://localhost:3000/users/me \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

### 4. Create a Post (Protected)

```bash
curl -X POST http://localhost:3000/posts \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "My First Post",
    "content": "Hello, World! This is my first blog post."
  }'
```

### 5. Get All Posts (Paginated)

```bash
curl "http://localhost:3000/posts?page=1&limit=10"
```

**Response:**
```json
{
  "data": [
    {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "title": "My First Post",
      "content": "Hello, World!",
      "author_id": "550e8400-e29b-41d4-a716-446655440000",
      "created_at": 1699564800,
      "updated_at": 1699564800
    }
  ],
  "page": 1,
  "limit": 10,
  "total": 1
}
```

### 6. Get Post by ID

```bash
curl http://localhost:3000/posts/123e4567-e89b-12d3-a456-426614174000
```

### 7. Delete Post (Protected, Owner Only)

```bash
curl -X DELETE http://localhost:3000/posts/123e4567-e89b-12d3-a456-426614174000 \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

## Configuration

The API uses the following default configurations:

- **Port**: 3000
- **JWT Secret**: `your-secret-key-change-this-in-production`
- **JWT Expiration**: 24 hours
- **Password Cost**: bcrypt DEFAULT_COST (12)
- **CORS**: Permissive (all origins) - Configure for production

## Error Responses

All errors follow a consistent JSON format:

```json
{
  "error": "Error message description"
}
```

### HTTP Status Codes

- `200 OK` - Request succeeded
- `201 Created` - Resource created successfully
- `204 No Content` - Request succeeded, no content to return
- `400 Bad Request` - Invalid input or validation error
- `401 Unauthorized` - Missing or invalid authentication token
- `404 Not Found` - Resource not found
- `409 Conflict` - Resource already exists (e.g., duplicate email)
- `500 Internal Server Error` - Server error

## Security Considerations

**Important for Production:**

1. **Change JWT Secret**: Update the JWT secret in `states.rs` to a strong, random value
2. **Environment Variables**: Move sensitive config to environment variables
3. **CORS**: Configure CORS to allow only trusted origins
4. **HTTPS**: Always use HTTPS in production
5. **Database**: Replace in-memory storage with a persistent database (PostgreSQL, MySQL)
6. **Rate Limiting**: Implement rate limiting to prevent abuse
7. **Logging**: Configure appropriate log levels for production

## Development

### Run in development mode:
```bash
cargo run
```

### Build for production:
```bash
cargo build --release
```

### Check code:
```bash
cargo check
cargo clippy
cargo fmt
```