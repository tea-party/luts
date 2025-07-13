//! Streaming response management
//!
//! This module contains the streaming response manager for handling
//! real-time AI responses with tool calling support.

pub mod manager;

// Re-export key types for convenience
pub use manager::{
    ChunkType, ResponseChunk, ResponseStreamManager, StreamConfig, StreamEvent, StreamableResponse,
    StreamingResponseBuilder, TypingIndicator, TypingStatus,
};