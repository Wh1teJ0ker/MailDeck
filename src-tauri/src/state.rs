use std::sync::Arc;

pub struct AppState {
    pub db: Arc<tokio::sync::Mutex<crate::store::Store>>,
}
