use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::db;

#[derive(Serialize)]
pub struct ScheduleEntry {
    pub id:          i64,
    pub staff_id:    i64,
    pub staff_name:  String,
    pub shift_date:  String,
    pub shift_start: String,
    pub shift_end:   String,
    pub shift_hours: f64,
    pub notes:       Option<String>,
}

#[derive(Deserialize)]
pub struct CreateScheduleInput {
    pub staff_id:    i64,
    pub shift_date:  String,
    pub shift_start: String,
    pub shift_end:   String,
    pub shift_hours: f64,
    pub notes:       Option<String>,
}

/// List all schedule entries for a date range (week_start..=week_end).
/// If staff_id is Some, filter to that staff member only.
#[tauri::command]
pub async fn list_schedules(
    staff_id:   Option<i64>,
    team_id:    Option<i64>,
    week_start: String,
    week_end:   String,
) -> Result<Vec<ScheduleEntry>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT s.id, s.staff_id, u.full_name,
                s.shift_date, s.shift_start, s.shift_end, s.shift_hours, s.notes
         FROM   schedules s
         JOIN   users    u  ON u.id      = s.staff_id
         LEFT JOIN staff st ON st.user_id = s.staff_id
         WHERE  s.shift_date >= ?1
           AND  s.shift_date <= ?2
           AND  (?3 IS NULL OR s.staff_id = ?3)
           AND  (?4 IS NULL OR st.team_id  = ?4)
         ORDER  BY s.shift_date, s.shift_start, u.full_name",
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![week_start, week_end, staff_id, team_id], |row| {
        Ok(ScheduleEntry {
            id:          row.get(0)?,
            staff_id:    row.get(1)?,
            staff_name:  row.get(2)?,
            shift_date:  row.get(3)?,
            shift_start: row.get(4)?,
            shift_end:   row.get(5)?,
            shift_hours: row.get(6)?,
            notes:       row.get(7)?,
        })
    }).map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Create a new schedule entry (manager/admin only).
#[tauri::command]
pub async fn create_schedule(
    input:      CreateScheduleInput,
    actor_role: String,
    actor_id:   i64,
) -> Result<i64, String> {
    if actor_role != "manager" && actor_role != "admin" {
        return Err("Permission denied: only managers and admins can create schedules.".to_string());
    }
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO schedules
         (staff_id, shift_date, shift_start, shift_end, shift_hours, notes, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            input.staff_id, input.shift_date, input.shift_start,
            input.shift_end, input.shift_hours, input.notes, actor_id
        ],
    ).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}

#[derive(Deserialize)]
pub struct UpdateScheduleInput {
    pub staff_id:    i64,
    pub shift_date:  String,
    pub shift_start: String,
    pub shift_end:   String,
    pub shift_hours: f64,
    pub notes:       Option<String>,
}

/// Update an existing schedule entry (manager/admin only).
#[tauri::command]
pub async fn update_schedule(
    id:         i64,
    input:      UpdateScheduleInput,
    actor_role: String,
) -> Result<(), String> {
    if actor_role != "manager" && actor_role != "admin" {
        return Err("Permission denied: only managers and admins can update schedules.".to_string());
    }
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE schedules
         SET staff_id = ?1, shift_date = ?2, shift_start = ?3,
             shift_end = ?4, shift_hours = ?5, notes = ?6
         WHERE id = ?7",
        params![
            input.staff_id, input.shift_date, input.shift_start,
            input.shift_end, input.shift_hours, input.notes, id
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a schedule entry (manager/admin only).
#[tauri::command]
pub async fn delete_schedule(id: i64, actor_role: String) -> Result<(), String> {
    if actor_role != "manager" && actor_role != "admin" {
        return Err("Permission denied: only managers and admins can delete schedules.".to_string());
    }
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM schedules WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
