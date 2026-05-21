use serde::{Deserialize, Serialize};
use tauri::command;
use crate::db;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VitalDto {
    pub id: i64,
    pub resident_id: i64,
    pub resident_name: String,
    pub staff_name: Option<String>,
    pub bp_systolic: Option<i64>,
    pub bp_diastolic: Option<i64>,
    pub heart_rate: Option<i64>,
    pub temperature: Option<f64>,
    pub weight: Option<f64>,
    pub blood_sugar: Option<i64>,
    pub spo2: Option<i64>,
    pub notes: Option<String>,
    pub measured_at: String,
}

#[derive(Debug, Serialize)]
pub struct PagedVitals {
    pub data: Vec<VitalDto>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateVitalInput {
    pub resident_id: i64,
    pub staff_id: Option<i64>,
    pub bp_systolic: Option<i64>,
    pub bp_diastolic: Option<i64>,
    pub heart_rate: Option<i64>,
    pub temperature: Option<f64>,
    pub weight: Option<f64>,
    pub blood_sugar: Option<i64>,
    pub spo2: Option<i64>,
    pub notes: Option<String>,
}

#[command]
pub fn list_vitals(
    resident_id:   Option<i64>,
    show_archived: Option<bool>,
    date_from:     Option<String>,
    date_to:       Option<String>,
    page:          Option<i64>,
    page_size:     Option<i64>,
    sort_by:       Option<String>,
    sort_desc:     Option<bool>,
) -> Result<PagedVitals, String> {
    let db        = db::get().lock().map_err(|e| e.to_string())?;
    let archived  = show_archived.unwrap_or(false);
    let page      = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(25).max(1);
    let offset    = (page - 1) * page_size;
    let dir       = if sort_desc.unwrap_or(true) { "DESC" } else { "ASC" };
    let order_col = match sort_by.as_deref() {
        Some("measured_at")   => format!("v.measured_at {dir}"),
        Some("resident_name") => format!("r.last_name {dir}, r.first_name {dir}"),
        _                     => "v.measured_at DESC".to_string(),
    };
    let arch_flag = if archived { 1i64 } else { 0i64 };

    let total: i64 = db.query_row(
        "SELECT COUNT(*) FROM vitals v
         WHERE (?1 IS NULL OR v.resident_id = ?1)
           AND COALESCE(v.is_archived, 0) = ?2
           AND (?3 IS NULL OR date(v.measured_at) >= ?3)
           AND (?4 IS NULL OR date(v.measured_at) <= ?4)",
        rusqlite::params![resident_id, arch_flag, date_from, date_to],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let sql = format!(
        "SELECT v.id, v.resident_id,
                (r.first_name || ' ' || r.last_name) AS resident_name,
                u.full_name AS staff_name,
                v.bp_systolic, v.bp_diastolic, v.heart_rate,
                v.temperature, v.weight, v.blood_sugar, v.spo2,
                v.notes, v.measured_at
         FROM vitals v
         JOIN residents r ON r.id = v.resident_id
         LEFT JOIN users u ON u.id = v.staff_id
         WHERE (?1 IS NULL OR v.resident_id = ?1)
           AND COALESCE(v.is_archived, 0) = ?2
           AND (?3 IS NULL OR date(v.measured_at) >= ?3)
           AND (?4 IS NULL OR date(v.measured_at) <= ?4)
         ORDER BY {order_col}
         LIMIT ?5 OFFSET ?6"
    );
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;

    let data = stmt.query_map(
        rusqlite::params![resident_id, arch_flag, date_from, date_to, page_size, offset],
        |row| Ok(VitalDto {
            id:            row.get(0)?,
            resident_id:   row.get(1)?,
            resident_name: row.get(2)?,
            staff_name:    row.get(3)?,
            bp_systolic:   row.get(4)?,
            bp_diastolic:  row.get(5)?,
            heart_rate:    row.get(6)?,
            temperature:   row.get(7)?,
            weight:        row.get(8)?,
            blood_sugar:   row.get(9)?,
            spo2:          row.get(10)?,
            notes:         row.get(11)?,
            measured_at:   row.get(12)?,
        }),
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(PagedVitals { data, total })
}

/// Soft-delete a vital record — archives it into history instead of erasing.
#[command]
pub fn archive_vital(id: i64) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE vitals SET is_archived = 1, archived_at = datetime('now') WHERE id = ?1",
        rusqlite::params![id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Permanently delete a vital record (manager/admin only).
#[command]
pub fn delete_vital(id: i64, actor_role: String) -> Result<(), String> {
    if actor_role != "manager" && actor_role != "admin" {
        return Err("Permission denied: only managers and admins can delete vital records.".to_string());
    }
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM vitals WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn create_vital(input: CreateVitalInput) -> Result<i64, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO vitals (resident_id, staff_id, bp_systolic, bp_diastolic,
         heart_rate, temperature, weight, blood_sugar, spo2, notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![
            input.resident_id, input.staff_id,
            input.bp_systolic, input.bp_diastolic, input.heart_rate,
            input.temperature, input.weight, input.blood_sugar,
            input.spo2, input.notes,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}
