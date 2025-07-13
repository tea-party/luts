//! Agent management API endpoints with SurrealDB persistence
//!
//! This module provides REST API endpoints for managing custom agents,
//! including create, read, update, and delete operations with persistent storage.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get},
    Router,
};
use surrealdb::{Surreal, engine::local::Db, RecordId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Agent configuration structure for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
    pub icon: Option<String>,
    pub color: String,
    pub custom: bool,
    pub system_prompt: Option<String>,
    pub provider: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// SurrealDB representation of an agent with RecordId
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SurrealAgentRecord {
    id: RecordId,
    name: String,
    role: String,
    description: String,
    capabilities: Vec<String>,
    tools: Vec<String>,
    icon: Option<String>,
    color: String,
    custom: bool,
    system_prompt: Option<String>,
    provider: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

impl From<SurrealAgentRecord> for AgentConfig {
    fn from(record: SurrealAgentRecord) -> Self {
        AgentConfig {
            id: record.id.to_string(),
            name: record.name,
            role: record.role,
            description: record.description,
            capabilities: record.capabilities,
            tools: record.tools,
            icon: record.icon,
            color: record.color,
            custom: record.custom,
            system_prompt: record.system_prompt,
            provider: record.provider,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

impl From<&AgentConfig> for SurrealAgentRecord {
    fn from(config: &AgentConfig) -> Self {
        SurrealAgentRecord {
            id: RecordId::from(("agents", config.id.clone())),
            name: config.name.clone(),
            role: config.role.clone(),
            description: config.description.clone(),
            capabilities: config.capabilities.clone(),
            tools: config.tools.clone(),
            icon: config.icon.clone(),
            color: config.color.clone(),
            custom: config.custom,
            system_prompt: config.system_prompt.clone(),
            provider: config.provider.clone(),
            created_at: config.created_at.clone(),
            updated_at: config.updated_at.clone(),
        }
    }
}

impl AgentConfig {
    /// Create a new custom agent configuration
    pub fn new_custom(
        name: String,
        role: String,
        description: String,
        capabilities: Vec<String>,
        tools: Vec<String>,
        icon: Option<String>,
        color: String,
        system_prompt: Option<String>,
        provider: Option<String>,
    ) -> Self {
        let id = Self::generate_id(&name);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        
        Self {
            id,
            name,
            role,
            description,
            capabilities,
            tools,
            icon,
            color,
            custom: true,
            system_prompt,
            provider,
            created_at: Some(now.clone()),
            updated_at: Some(now),
        }
    }

    /// Generate a unique ID from the agent name
    fn generate_id(name: &str) -> String {
        let base_id = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        
        // Add a short UUID suffix to ensure uniqueness
        let suffix = Uuid::new_v4().to_string()[..8].to_string();
        format!("{}-{}", base_id, suffix)
    }

    /// Update the agent configuration
    pub fn update(mut self, updates: AgentConfigUpdate) -> Self {
        if let Some(name) = updates.name {
            self.name = name;
        }
        if let Some(role) = updates.role {
            self.role = role;
        }
        if let Some(description) = updates.description {
            self.description = description;
        }
        if let Some(capabilities) = updates.capabilities {
            self.capabilities = capabilities;
        }
        if let Some(tools) = updates.tools {
            self.tools = tools;
        }
        if let Some(icon) = updates.icon {
            self.icon = Some(icon);
        }
        if let Some(color) = updates.color {
            self.color = color;
        }
        if let Some(system_prompt) = updates.system_prompt {
            self.system_prompt = Some(system_prompt);
        }
        if let Some(provider) = updates.provider {
            self.provider = Some(provider);
        }
        
        self.updated_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string()
        );
        self
    }
}

/// Structure for updating agent configurations
#[derive(Debug, Deserialize)]
pub struct AgentConfigUpdate {
    pub name: Option<String>,
    pub role: Option<String>,
    pub description: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub system_prompt: Option<String>,
    pub provider: Option<String>,
}

/// Structure for creating new agents
#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub role: String,
    pub description: String,
    pub capabilities: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub system_prompt: Option<String>,
    pub provider: Option<String>,
}

/// Shared state for agent API endpoints
pub struct AgentApiState {
    pub db: Arc<Surreal<Db>>,
}

/// Default built-in agents
fn get_default_agents() -> Vec<AgentConfig> {
    vec![
        AgentConfig {
            id: "researcher".to_string(),
            name: "Dr. Research".to_string(),
            role: "Thorough analyst and fact-finder".to_string(),
            description: "A methodical researcher who excels at finding accurate information, analyzing sources, and storing knowledge for future reference.".to_string(),
            capabilities: vec![
                "Web search and source verification".to_string(),
                "Website analysis and data extraction".to_string(),
                "Memory block storage and retrieval".to_string(),
                "Semantic search across stored knowledge".to_string(),
                "Multi-source information synthesis".to_string(),
                "Fact-checking and citation".to_string(),
            ],
            tools: vec!["search".to_string(), "website".to_string(), "memory_blocks".to_string(), "semantic_search".to_string()],
            icon: Some("Search".to_string()),
            color: "from-blue-500 to-cyan-500".to_string(),
            custom: false,
            system_prompt: None,
            provider: None,
            created_at: None,
            updated_at: None,
        },
        AgentConfig {
            id: "calculator".to_string(),
            name: "Logic".to_string(),
            role: "Precise mathematical problem-solver".to_string(),
            description: "A systematic thinker who approaches problems with mathematical precision and methodical step-by-step analysis.".to_string(),
            capabilities: vec![
                "Complex mathematical calculations".to_string(),
                "Step-by-step problem breakdown".to_string(),
                "Pattern recognition in data".to_string(),
                "Error checking and verification".to_string(),
                "Clear explanation of solutions".to_string(),
                "Unit conversion and formatting".to_string(),
            ],
            tools: vec!["calculator".to_string()],
            icon: Some("Calculator".to_string()),
            color: "from-green-500 to-emerald-500".to_string(),
            custom: false,
            system_prompt: None,
            provider: None,
            created_at: None,
            updated_at: None,
        },
        AgentConfig {
            id: "creative".to_string(),
            name: "Spark".to_string(),
            role: "Imaginative and artistic thinker".to_string(),
            description: "A creative mind that generates novel ideas, thinks outside the box, and finds artistic solutions to challenges.".to_string(),
            capabilities: vec![
                "Creative ideation and brainstorming".to_string(),
                "Storytelling and creative writing".to_string(),
                "Unexpected connection making".to_string(),
                "Multiple alternative proposals".to_string(),
                "Aesthetic problem solving".to_string(),
                "Pure reasoning and intuition".to_string(),
            ],
            tools: vec![],
            icon: Some("Lightbulb".to_string()),
            color: "from-purple-500 to-pink-500".to_string(),
            custom: false,
            system_prompt: None,
            provider: None,
            created_at: None,
            updated_at: None,
        },
        AgentConfig {
            id: "coordinator".to_string(),
            name: "Maestro".to_string(),
            role: "Strategic organizer and delegator".to_string(),
            description: "A master coordinator who breaks down complex projects, manages resources, and orchestrates team efforts.".to_string(),
            capabilities: vec![
                "Strategic planning and organization".to_string(),
                "Complex task decomposition".to_string(),
                "Resource coordination and delegation".to_string(),
                "Progress tracking and goal management".to_string(),
                "Cross-project knowledge synthesis".to_string(),
                "Workflow optimization".to_string(),
            ],
            tools: vec!["calculator".to_string(), "search".to_string(), "website".to_string(), "memory_blocks".to_string(), "semantic_search".to_string()],
            icon: Some("Users".to_string()),
            color: "from-orange-500 to-red-500".to_string(),
            custom: false,
            system_prompt: None,
            provider: None,
            created_at: None,
            updated_at: None,
        },
        AgentConfig {
            id: "pragmatic".to_string(),
            name: "Practical".to_string(),
            role: "Efficient and solution-focused".to_string(),
            description: "A pragmatic problem-solver who cuts through complexity to deliver efficient, actionable solutions.".to_string(),
            capabilities: vec![
                "Efficient problem-solving".to_string(),
                "Complexity reduction and focus".to_string(),
                "Actionable advice delivery".to_string(),
                "Trade-off analysis and decisions".to_string(),
                "Minimal-fuss implementation".to_string(),
                "Essential tool utilization".to_string(),
            ],
            tools: vec!["calculator".to_string(), "search".to_string()],
            icon: Some("Wrench".to_string()),
            color: "from-gray-500 to-slate-500".to_string(),
            custom: false,
            system_prompt: None,
            provider: None,
            created_at: None,
            updated_at: None,
        },
    ]
}

/// Get all agents (from database)
pub async fn list_agents(
    State(state): State<Arc<AgentApiState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Listing all agents");
    
    // Ensure default agents are seeded in the database
    if let Err(e) = seed_default_agents(&state.db).await {
        error!("Failed to seed default agents: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to seed default agents".to_string()));
    }
    
    // Load all agents from database
    match load_custom_agents(&state.db).await {
        Ok(agents) => {
            debug!("Loaded {} agents from database", agents.len());
            Ok(Json(agents))
        }
        Err(e) => {
            error!("Failed to load agents: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load agents".to_string()))
        }
    }
}

/// Get a specific agent by ID
pub async fn get_agent(
    State(state): State<Arc<AgentApiState>>,
    Path(agent_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Getting agent: {}", agent_id);
    
    // Ensure default agents are seeded in the database
    if let Err(e) = seed_default_agents(&state.db).await {
        error!("Failed to seed default agents: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to seed default agents".to_string()));
    }
    
    // Load agent from database
    match load_custom_agent(&state.db, &agent_id).await {
        Ok(Some(agent)) => Ok(Json(agent)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Agent not found".to_string())),
        Err(e) => {
            error!("Failed to load agent {}: {}", agent_id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load agent".to_string()))
        }
    }
}

/// Create a new custom agent
pub async fn create_agent(
    State(state): State<Arc<AgentApiState>>,
    Json(request): Json<CreateAgentRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Creating new agent: {}", request.name);
    
    // Validate required fields
    if request.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Agent name is required".to_string()));
    }
    if request.role.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Agent role is required".to_string()));
    }
    if request.description.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Agent description is required".to_string()));
    }
    
    // Ensure default agents are seeded in the database
    if let Err(e) = seed_default_agents(&state.db).await {
        error!("Failed to seed default agents: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to seed default agents".to_string()));
    }
    
    // Check if agent name already exists
    match load_custom_agents(&state.db).await {
        Ok(existing_agents) => {
            if existing_agents.iter().any(|a| a.name == request.name) {
                return Err((StatusCode::CONFLICT, "Agent name already exists".to_string()));
            }
        }
        Err(e) => {
            error!("Failed to check existing agents: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to validate agent name".to_string()));
        }
    }
    
    // Create new agent
    let agent = AgentConfig::new_custom(
        request.name,
        request.role,
        request.description,
        request.capabilities.unwrap_or_default(),
        request.tools.unwrap_or_default(),
        request.icon,
        request.color.unwrap_or_else(|| "from-gray-500 to-slate-500".to_string()),
        request.system_prompt,
        request.provider,
    );
    
    // Save to database
    match save_custom_agent(&state.db, &agent).await {
        Ok(_) => {
            info!("Successfully created agent: {}", agent.id);
            Ok((StatusCode::CREATED, Json(agent)))
        }
        Err(e) => {
            error!("Failed to save agent: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create agent".to_string()))
        }
    }
}

/// Update an existing agent
pub async fn update_agent(
    State(state): State<Arc<AgentApiState>>,
    Path(agent_id): Path<String>,
    Json(updates): Json<AgentConfigUpdate>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Updating agent: {}", agent_id);
    
    // Ensure default agents are seeded in the database
    if let Err(e) = seed_default_agents(&state.db).await {
        error!("Failed to seed default agents: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to seed default agents".to_string()));
    }
    
    // Load existing agent
    match load_custom_agent(&state.db, &agent_id).await {
        Ok(Some(agent)) => {
            let updated_agent = agent.update(updates);
            
            // Save updated agent
            match save_custom_agent(&state.db, &updated_agent).await {
                Ok(_) => {
                    info!("Successfully updated agent: {}", agent_id);
                    Ok(Json(updated_agent))
                }
                Err(e) => {
                    error!("Failed to save updated agent: {}", e);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update agent".to_string()))
                }
            }
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "Agent not found".to_string())),
        Err(e) => {
            error!("Failed to load agent for update: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load agent".to_string()))
        }
    }
}

/// Delete an agent
pub async fn delete_agent(
    State(state): State<Arc<AgentApiState>>,
    Path(agent_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Deleting agent: {}", agent_id);
    
    // Ensure default agents are seeded in the database
    if let Err(e) = seed_default_agents(&state.db).await {
        error!("Failed to seed default agents: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to seed default agents".to_string()));
    }
    
    // Load existing agent
    match load_custom_agent(&state.db, &agent_id).await {
        Ok(Some(agent)) => {
            // Delete from database
            match delete_custom_agent(&state.db, &agent_id).await {
                Ok(_) => {
                    info!("Successfully deleted agent: {}", agent_id);
                    Ok(Json(serde_json::json!({
                        "message": "Agent deleted successfully",
                        "agent": agent
                    })))
                }
                Err(e) => {
                    error!("Failed to delete agent: {}", e);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete agent".to_string()))
                }
            }
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "Agent not found".to_string())),
        Err(e) => {
            error!("Failed to load agent for deletion: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load agent".to_string()))
        }
    }
}

/// Seed default agents into the database if they don't exist
async fn seed_default_agents(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Seeding default agents into database");

    // Check if we already have agents in the database
    let existing_agents = load_custom_agents(db).await?;
    
    // Get default agent IDs
    let default_agent_ids: Vec<&str> = vec!["researcher", "calculator", "creative", "coordinator", "pragmatic"];
    
    // Check if all default agents exist
    let has_all_defaults = default_agent_ids.iter().all(|&id| {
        existing_agents.iter().any(|agent| agent.id == id)
    });
    
    if has_all_defaults {
        debug!("All default agents already exist in database");
        return Ok(());
    }
    
    info!("Seeding default agents into database");
    
    let default_agents = get_default_agents();
    for agent in default_agents {
        // Only seed if the agent doesn't already exist
        if !existing_agents.iter().any(|existing| existing.id == agent.id) {
            if let Err(e) = save_custom_agent(db, &agent).await {
                error!("Failed to seed agent {}: {}", agent.id, e);
            } else {
                debug!("Seeded default agent: {}", agent.id);
            }
        }
    }
    
    info!("Finished seeding default agents");
    Ok(())
}

/// Load all custom agents from database
async fn load_custom_agents(db: &Surreal<Db>) -> Result<Vec<AgentConfig>, Box<dyn std::error::Error + Send + Sync>> {
    debug!("Loading custom agents from database");
    
    // Use strongly-typed struct approach (recommended by SurrealDB docs)
    let records: Vec<SurrealAgentRecord> = db.select("agents").await?;
    debug!("Loaded {} custom agents", records.len());
    
    let agents: Vec<AgentConfig> = records.into_iter().map(|r| r.into()).collect();
    debug!("Successfully processed {} agents", agents.len());
    Ok(agents)
}

/// Load a specific custom agent from database
async fn load_custom_agent(db: &Surreal<Db>, agent_id: &str) -> Result<Option<AgentConfig>, Box<dyn std::error::Error + Send + Sync>> {
    debug!("Loading custom agent: {}", agent_id);
    
    // Use strongly-typed struct approach (recommended by SurrealDB docs)
    let record: Option<SurrealAgentRecord> = db.select(("agents", agent_id)).await?;
    debug!("Found agent: {:?}", record.is_some());
    
    Ok(record.map(|r| r.into()))
}

/// Save a custom agent to database
async fn save_custom_agent(db: &Surreal<Db>, agent: &AgentConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Saving custom agent: {} ({})", agent.name, agent.id);
    
    // Use strongly-typed struct approach (recommended by SurrealDB docs)
    let record = SurrealAgentRecord::from(agent);
    let _: Option<SurrealAgentRecord> = db.create(("agents", &agent.id)).content(record).await?;
    
    debug!("Successfully saved agent: {}", agent.id);
    Ok(())
}

/// Delete a custom agent from database
async fn delete_custom_agent(db: &Surreal<Db>, agent_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Deleting custom agent: {}", agent_id);
    
    // Use strongly-typed struct approach (recommended by SurrealDB docs)
    let _: Option<SurrealAgentRecord> = db.delete(("agents", agent_id)).await?;
    
    debug!("Successfully deleted agent: {}", agent_id);
    Ok(())
}

/// Create router for agent API endpoints
pub fn agent_routes(state: AgentApiState) -> Router {
    Router::new()
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/:id", get(get_agent).put(update_agent).delete(delete_agent))
        .with_state(Arc::new(state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_generation() {
        let agent = AgentConfig::new_custom(
            "Test Agent".to_string(),
            "Test Role".to_string(),
            "Test Description".to_string(),
            vec![],
            vec![],
            None,
            "from-blue-500 to-cyan-500".to_string(),
            None,
            None,
        );
        
        assert!(agent.id.starts_with("test-agent-"));
        assert!(agent.custom);
        assert!(agent.created_at.is_some());
        assert!(agent.updated_at.is_some());
    }

    #[test]
    fn test_default_agents() {
        let agents = get_default_agents();
        assert_eq!(agents.len(), 5);
        assert!(agents.iter().all(|a| !a.custom));
        assert!(agents.iter().any(|a| a.id == "researcher"));
        assert!(agents.iter().any(|a| a.id == "calculator"));
        assert!(agents.iter().any(|a| a.id == "creative"));
        assert!(agents.iter().any(|a| a.id == "coordinator"));
        assert!(agents.iter().any(|a| a.id == "pragmatic"));
    }
}