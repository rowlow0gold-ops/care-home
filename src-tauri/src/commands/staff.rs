use serde::{Deserialize, Serialize};
use tauri::command;
use crate::db;
extern crate bcrypt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StaffDto {
    pub id: i64,
    pub user_id: Option<i64>,
    pub full_name: String,
    pub username: Option<String>,
    pub role: Option<String>,
    pub employee_id: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub hire_date: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub hourly_rate: Option<f64>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateStaffInput {
    pub full_name: String,
    pub username: String,
    pub password: String,
    pub role: String,
    pub employee_id: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub hire_date: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub hourly_rate: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStaffInput {
    pub full_name: String,
    pub department: Option<String>,
    pub position: Option<String>,
    pub hire_date: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub hourly_rate: Option<f64>,
}

#[command]
pub fn list_staff(active_only: Option<bool>) -> Result<Vec<StaffDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let active = active_only.unwrap_or(true);

    let mut stmt = db.prepare(
        "SELECT s.id, s.user_id, u.full_name, u.username, u.role,
                s.employee_id, s.department, s.position, s.hire_date,
                s.phone, s.email, s.hourly_rate, s.is_active
         FROM staff s
         JOIN users u ON u.id = s.user_id
         WHERE (s.is_active = ?1 OR ?1 = 0)
         ORDER BY u.full_name"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(
        [if active { 1i64 } else { 0i64 }],
        |row| Ok(StaffDto {
            id: row.get(0)?,
            user_id: row.get(1)?,
            full_name: row.get(2)?,
            username: row.get(3)?,
            role: row.get(4)?,
            employee_id: row.get(5)?,
            department: row.get(6)?,
            position: row.get(7)?,
            hire_date: row.get(8)?,
            phone: row.get(9)?,
            email: row.get(10)?,
            hourly_rate: row.get(11)?,
            is_active: row.get::<_, i64>(12)? != 0,
        }),
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(rows)
}

#[command]
pub fn create_staff_member(input: CreateStaffInput) -> Result<i64, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let hash = bcrypt::hash(&input.password, bcrypt::DEFAULT_COST)
        .map_err(|e| e.to_string())?;

    db.execute(
        "INSERT INTO users (username, password_hash, full_name, role) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![input.username, hash, input.full_name, input.role],
    ).map_err(|e| e.to_string())?;
    let user_id = db.last_insert_rowid();

    db.execute(
        "INSERT INTO staff (user_id, employee_id, department, position, hire_date, phone, email, hourly_rate)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            user_id, input.employee_id, input.department, input.position,
            input.hire_date, input.phone, input.email, input.hourly_rate,
        ],
    ).map_err(|e| e.to_string())?;

    Ok(db.last_insert_rowid())
}

#[command]
pub fn update_staff_member(id: i64, input: UpdateStaffInput, actor_id: i64, actor_role: String) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;

    // Look up target's user_id and role
    let (target_user_id, target_role): (i64, String) = db.query_row(
        "SELECT s.user_id, u.role FROM staff s JOIN users u ON u.id = s.user_id WHERE s.id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|e| e.to_string())?;

    if actor_role != "admin" {
        if target_user_id == actor_id {
            return Err("Permission denied: you cannot edit your own account.".into());
        }
        if target_role != "staff" {
            return Err("Permission denied: managers can only edit staff accounts.".into());
        }
    }

    // Update user full_name
    db.execute(
        "UPDATE users SET full_name = ?1 WHERE id = (SELECT user_id FROM staff WHERE id = ?2)",
        rusqlite::params![input.full_name, id],
    ).map_err(|e| e.to_string())?;

    db.execute(
        "UPDATE staff SET department=?1, position=?2,
         hire_date=?3, phone=?4, email=?5, hourly_rate=?6 WHERE id=?7",
        rusqlite::params![
            input.department, input.position,
            input.hire_date, input.phone, input.email, input.hourly_rate, id,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn deactivate_staff(id: i64, actor_id: i64, actor_role: String) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;

    // Check the target account's user_id and role
    let (target_user_id, target_role): (i64, String) = db.query_row(
        "SELECT s.user_id, u.role FROM staff s JOIN users u ON u.id = s.user_id WHERE s.id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|e| e.to_string())?;

    if actor_role != "admin" {
        if target_user_id == actor_id {
            return Err("Permission denied: you cannot deactivate yourself.".into());
        }
        if target_role != "staff" {
            return Err("Permission denied: managers can only deactivate staff accounts.".into());
        }
    }

    db.execute("UPDATE staff SET is_active=0 WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn set_staff_role(id: i64, new_role: String, actor_id: i64, actor_role: String) -> Result<(), String> {
    // Only admin can change roles
    if actor_role != "admin" {
        return Err("Permission denied: only an admin can change roles.".into());
    }
    // Only staff <-> manager transitions are allowed
    if new_role != "staff" && new_role != "manager" {
        return Err("Invalid role: only 'staff' and 'manager' are allowed.".into());
    }

    let db = db::get().lock().map_err(|e| e.to_string())?;

    let (target_user_id, target_role): (i64, String) = db.query_row(
        "SELECT s.user_id, u.role FROM staff s JOIN users u ON u.id = s.user_id WHERE s.id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|e| e.to_string())?;

    // Can't change your own role
    if target_user_id == actor_id {
        return Err("Permission denied: you cannot change your own role.".into());
    }
    // Can't touch another admin
    if target_role == "admin" {
        return Err("Permission denied: cannot change an admin's role.".into());
    }

    db.execute(
        "UPDATE users SET role = ?1 WHERE id = ?2",
        rusqlite::params![new_role, target_user_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
