use serde::{Deserialize, Serialize};
use tauri::command;
use crate::db;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserDto {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserInput {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub role: String,
}

fn role_level(role: &str) -> i32 {
    match role {
        "staff"      => 1,
        "manager"     => 2,
        "admin"       => 3,
        
        _             => 0,
    }
}

#[command]
pub fn login(username: String, password: String) -> Result<UserDto, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;

    let result = db.query_row(
        "SELECT id, username, password_hash, full_name, role, is_active FROM users WHERE username = ?1",
        [&username],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        },
    );

    match result {
        Ok((id, uname, hash, full_name, role, is_active)) => {
            if is_active == 0 {
                return Err("Account is disabled".into());
            }
            let valid = bcrypt::verify(&password, &hash)
                .map_err(|e| e.to_string())?;
            if !valid {
                return Err("Invalid username or password".into());
            }
            Ok(UserDto { id, username: uname, full_name, role })
        }
        Err(_) => Err("Invalid username or password".into()),
    }
}

#[command]
pub fn create_user(
    input: CreateUserInput,
    actor_role: String,
) -> Result<i64, String> {
    // Actor must have strictly higher level than the role being created
    if role_level(&actor_role) <= role_level(&input.role) {
        return Err("You do not have permission to create this role.".into());
    }
    let valid_roles = ["staff", "manager", "admin"];
    if !valid_roles.contains(&input.role.as_str()) {
        return Err("Invalid role.".into());
    }
    let hash = bcrypt::hash(&input.password, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())?;
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO users (username, password_hash, full_name, role) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![input.username, hash, input.full_name, input.role],
    ).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}

#[command]
pub fn delete_user(
    user_id: i64,
    actor_id: i64,
    actor_role: String,
) -> Result<(), String> {
    if user_id == actor_id {
        return Err("You cannot delete your own account.".into());
    }
    let db = db::get().lock().map_err(|e| e.to_string())?;

    // Fetch the target user's role
    let target_role: String = db.query_row(
        "SELECT role FROM users WHERE id = ?1",
        [user_id],
        |row| row.get(0),
    ).map_err(|_| "User not found.".to_string())?;

    // Actor must have strictly higher level than the target
    if role_level(&actor_role) <= role_level(&target_role) {
        return Err("You do not have permission to delete this account.".into());
    }

    // Soft-delete: set is_active = 0
    db.execute(
        "UPDATE users SET is_active = 0 WHERE id = ?1",
        [user_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn list_users() -> Result<Vec<UserDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT id, username, full_name, role FROM users WHERE is_active = 1 ORDER BY full_name"
    ).map_err(|e| e.to_string())?;
    let users = stmt.query_map([], |row| {
        Ok(UserDto {
            id: row.get(0)?,
            username: row.get(1)?,
            full_name: row.get(2)?,
            role: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    Ok(users)
}
