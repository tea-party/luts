use crate::context::ContextProvider;
use anyhow::{Error, Result};
use async_trait::async_trait;
use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle, PersistMode};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// FjallContextProvider implements the ContextProvider trait using Fjäll as the storage backend.
/// Each namespace is mapped to a separate partition.
pub struct FjallContextProvider {
    keyspace: Arc<Keyspace>,
    partition: PartitionHandle,
    _namespace: String,
}

impl FjallContextProvider {
    /// Create a new FjallContextProvider with the given data directory path and default namespace "context"
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        Self::with_namespace(data_dir, "context")
    }

    /// Create a new FjallContextProvider with a custom namespace (partition)
    pub fn with_namespace<P: AsRef<Path>>(data_dir: P, namespace: &str) -> Result<Self> {
        let keyspace = Config::new(data_dir).open()?;
        let partition = keyspace.open_partition(namespace, PartitionCreateOptions::default())?;
        Ok(Self {
            keyspace: Arc::new(keyspace),
            partition,
            _namespace: namespace.to_string(),
        })
    }
}

#[async_trait]
impl ContextProvider for FjallContextProvider {
    async fn store(&self, id: &str, data: &Value) -> Result<(), Error> {
        let value = serde_json::to_vec(data)?;
        let key = id.as_bytes().to_vec();
        let partition = self.partition.clone();
        let keyspace = self.keyspace.clone();

        // Use spawn_blocking since fjall is sync
        tokio::task::spawn_blocking(move || {
            partition.insert(key, value)?;
            keyspace.persist(PersistMode::Buffer)?;
            Ok(())
        })
        .await?
    }

    async fn retrieve(&self, id: &str) -> Result<Option<Value>, Error> {
        let key = id.as_bytes().to_vec();
        let partition = self.partition.clone();
        let result = tokio::task::spawn_blocking(move || partition.get(key)).await??;
        match result {
            Some(bytes) => {
                let value: Value = serde_json::from_slice(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &str) -> Result<(), Error> {
        let key = id.as_bytes().to_vec();
        let partition = self.partition.clone();
        let keyspace = self.keyspace.clone();
        tokio::task::spawn_blocking(move || {
            partition.remove(key)?;
            keyspace.persist(PersistMode::Buffer)?;
            Ok(())
        })
        .await?
    }

    async fn exists(&self, id: &str) -> Result<bool, Error> {
        let key = id.as_bytes().to_vec();
        let partition = self.partition.clone();
        let result = tokio::task::spawn_blocking(move || partition.get(key)).await??;
        Ok(result.is_some())
    }

    fn name(&self) -> &str {
        "fjall"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fjall_provider() -> Result<()> {
        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        let provider = FjallContextProvider::new(temp_dir.path())?;

        // Test data
        let id = "test_id";
        let data = json!({"name": "Test", "value": 42});

        // Store data
        provider.store(id, &data).await?;

        // Check if data exists
        assert!(provider.exists(id).await?);

        // Retrieve data
        let retrieved = provider.retrieve(id).await?;
        assert_eq!(retrieved, Some(data.clone()));

        // Delete data
        provider.delete(id).await?;

        // Verify deletion
        assert!(!provider.exists(id).await?);
        assert_eq!(provider.retrieve(id).await?, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_namespaced_provider() -> Result<()> {
        // Create a temporary directory for the test
        let temp_dir = tempdir()?;

        // Create two providers with different namespaces
        let provider1 = FjallContextProvider::with_namespace(temp_dir.path(), "ns1")?;
        let provider2 = FjallContextProvider::with_namespace(temp_dir.path(), "ns2")?;

        // Test data
        let id = "same_id";
        let data1 = json!({"provider": 1});
        let data2 = json!({"provider": 2});

        // Store data in both providers
        provider1.store(id, &data1).await?;
        provider2.store(id, &data2).await?;

        // Retrieve data from both providers
        let retrieved1 = provider1.retrieve(id).await?;
        let retrieved2 = provider2.retrieve(id).await?;

        // Verify each provider has its own data
        assert_eq!(retrieved1, Some(data1));
        assert_eq!(retrieved2, Some(data2));

        Ok(())
    }
}
