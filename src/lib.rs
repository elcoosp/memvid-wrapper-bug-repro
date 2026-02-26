use chrono::Utc;
use memvid_core::{AclContext, AclEnforcementMode, Memvid, PutOptions, SearchRequest};
use serde_json;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use tokio::sync::Mutex;

// -----------------------------------------------------------------------------
// Wrapper that mimics your MemvidStore (simplified)
// -----------------------------------------------------------------------------
pub struct MemvidStore {
    inner: Mutex<Memvid>,
    path: PathBuf,
}

impl MemvidStore {
    pub async fn open_or_create(project_root: &Path) -> Result<Self, String> {
        let dir = project_root.join(".codebridge");
        std::fs::create_dir_all(&dir).map_err(|e| format!("dir creation: {}", e))?;
        let path = dir.join("sessions.mv2");

        let mem = if path.exists() {
            Memvid::open(&path).map_err(|e| format!("open: {}", e))?
        } else {
            Memvid::create(&path).map_err(|e| format!("create: {}", e))?
        };

        let store = Self {
            inner: Mutex::new(mem),
            path,
        };

        // Enable lexical index while holding the lock
        {
            let mut mem = store.inner.lock().await;
            mem.enable_lex().map_err(|e| format!("enable_lex: {}", e))?;
            mem.commit()
                .map_err(|e| format!("commit after enable: {}", e))?;
        }

        Ok(store)
    }

    pub async fn append_frame(&self, content: &str, session_id: &str) -> Result<(), String> {
        let opts = PutOptions::builder()
            .title("Test Frame")
            .uri(&format!("session://{}/test-id", session_id))
            .tag("session_id", session_id)
            .tag("message", content)
            .build();

        let mut mem = self.inner.lock().await;
        mem.enable_lex()
            .map_err(|e| format!("enable_lex before put: {}", e))?;
        mem.put_bytes_with_options(content.as_bytes(), opts)
            .map_err(|e| format!("put: {}", e))?;
        mem.commit().map_err(|e| format!("commit: {}", e))?;
        Ok(())
    }

    pub async fn search_text(&self, query: &str) -> Result<Vec<String>, String> {
        let mut mem = self.inner.lock().await;
        mem.enable_lex()
            .map_err(|e| format!("enable_lex before search: {}", e))?;

        let acl_context = AclContext {
            tenant_id: None,
            subject_id: Some("anonymous".to_string()),
            roles: vec![],
            group_ids: vec![],
        };

        let request = SearchRequest {
            query: query.to_string(),
            top_k: 10,
            snippet_chars: 500,
            uri: None,
            scope: None,
            cursor: None,
            as_of_frame: None,
            as_of_ts: None,
            no_sketch: false,
            acl_context: Some(acl_context),
            acl_enforcement_mode: AclEnforcementMode::Audit,
        };

        let response = mem.search(request).map_err(|e| format!("search: {}", e))?;
        Ok(response.hits.into_iter().map(|h| h.text).collect())
    }

    pub async fn get_session_frames(&self, session_id: &str) -> Result<Vec<String>, String> {
        // Use tag search
        let query = format!("session_id:{}", session_id);
        self.search_text(&query).await
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_raw_with_mutex_passes() {
    use tokio::sync::Mutex;
    let dir = tempdir().unwrap();
    let codebridge = dir.path().join(".codebridge");
    std::fs::create_dir_all(&codebridge).unwrap();
    let path = codebridge.join("sessions.mv2");

    let memvid = if path.exists() {
        Memvid::open(&path).unwrap()
    } else {
        Memvid::create(&path).unwrap()
    };
    let store = Mutex::new(memvid);

    // Enable index while locked
    {
        let mut mem = store.lock().await;
        mem.enable_lex().unwrap();
        mem.commit().unwrap();
    }

    // Insert a frame
    {
        let mut mem = store.lock().await;
        let opts = PutOptions::builder()
            .title("Test")
            .uri("session://test-session/test-id")
            .tag("session_id", "test-session")
            .tag("message", "Fix the bug in login")
            .build();
        mem.put_bytes_with_options(b"Fix the bug in login", opts)
            .unwrap();
        mem.commit().unwrap();
    }

    // Search for "login"
    {
        let mut mem = store.lock().await;
        mem.enable_lex().unwrap();
        let acl_context = AclContext {
            tenant_id: None,
            subject_id: Some("anonymous".to_string()),
            roles: vec![],
            group_ids: vec![],
        };
        let request = SearchRequest {
            query: "login".to_string(),
            top_k: 5,
            snippet_chars: 300,
            uri: None,
            scope: None,
            cursor: None,
            as_of_frame: None,
            as_of_ts: None,
            no_sketch: false,
            acl_context: Some(acl_context),
            acl_enforcement_mode: AclEnforcementMode::Audit,
        };
        let response = mem.search(request).unwrap();
        assert!(!response.hits.is_empty(), "Should find the frame");
        println!("raw test passed, hits: {:?}", response.hits);
    }
}

#[tokio::test]
async fn test_wrapper_fails() {
    let dir = tempdir().unwrap();
    let store = MemvidStore::open_or_create(dir.path()).await.unwrap();

    // Insert a frame
    store
        .append_frame("Fix the bug in login", "test-session")
        .await
        .unwrap();

    // Allow some time for indexing
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Search for "login"
    let results = store.search_text("login").await.unwrap();
    assert_eq!(results.len(), 1, "Should find the frame with 'login'");

    // Also test tag search
    let session_results = store.get_session_frames("test-session").await.unwrap();
    assert_eq!(session_results.len(), 1, "Should find frame by session_id");
}
