use axum::{
    Router,
    extract::{Json, Path, State},
    routing::{get, post},
};
use luts_core::utils::blocks::BlockUtils;
use luts_core::memory::{BlockId, MemoryBlock, MemoryQuery};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiState {
    pub block_utils: Arc<BlockUtils>,
}

/// Handler to create a new memory block.
/// POST /blocks
pub async fn create_block(
    State(state): State<ApiState>,
    Json(block): Json<MemoryBlock>,
) -> Json<serde_json::Value> {
    match state.block_utils.create_block(block).await {
        Ok(id) => Json(json!({ "block_id": id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Handler to get a memory block by ID.
/// GET /blocks/:id
pub async fn get_block(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let bid = BlockId::from(id);
    match state.block_utils.get_block(&bid).await {
        Ok(Some(block)) => Json(json!({ "block": block })),
        Ok(None) => Json(json!({ "error": "Block not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Handler to delete a memory block by ID.
/// DELETE /blocks/:id
pub async fn delete_block(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let bid = BlockId::from(id);
    match state.block_utils.delete_block(&bid).await {
        Ok(_) => Json(json!({ "status": "deleted" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Handler to update a memory block by ID (delete + create).
/// PUT /blocks/:id
pub async fn update_block(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(new_block): Json<MemoryBlock>,
) -> Json<serde_json::Value> {
    let bid = BlockId::from(id);
    match state.block_utils.update_block(&bid, new_block).await {
        Ok(new_id) => Json(json!({ "block_id": new_id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Handler to search for memory blocks.
/// POST /blocks/search
pub async fn search_blocks(
    State(state): State<ApiState>,
    Json(query): Json<MemoryQuery>,
) -> Json<serde_json::Value> {
    match state.block_utils.search_blocks(&query).await {
        Ok(blocks) => Json(json!({ "blocks": blocks })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Handler to list all blocks for a user.
/// GET /blocks/user/:user_id
pub async fn list_blocks_for_user(
    State(state): State<ApiState>,
    Path(user_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.block_utils.list_blocks(&user_id).await {
        Ok(blocks) => Json(json!({ "blocks": blocks })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

/// Register block management routes under /blocks
pub fn block_routes(state: ApiState) -> Router {
    Router::new()
        .route("/blocks", post(create_block))
        .route("/blocks/search", post(search_blocks))
        .route(
            "/blocks/:id",
            get(get_block).delete(delete_block).put(update_block),
        )
        .route("/blocks/user/:user_id", get(list_blocks_for_user))
        .with_state(state)
}
