//! Agent registry for managing multiple agents

use crate::agents::{Agent, AgentMessage, MessageResponse};
use crate::agents::base_agent::{BaseAgent, MessageSender};
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

/// Type alias for agent storage
type AgentMap = Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn Agent>>>>>>;

/// Registry for managing multiple agents and routing messages between them
pub struct AgentRegistry {
    /// Map of agent_id -> agent
    agents: AgentMap,
    
    /// Message routing and delivery
    message_router: MessageRouter,
}

/// Internal message router
struct MessageRouter {
    agents: AgentMap,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        let agents = Arc::new(RwLock::new(HashMap::new()));
        let message_router = MessageRouter {
            agents: agents.clone(),
        };
        
        AgentRegistry {
            agents,
            message_router,
        }
    }
    
    /// Register a new agent
    pub async fn register_agent(&self, agent: Box<dyn Agent>) -> Result<(), Error> {
        let agent_id = agent.agent_id().to_string();
        debug!("Registering agent: {}", agent_id);
        
        // If it's a BaseAgent, inject the message sender
        if let Some(_base_agent) = agent.as_any().downcast_ref::<BaseAgent>() {
            // This would need a proper implementation to set message sender
            // For now, we'll register without the sender injection
        }
        
        let mut agents = self.agents.write().await;
        if agents.contains_key(&agent_id) {
            return Err(anyhow!("Agent with ID {} already exists", agent_id));
        }
        
        agents.insert(agent_id.clone(), Arc::new(RwLock::new(agent)));
        debug!("Successfully registered agent: {}", agent_id);
        Ok(())
    }
    
    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<(), Error> {
        debug!("Unregistering agent: {}", agent_id);
        
        let mut agents = self.agents.write().await;
        agents.remove(agent_id)
            .ok_or_else(|| anyhow!("Agent {} not found", agent_id))?;
        
        debug!("Successfully unregistered agent: {}", agent_id);
        Ok(())
    }
    
    /// Send a message to an agent
    pub async fn send_message(&self, message: AgentMessage) -> Result<(), Error> {
        self.message_router.send_message(message).await
    }
    
    /// Send a message and wait for a response
    pub async fn send_message_and_wait(&self, message: AgentMessage) -> Result<MessageResponse, Error> {
        self.message_router.send_message_and_wait(message).await
    }
    
    /// List all registered agents
    pub async fn list_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }
    
    /// Get agent information
    pub async fn get_agent_info(&self, agent_id: &str) -> Option<(String, String, String)> {
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(agent_id) {
            let agent_guard = agent.read().await;
            Some((agent_guard.agent_id().to_string(), agent_guard.name().to_string(), agent_guard.role().to_string()))
        } else {
            None
        }
    }
    
    /// Check if an agent exists
    pub async fn has_agent(&self, agent_id: &str) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(agent_id)
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageSender for MessageRouter {
    async fn send_message(&self, message: AgentMessage) -> Result<(), Error> {
        debug!("Routing message from {} to {}", message.from_agent_id, message.to_agent_id);
        
        let agents = self.agents.read().await;
        let target_agent = agents.get(&message.to_agent_id)
            .ok_or_else(|| anyhow!("Target agent {} not found", message.to_agent_id))?
            .clone();
        drop(agents); // Release the read lock early
        
        // Process the message asynchronously (fire and forget)
        let result = target_agent.write().await.process_message(message).await;
        match result {
            Ok(response) => {
                debug!("Message processed successfully, response: {:?}", response);
                Ok(())
            }
            Err(e) => {
                error!("Failed to process message: {}", e);
                Err(e)
            }
        }
    }
    
    async fn send_message_and_wait(&self, message: AgentMessage) -> Result<MessageResponse, Error> {
        debug!("Routing message from {} to {} (with response)", message.from_agent_id, message.to_agent_id);
        
        let agents = self.agents.read().await;
        let target_agent = agents.get(&message.to_agent_id)
            .ok_or_else(|| anyhow!("Target agent {} not found", message.to_agent_id))?
            .clone();
        drop(agents); // Release the read lock early
        
        // Process the message and return the response
        let result = target_agent.write().await.process_message(message).await;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentMessage;
    
    
    // Mock agent for testing
    struct MockAgent {
        id: String,
        name: String,
        role: String,
    }
    
    #[async_trait]
    impl Agent for MockAgent {
        fn agent_id(&self) -> &str { &self.id }
        fn name(&self) -> &str { &self.name }
        fn role(&self) -> &str { &self.role }
        
        async fn process_message(&mut self, message: AgentMessage) -> Result<MessageResponse, Error> {
            Ok(MessageResponse::success(
                message.message_id,
                format!("Echo from {}: {}", self.name, message.content),
                None,
            ))
        }
        
        async fn send_message(&self, _message: AgentMessage) -> Result<(), Error> {
            Ok(())
        }
        
        fn get_available_tools(&self) -> Vec<String> {
            vec![]
        }
        
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    
    #[tokio::test]
    async fn test_agent_registration() {
        let registry = AgentRegistry::new();
        
        let agent = Box::new(MockAgent {
            id: "test_agent".to_string(),
            name: "Test Agent".to_string(),
            role: "test".to_string(),
        });
        
        registry.register_agent(agent).await.unwrap();
        assert!(registry.has_agent("test_agent").await);
        
        let agents = registry.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], "test_agent");
    }
    
    #[tokio::test]
    async fn test_message_routing() {
        let registry = AgentRegistry::new();
        
        let agent = Box::new(MockAgent {
            id: "echo_agent".to_string(),
            name: "Echo Agent".to_string(),
            role: "echo".to_string(),
        });
        
        registry.register_agent(agent).await.unwrap();
        
        let message = AgentMessage::new_chat(
            "user".to_string(),
            "echo_agent".to_string(),
            "Hello, agent!".to_string(),
        );
        
        let response = registry.send_message_and_wait(message).await.unwrap();
        assert!(response.success);
        assert!(response.content.contains("Echo from Echo Agent: Hello, agent!"));
    }
}