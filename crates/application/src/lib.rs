use async_trait::async_trait;
use domain::{User, UserRepository};
use std::sync::Arc;

#[async_trait]
pub trait UserService: Send + Sync {
    async fn create_user(&self, username: String, email: String) -> Result<User, String>;
    async fn get_user(&self, id: uuid::Uuid) -> Result<Option<User>, String>;
}

pub struct UserServiceImpl {
    repository: Arc<dyn UserRepository>,
}

impl UserServiceImpl {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn create_user(&self, username: String, email: String) -> Result<User, String> {
        let user = User::new(username, email);
        self.repository.create(&user).await
    }

    async fn get_user(&self, id: uuid::Uuid) -> Result<Option<User>, String> {
        self.repository.find_by_id(id).await
    }
}
