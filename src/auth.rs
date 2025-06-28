use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use warp::{Filter, Rejection, reject};
use base64::prelude::*;

const JWT_SECRET: &[u8] = b"your-secret-key-change-this-in-production";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub api_keys: Vec<ApiKey>,
    pub quota: ResourceQuota,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub user_id: String,
    pub permissions: Vec<Permission>,
    pub rate_limit: RateLimit,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    CreateEmulator,
    DeleteEmulator,
    ReadEmulator,
    WriteEmulator,
    ManageSnapshots,
    ViewMetrics,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub max_concurrent_emulators: u32,
    pub max_cpu_cycles_per_second: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub max_emulators: u32,
    pub max_memory_per_emulator: u64,
    pub max_storage_mb: u64,
    pub max_api_calls_per_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub username: String,
    pub permissions: Vec<Permission>,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Vec<Permission>,
    pub expires_in_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub quota: ResourceQuota,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub key: String, // Only returned on creation
    pub permissions: Vec<Permission>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub type UserStore = Arc<Mutex<HashMap<String, User>>>;
pub type SessionStore = Arc<Mutex<HashMap<String, DateTime<Utc>>>>; // user_id -> last_activity

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_emulators: 5,
            max_memory_per_emulator: 64 * 1024, // 64KB
            max_storage_mb: 100,
            max_api_calls_per_hour: 1000,
        }
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            max_concurrent_emulators: 3,
            max_cpu_cycles_per_second: 1_000_000,
        }
    }
}

impl User {
    pub fn new(username: String, email: String, password: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let password_hash = hash(password, DEFAULT_COST)?;
        
        Ok(User {
            id: Uuid::new_v4().to_string(),
            username,
            email,
            password_hash,
            api_keys: Vec::new(),
            quota: ResourceQuota::default(),
            created_at: Utc::now(),
            is_active: true,
        })
    }
    
    pub fn verify_password(&self, password: &str) -> bool {
        verify(password, &self.password_hash).unwrap_or(false)
    }
    
    pub fn create_api_key(&mut self, name: String, permissions: Vec<Permission>, expires_in_days: Option<u32>) -> ApiKeyResponse {
        let key_id = Uuid::new_v4().to_string();
        let raw_key = generate_api_key();
        let key_hash = hash_api_key(&raw_key);
        
        let expires_at = expires_in_days.map(|days| {
            Utc::now() + Duration::days(days as i64)
        });
        
        let api_key = ApiKey {
            id: key_id.clone(),
            name: name.clone(),
            key_hash,
            user_id: self.id.clone(),
            permissions: permissions.clone(),
            rate_limit: RateLimit::default(),
            created_at: Utc::now(),
            last_used: None,
            expires_at,
            is_active: true,
        };
        
        self.api_keys.push(api_key);
        
        ApiKeyResponse {
            id: key_id,
            name,
            key: raw_key,
            permissions,
            expires_at,
        }
    }
    
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.api_keys.iter().any(|key| {
            key.is_active && 
            key.expires_at.map_or(true, |exp| exp > Utc::now()) &&
            (key.permissions.contains(permission) || key.permissions.contains(&Permission::Admin))
        })
    }
}

pub fn generate_api_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    format!("mos6502_{}", BASE64_STANDARD.encode(bytes))
}

pub fn hash_api_key(key: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn create_jwt_token(user: &User) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now() + Duration::hours(24);
    
    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        permissions: user.api_keys.iter()
            .filter(|key| key.is_active)
            .flat_map(|key| key.permissions.iter())
            .cloned()
            .collect(),
        exp: expiration.timestamp(),
        iat: Utc::now().timestamp(),
    };
    
    encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))
}

pub fn verify_jwt_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::new(Algorithm::HS256),
    ).map(|data| data.claims)
}

pub fn authenticate_api_key(users: UserStore, key: &str) -> Result<User, AuthError> {
    let key_hash = hash_api_key(key);
    let users_lock = users.lock().unwrap();
    
    for user in users_lock.values() {
        if !user.is_active {
            continue;
        }
        
        for api_key in &user.api_keys {
            if api_key.key_hash == key_hash && 
               api_key.is_active &&
               api_key.expires_at.map_or(true, |exp| exp > Utc::now()) {
                return Ok(user.clone());
            }
        }
    }
    
    Err(AuthError::InvalidApiKey)
}

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    InvalidApiKey,
    InsufficientPermissions,
    RateLimitExceeded,
    QuotaExceeded,
    UserNotFound,
    UserInactive,
}

impl reject::Reject for AuthError {}

pub fn with_auth(users: UserStore) -> impl Filter<Extract = (User,), Error = Rejection> + Clone {
    warp::header::<String>("authorization")
        .and_then(move |auth_header: String| {
            let users = users.clone();
            async move {
                if let Some(token) = auth_header.strip_prefix("Bearer ") {
                    // JWT token authentication
                    match verify_jwt_token(token) {
                        Ok(claims) => {
                            let users_lock = users.lock().unwrap();
                            if let Some(user) = users_lock.get(&claims.sub) {
                                if user.is_active {
                                    return Ok(user.clone());
                                }
                            }
                        }
                        Err(_) => {}
                    }
                } else if let Some(api_key) = auth_header.strip_prefix("ApiKey ") {
                    // API key authentication
                    match authenticate_api_key(users, api_key) {
                        Ok(user) => return Ok(user),
                        Err(_) => {}
                    }
                }
                
                Err(reject::custom(AuthError::InvalidCredentials))
            }
        })
}

// TODO: Fix filter type issues
// pub fn require_permission(permission: Permission) -> impl Filter<Extract = (), Error = Rejection> + Clone {
//     warp::any()
//         .map(move || {
//             let _perm = permission.clone();
//             // This will be combined with with_auth to check permissions
//             ()
//         })
// }

// Middleware to check permissions after authentication
pub fn with_permission(
    users: UserStore,
    permission: Permission,
) -> impl Filter<Extract = (User,), Error = Rejection> + Clone {
    with_auth(users)
        .and_then(move |user: User| {
            let perm = permission.clone();
            async move {
                if user.has_permission(&perm) {
                    Ok(user)
                } else {
                    Err(reject::custom(AuthError::InsufficientPermissions))
                }
            }
        })
}

pub fn init_default_users(users: UserStore) {
    let mut users_lock = users.lock().unwrap();
    
    // Create admin user
    if let Ok(mut admin) = User::new(
        "admin".to_string(),
        "admin@localhost".to_string(),
        "admin123"
    ) {
        let api_key = admin.create_api_key(
            "Default Admin Key".to_string(),
            vec![Permission::Admin],
            None,
        );
        
        println!("Created admin user with API key: {}", api_key.key);
        println!("Use this key in the Authorization header: ApiKey {}", api_key.key);
        
        users_lock.insert(admin.id.clone(), admin);
    }
    
    // Create demo user
    if let Ok(mut demo) = User::new(
        "demo".to_string(),
        "demo@localhost".to_string(),
        "demo123"
    ) {
        let api_key = demo.create_api_key(
            "Demo Key".to_string(),
            vec![
                Permission::CreateEmulator,
                Permission::ReadEmulator,
                Permission::WriteEmulator,
                Permission::ViewMetrics,
            ],
            Some(30), // 30 days
        );
        
        println!("Created demo user with API key: {}", api_key.key);
        
        users_lock.insert(demo.id.clone(), demo);
    }
}