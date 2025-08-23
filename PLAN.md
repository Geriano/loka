# Loka-Stratum Database Integration Plan

## Overview
Integrate SeaORM database layer to replace in-memory submission handling with persistent storage, enabling production-ready mining pool operations with proper data persistence and analytics.

## Current State Analysis
- **Memory-based**: Submissions currently handled in memory (lost on restart)
- **Database ready**: Migration and model layer completed with 6 tables
- **Architecture**: SeaORM entities and methods already implemented

## Technical Implementation Strategy

### Phase 1: Database Foundation
**1. Database Connection Integration**
```rust
// Add to main.rs or cli/commands.rs
use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use migration::{Migrator, MigratorTrait};

pub struct DatabaseService {
    pub connection: DatabaseConnection,
}

impl DatabaseService {
    pub async fn new(database_url: &str) -> Result<Self> {
        let mut opt = ConnectOptions::new(database_url.to_owned());
        // Configure SeaORM's built-in connection pool
        opt.max_connections(100)
           .min_connections(5)
           .connect_timeout(Duration::from_secs(8))
           .idle_timeout(Duration::from_secs(8));
        
        let connection = Database::connect(opt).await?;
        
        // Run migrations automatically
        Migrator::up(&connection, None).await?;
        
        Ok(DatabaseService { connection })
    }
}
```

**2. Configuration Extension**
```toml
# Add to loka-stratum.toml
[database]
url = "postgresql://localhost:5432/loka_stratum"
max_connections = 100
min_connections = 5
```

**3. CLI Integration**
```rust
// Add to cli/args.rs
#[derive(Parser)]
pub enum Commands {
    Start(StartArgs),
    Migrate(MigrateArgs), // New command
}

#[derive(Parser)]
pub struct MigrateArgs {
    #[arg(long)]
    pub up: bool,
    #[arg(long)]
    pub down: Option<u32>,
}
```

### Phase 2: Integrate Database with Existing Architecture
**1. Add Database Service to Manager**
```rust
// Update src/manager.rs to include database service
pub struct Manager {
    auth: auth::Manager,
    jobs: job::Manager, 
    submissions: submission::Manager,
    database: Arc<DatabaseService>, // Add database service
}

impl Manager {
    pub fn new(config: Arc<Config>, database: Arc<DatabaseService>) -> Self {
        Self {
            auth: auth::Manager::default(),
            jobs: job::Manager::default(),
            submissions: submission::Manager::default(),
            database,
        }
    }
    
    pub fn database(&self) -> &Arc<DatabaseService> {
        &self.database
    }
}
```

**2. Enhance Existing Authentication Handler**
```rust
// Update protocol/handler.rs handle_parsed_message for StratumMessage::Authenticate
StratumMessage::Authenticate { user, worker, .. } => {
    // Existing in-memory authentication (keep for performance)
    manager
        .submissions()
        .authenticated(manager.auth().authenticate(addr, user, worker));

    // Add database storage for persistence (async, non-blocking)
    tokio::spawn({
        let db = manager.database().clone();
        let pool_id = get_current_pool_id(); // Get from config or path
        let user = user.clone();
        let worker = worker.clone();
        async move {
            if let Err(e) = workers::Model::first_or_create(
                &db.connection,
                pool_id,
                &user,
                &worker,
            ).await {
                error!("Failed to store miner/worker: {}", e);
            }
        }
    });

    // Continue with existing upstream message modification
    let mut upstream_message = serde_json::from_str::<Value>(raw_message)?;
    // ... existing code
}
```

**3. Enhance Existing Submission Handler**
```rust
// Update protocol/handler.rs handle_parsed_message for StratumMessage::Submit
StratumMessage::Submit { id, job_id } => {
    // Existing in-memory submission tracking (now with database storage)
    let jobs = manager.jobs();
    if let Some(job) = jobs.get(job_id) {
        if let Some(auth) = manager.auth().get(&addr) {
            // Enhanced submit method will handle both in-memory and database storage
            manager.submissions().submit(*id, job, auth);
        }
    }

    // Continue with existing message forwarding
    let original_value = serde_json::from_str::<Value>(raw_message)?;
    downstream_to_upstream_tx.send(original_value).map_err(|e| {
        // ... existing error handling
    })
}
```

**4. Enhanced Submission Manager**
```rust
// Update submission/manager.rs to include database storage
impl submission::Manager {
    pub fn new(database: Option<Arc<DatabaseService>>) -> Self {
        Self {
            authenticated: DashMap::new(),
            database,
        }
    }

    pub fn submit(&self, id: u64, job: Arc<job::State>, auth: Arc<auth::State>) {
        // Existing in-memory logic only
        if let Some(auth_submission) = self.authenticated.get(&auth) {
            auth_submission.submit(id, job);
        }
    }
    
    pub fn submitted(&self, id: u64, valid: bool, auth: Arc<auth::State>) -> Option<Arc<Submit>> {
        // Existing in-memory logic
        let result = if let Some(auth_submission) = self.authenticated.get(&auth) {
            auth_submission.submitted(&id, valid)
        } else {
            None
        };
        
        // Store completed submission to database (async, non-blocking)
        if let Some(db) = &self.database {
            tokio::spawn({
                let db = db.clone();
                let auth = auth.clone();
                async move {
                    let pool_id = get_current_pool_id(); // From config
                    if let Ok((_miner, worker)) = workers::Model::first_or_create(
                        &db.connection,
                        pool_id,
                        auth.user(),
                        auth.worker(),
                    ).await {
                        // Get hashes from the completed submission result
                        let hashes = if let Some(submit) = &result {
                            if valid {
                                submit.accepted() as f64
                            } else {
                                submit.rejected() as f64
                            }
                        } else {
                            1.0 // Default hash count
                        };
                        
                        // Store final submission with known validity
                        if let Err(e) = submissions::Model::store(
                            &db.connection,
                            &worker,
                            hashes,
                            valid,
                        ).await {
                            error!("Failed to store submission: {}", e);
                        }
                    }
                }
            });
        }
        
        result
    }
}
```

**5. Simplified Pool Response Handler**
```rust
// Existing submission.submitted() call now handles database updates
if let Some(auth) = manager.auth().get_by_submission_id(&id) {
    manager.submissions().submitted(id, is_valid, auth); // Database update inside
}
```

**6. Submission Validity Strategy**
- **Pool Response Validation**: Validity determined by pool's response to `mining.submit`
- **Single-Phase Storage**: Store submission only when validity is known from pool response
- **Complete Data**: Store final submission with known validity and hash counts
- **No Pending State**: No need to track pending submissions or update validity later
- **Pool Authority**: Pool response (`result: true/false`) determines final validity

**7. Non-Blocking Design Principles**
- **Keep Existing Performance**: In-memory structures remain for immediate mining responses
- **Add Database Persistence**: `tokio::spawn` for async database operations  
- **Dual Architecture**: Fast in-memory + persistent database storage
- **No Breaking Changes**: Existing mining operations continue unchanged
- **Gradual Enhancement**: Database features enhance rather than replace existing functionality

### Phase 3: Pool Path Integration  
**1. Use Existing Pool Configuration**
```rust
// Update connection handler to use pre-configured pool
impl ConnectionHandler {
    // Pool ID should be set during handler initialization from configuration
    // No need to create pools dynamically - assume they exist in database
    async fn new(database: Arc<DatabaseService>, pool_id: Uuid, /* other params */) -> Result<Self> {
        Ok(ConnectionHandler {
            database,
            pool_id, // Use existing pool from database
            // ... other fields
        })
    }
}
```

**2. Path-Based Pool Selection (Future Enhancement)**
```rust
// Future enhancement: pool selection based on extracted HTTP CONNECT path
impl ConnectionHandler {
    async fn select_pool_by_path(&self, path: Option<String>) -> Result<Uuid> {
        match path {
            Some(path_name) => {
                info!("Pool path detected: {}", path_name);
                // TODO: Add pool lookup by path/name once needed
                // For now, use configured pool
                Ok(self.pool_id)
            }
            None => {
                info!("No path provided, using configured pool");
                Ok(self.pool_id)
            }
        }
    }
}
```

### Phase 4: Analytics Integration
**1. Database-Backed Metrics**
```rust
// Add to metrics service
impl MetricsService {
    pub async fn get_submission_rate(&self, db: &DatabaseConnection) -> Result<f64> {
        let count = submissions::Entity::find()
            .filter(submissions::Column::SubmittedAt.gte(Utc::now().naive_utc() - Duration::minutes(1)))
            .count(db)
            .await?;
        
        Ok(count as f64 / 60.0) // submissions per second
    }
    
    pub async fn export_database_metrics(&self, db: &DatabaseConnection) -> String {
        let mut output = String::new();
        
        // Active miners (updated in last hour)  
        let active_miners = workers::Entity::find()
            .filter(workers::Column::LastSeen.gte(Utc::now().naive_utc() - Duration::hours(1)))
            .count(db)
            .await
            .unwrap_or(0);
        output.push_str(&format!("loka_active_miners {}\n", active_miners));
        
        // Total submissions
        let total_submissions = submissions::Entity::find().count(db).await.unwrap_or(0);
        output.push_str(&format!("loka_total_submissions {}\n", total_submissions));
        
        // Valid vs invalid submissions
        let valid_submissions = submissions::Entity::find()
            .filter(submissions::Column::Valid.eq(true))
            .count(db)
            .await
            .unwrap_or(0);
        output.push_str(&format!("loka_valid_submissions {}\n", valid_submissions));
        
        let invalid_submissions = submissions::Entity::find()
            .filter(submissions::Column::Valid.eq(false))
            .count(db)
            .await
            .unwrap_or(0);
        output.push_str(&format!("loka_invalid_submissions {}\n", invalid_submissions));
        
        // Total hashrate from submissions
        let total_hashrate: Option<Decimal> = submissions::Entity::find()
            .filter(submissions::Column::SubmittedAt.gte(Utc::now().naive_utc() - Duration::hours(1)))
            .sum(submissions::Column::Hashes)
            .one(db)
            .await
            .unwrap_or(None);
        
        if let Some(hashrate) = total_hashrate {
            output.push_str(&format!("loka_total_hashrate {}\n", hashrate));
        }
        
        output
    }
}
```

**2. Distribution Management**
Your existing `submissions::Model::distribute()` method handles:
- Daily hashrate aggregation with timezone offsets
- Automatic cleanup of processed submissions
- Raw SQL for performance with large datasets

## Performance & Architecture Considerations

### Database Performance
- **SeaORM Connection Pool**: Configure built-in pool with 100 max connections for high-concurrency mining
- **Async Non-Blocking**: Use `tokio::spawn` for database writes to avoid blocking mining responses
- **Batch Operations**: Group multiple submissions for bulk inserts during high activity
- **Indexing Strategy** (your migrations may already include these):
  ```sql
  CREATE INDEX idx_submissions_worker_submitted ON submissions(worker_id, submitted_at DESC);
  CREATE INDEX idx_submissions_submitted ON submissions(submitted_at);
  CREATE INDEX idx_miners_username ON miners(username);
  CREATE INDEX idx_workers_pool_miner_name ON workers(pool_id, miner_id, name);
  ```

### Error Handling & Reliability
```rust
// Graceful degradation pattern
async fn store_submission_with_fallback(&self, submission: SubmissionData) -> Result<()> {
    match submissions::create_submission(&self.database.connection, submission.clone()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            warn!("Database write failed, continuing mining: {}", e);
            // Log to file or memory buffer as fallback
            self.fallback_storage.store(submission).await?;
            Ok(())
        }
    }
}
```

### Memory Management
- **Connection Reuse**: Reuse database connections across handler instances
- **Prepared Statements**: Use SeaORM's built-in statement preparation
- **Metrics Caching**: Cache frequently accessed metrics to reduce database load

## Implementation Order & Dependencies

### Phase 1: Foundation (Required First)
1. **Database Service Setup** - Core connection management
2. **Configuration Integration** - TOML parsing for database settings
3. **Migration Runner** - Automatic schema setup

### Phase 2: Core Integration (Critical Path)
1. **Handler Database Injection** - Pass DatabaseService to ConnectionHandler
2. **Submission Storage** - Replace in-memory with database persistence
3. **Miner Auto-Registration** - Create miner/worker records on connection

### Phase 3: Advanced Features (Incremental)
1. **Pool Path Selection** - Database-driven pool routing
2. **Analytics Integration** - Database-backed metrics export

### Phase 4: Production Features (Optional)
1. **Performance Monitoring** - Database performance metrics
2. **Admin APIs** - Database management endpoints

## Testing Strategy
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, DatabaseBackend, MockDatabase, MockExecResult};

    #[tokio::test]
    async fn test_submission_storage() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let result = submissions::create_submission(&db, test_submission_data()).await;
        assert!(result.is_ok());
    }
}
```

## Estimated Implementation Time
- **Phase 1**: 3-4 hours (database foundation)
- **Phase 2**: 5-6 hours (core submission handling)  
- **Phase 3**: 4-5 hours (advanced features)
- **Phase 4**: 3-4 hours (production features)

**Total**: 15-19 hours for complete implementation