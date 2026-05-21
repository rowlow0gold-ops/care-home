use serde::{Deserialize, Serialize};
use tauri::command;
use crate::db;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CareLogDto {
    pub id: i64,
    pub resident_id: i64,
    pub resident_name: String,
    pub staff_id: Option<i64>,
    pub staff_name: Option<String>,
    pub shift: String,
    pub category: String,
    pub content: String,
    pub is_incident: bool,
    pub is_flagged: bool,
    pub logged_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCareLogInput {
    pub resident_id: i64,
    pub shift: String,
    pub category: String,
    pub content: String,
    pub is_incident: Option<bool>,
    pub staff_id: Option<i64>,
    pub logged_at: Option<String>,  // "YYYY-MM-DD HH:MM"; defaults to now
}

#[derive(Debug, Deserialize)]
pub struct UpdateCareLogInput {
    pub content: String,
    pub is_incident: Option<bool>,
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<CareLogDto> {
    Ok(CareLogDto {
        id:            row.get(0)?,
        resident_id:   row.get(1)?,
        resident_name: row.get(2)?,
        staff_id:      row.get(3)?,
        staff_name:    row.get(4)?,
        shift:         row.get(5)?,
        category:      row.get(6)?,
        content:       row.get(7)?,
        is_incident:   row.get::<_, i64>(8)? != 0,
        is_flagged:    row.get::<_, i64>(9)? != 0,
        logged_at:     row.get(10)?,
    })
}

/// Daily view: fetch logs for a specific date, optionally filtered by resident
#[command]
pub fn list_care_logs(
    resident_id: Option<i64>,
    date:        Option<String>,
    category:    Option<String>,
    shift:       Option<String>,
    limit:       Option<i64>,
) -> Result<Vec<CareLogDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(-1);
    let mut stmt = db.prepare(
        "SELECT cl.id, cl.resident_id,
                (r.first_name || ' ' || r.last_name) AS resident_name,
                cl.staff_id, u.full_name AS staff_name,
                cl.shift, cl.category, cl.content, cl.is_incident, cl.is_flagged, cl.logged_at
         FROM care_logs cl
         JOIN residents r ON r.id = cl.resident_id
         LEFT JOIN users u ON u.id = cl.staff_id
         WHERE (?1 IS NULL OR cl.resident_id = ?1)
           AND (?2 IS NULL OR date(cl.logged_at) = ?2)
           AND (?3 IS NULL OR cl.category = ?3)
           AND (?4 IS NULL OR cl.shift    = ?4)
         ORDER BY cl.logged_at DESC
         LIMIT ?5"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(
        rusqlite::params![resident_id, date, category, shift, lim],
        map_row,
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    Ok(rows)
}

/// History view: flexible date-range query with optional filters + pagination
#[derive(Debug, Serialize)]
pub struct PagedCareLogs {
    pub data:  Vec<CareLogDto>,
    pub total: i64,
}

#[command]
pub fn list_care_logs_history(
    resident_id:   Option<i64>,
    date_from:     Option<String>,
    date_to:       Option<String>,
    category:      Option<String>,
    shift:         Option<String>,
    incident_only: Option<bool>,
    page:          Option<i64>,
    page_size:     Option<i64>,
    sort_by:       Option<String>,
    sort_desc:     Option<bool>,
) -> Result<PagedCareLogs, String> {
    let db        = db::get().lock().map_err(|e| e.to_string())?;
    let page      = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(25).max(1);
    let offset    = (page - 1) * page_size;
    let incident_filter = incident_only.unwrap_or(false);
    let dir       = if sort_desc.unwrap_or(true) { "DESC" } else { "ASC" };
    let order_col = match sort_by.as_deref() {
        Some("logged_at")     => format!("cl.logged_at {dir}"),
        Some("resident_name") => format!("r.last_name {dir}, r.first_name {dir}"),
        Some("category")      => format!("cl.category {dir}"),
        Some("shift")         => format!("cl.shift {dir}"),
        Some("staff_name")    => format!("u.full_name {dir}"),
        _                     => "cl.logged_at DESC".to_string(),
    };

    let total: i64 = db.query_row(
        "SELECT COUNT(*) FROM care_logs cl
         JOIN residents r ON r.id = cl.resident_id
         WHERE (?1 IS NULL OR cl.resident_id = ?1)
           AND (?2 IS NULL OR date(cl.logged_at) >= ?2)
           AND (?3 IS NULL OR date(cl.logged_at) <= ?3)
           AND (?4 IS NULL OR cl.category = ?4)
           AND (?5 IS NULL OR cl.shift = ?5)
           AND (?6 = 0 OR cl.is_incident = 1)",
        rusqlite::params![resident_id, date_from, date_to, category, shift, incident_filter as i64],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let sql = format!(
        "SELECT cl.id, cl.resident_id,
                (r.first_name || ' ' || r.last_name) AS resident_name,
                cl.staff_id, u.full_name AS staff_name,
                cl.shift, cl.category, cl.content, cl.is_incident, cl.is_flagged, cl.logged_at
         FROM care_logs cl
         JOIN residents r ON r.id = cl.resident_id
         LEFT JOIN users u ON u.id = cl.staff_id
         WHERE (?1 IS NULL OR cl.resident_id = ?1)
           AND (?2 IS NULL OR date(cl.logged_at) >= ?2)
           AND (?3 IS NULL OR date(cl.logged_at) <= ?3)
           AND (?4 IS NULL OR cl.category = ?4)
           AND (?5 IS NULL OR cl.shift = ?5)
           AND (?6 = 0 OR cl.is_incident = 1)
         ORDER BY {order_col}
         LIMIT ?7 OFFSET ?8"
    );
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let data = stmt.query_map(
        rusqlite::params![
            resident_id, date_from, date_to,
            category, shift, incident_filter as i64,
            page_size, offset
        ],
        map_row,
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    Ok(PagedCareLogs { data, total })
}

#[command]
pub fn create_care_log(input: CreateCareLogInput) -> Result<i64, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    if let Some(ref ts) = input.logged_at {
        db.execute(
            "INSERT INTO care_logs (resident_id, staff_id, shift, category, content, is_incident, logged_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                input.resident_id, input.staff_id,
                input.shift, input.category, input.content,
                input.is_incident.unwrap_or(false) as i64,
                ts,
            ],
        ).map_err(|e| e.to_string())?;
    } else {
        db.execute(
            "INSERT INTO care_logs (resident_id, staff_id, shift, category, content, is_incident)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                input.resident_id, input.staff_id,
                input.shift, input.category, input.content,
                input.is_incident.unwrap_or(false) as i64,
            ],
        ).map_err(|e| e.to_string())?;
    }
    Ok(db.last_insert_rowid())
}

/// Update content/incident flag — staff may only edit their own entries
#[command]
pub fn update_care_log(
    id: i64,
    input: UpdateCareLogInput,
    actor_id: i64,
    actor_role: String,
) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;

    // Staff can only edit their own entries
    if actor_role == "staff" {
        let owner: Option<i64> = db.query_row(
            "SELECT staff_id FROM care_logs WHERE id = ?1",
            [id],
            |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        if owner != Some(actor_id) {
            return Err("Permission denied: you can only edit your own entries.".into());
        }
    }

    db.execute(
        "UPDATE care_logs SET content = ?1, is_incident = ?2 WHERE id = ?3",
        rusqlite::params![
            input.content,
            input.is_incident.unwrap_or(false) as i64,
            id,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Toggle the flagged state — staff may only flag their own entries; manager+ can flag any
#[command]
pub fn flag_care_log(
    id: i64,
    flagged: bool,
    actor_id: i64,
    actor_role: String,
) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;

    if actor_role == "staff" {
        let owner: Option<i64> = db.query_row(
            "SELECT staff_id FROM care_logs WHERE id = ?1",
            [id],
            |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        if owner != Some(actor_id) {
            return Err("Permission denied: you can only flag your own entries.".into());
        }
    }

    db.execute(
        "UPDATE care_logs SET is_flagged = ?1 WHERE id = ?2",
        rusqlite::params![flagged as i64, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn delete_care_log(id: i64) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM care_logs WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
