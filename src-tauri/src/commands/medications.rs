use serde::{Deserialize, Serialize};
use tauri::command;
use crate::db;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MedicationDto {
    pub id: i64,
    pub resident_id: i64,
    pub resident_name: String,
    pub name: String,
    pub dosage: String,
    pub frequency: String,
    pub route: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub prescriber: Option<String>,
    pub instructions: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateMedicationInput {
    pub resident_id: i64,
    pub name: String,
    pub dosage: String,
    pub frequency: String,
    pub route: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub prescriber: Option<String>,
    pub instructions: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MedAdminDto {
    pub id: i64,
    pub medication_id: i64,
    pub medication_name: String,
    pub resident_id: i64,
    pub resident_name: String,
    pub staff_name: Option<String>,
    pub status: String,
    pub administered_at: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecordAdminInput {
    pub medication_id: i64,
    pub resident_id: i64,
    pub staff_id: Option<i64>,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PagedMedications {
    pub data: Vec<MedicationDto>,
    pub total: i64,
}

#[command]
pub fn list_medications(
    resident_id: Option<i64>,
    active_only: Option<bool>,
    route:       Option<String>,
    page:        Option<i64>,
    page_size:   Option<i64>,
    sort_by:     Option<String>,
    sort_desc:   Option<bool>,
) -> Result<PagedMedications, String> {
    let db        = db::get().lock().map_err(|e| e.to_string())?;
    let page      = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(25).max(1);
    let offset    = (page - 1) * page_size;
    let dir       = if sort_desc.unwrap_or(false) { "DESC" } else { "ASC" };
    let order_col = match sort_by.as_deref() {
        Some("name")          => format!("m.name {dir}"),
        Some("start_date")    => format!("m.start_date {dir}"),
        Some("resident_name") => format!("r.last_name {dir}, r.first_name {dir}"),
        _                     => "r.last_name ASC, r.first_name ASC, m.name ASC".to_string(),
    };
    let active_filter: Option<i64> = match active_only {
        Some(true)  => Some(1),
        Some(false) => Some(0),
        None        => None,
    };
    let hide_expired = active_filter == Some(1);

    let total: i64 = db.query_row(
        "SELECT COUNT(*) FROM medications m
         WHERE (?1 IS NULL OR m.resident_id = ?1)
           AND (?2 IS NULL OR m.is_active = ?2)
           AND (NOT ?3 OR m.end_date IS NULL OR m.end_date >= date('now'))
           AND (?4 IS NULL OR lower(m.route) = lower(?4))",
        rusqlite::params![resident_id, active_filter, hide_expired, route],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let sql = format!(
        "SELECT m.id, m.resident_id,
                (r.first_name || ' ' || r.last_name) AS resident_name,
                m.name, m.dosage, m.frequency, m.route,
                m.start_date, m.end_date, m.prescriber, m.instructions, m.is_active
         FROM medications m
         JOIN residents r ON r.id = m.resident_id
         WHERE (?1 IS NULL OR m.resident_id = ?1)
           AND (?2 IS NULL OR m.is_active = ?2)
           AND (NOT ?5 OR m.end_date IS NULL OR m.end_date >= date('now'))
           AND (?6 IS NULL OR lower(m.route) = lower(?6))
         ORDER BY {order_col}
         LIMIT ?3 OFFSET ?4"
    );
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;

    let data = stmt.query_map(
        rusqlite::params![resident_id, active_filter, page_size, offset, hide_expired, route],
        |row| Ok(MedicationDto {
            id:            row.get(0)?,
            resident_id:   row.get(1)?,
            resident_name: row.get(2)?,
            name:          row.get(3)?,
            dosage:        row.get(4)?,
            frequency:     row.get(5)?,
            route:         row.get(6)?,
            start_date:    row.get(7)?,
            end_date:      row.get(8)?,
            prescriber:    row.get(9)?,
            instructions:  row.get(10)?,
            is_active:     row.get::<_, i64>(11)? != 0,
        }),
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(PagedMedications { data, total })
}

#[command]
pub fn create_medication(input: CreateMedicationInput) -> Result<i64, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO medications (resident_id, name, dosage, frequency, route, start_date, end_date, prescriber, instructions)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            input.resident_id, input.name, input.dosage, input.frequency,
            input.route.unwrap_or_else(|| "oral".to_string()),
            input.start_date, input.end_date, input.prescriber, input.instructions,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}

#[command]
pub fn update_medication(id: i64, input: CreateMedicationInput) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE medications SET name=?1, dosage=?2, frequency=?3, route=?4,
         start_date=?5, end_date=?6, prescriber=?7, instructions=?8 WHERE id=?9",
        rusqlite::params![
            input.name, input.dosage, input.frequency,
            input.route.unwrap_or_else(|| "oral".to_string()),
            input.start_date, input.end_date, input.prescriber, input.instructions, id,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn stop_medication(id: i64) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE medications SET is_active=0, end_date=date('now') WHERE id=?1",
        [id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn list_med_administrations(
    resident_id: Option<i64>,
    date: Option<String>,
) -> Result<Vec<MedAdminDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT ma.id, ma.medication_id, m.name,
                ma.resident_id, (r.first_name || ' ' || r.last_name),
                u.full_name, ma.status, ma.administered_at, ma.notes
         FROM medication_administrations ma
         JOIN medications m ON m.id = ma.medication_id
         JOIN residents r ON r.id = ma.resident_id
         LEFT JOIN users u ON u.id = ma.staff_id
         WHERE (?1 IS NULL OR ma.resident_id = ?1)
           AND (?2 IS NULL OR date(ma.administered_at) = ?2)
         ORDER BY ma.administered_at DESC
         LIMIT 200"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(
        rusqlite::params![resident_id, date],
        |row| Ok(MedAdminDto {
            id: row.get(0)?,
            medication_id: row.get(1)?,
            medication_name: row.get(2)?,
            resident_id: row.get(3)?,
            resident_name: row.get(4)?,
            staff_name: row.get(5)?,
            status: row.get(6)?,
            administered_at: row.get(7)?,
            notes: row.get(8)?,
        }),
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(rows)
}

#[command]
pub fn record_med_administration(input: RecordAdminInput) -> Result<i64, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO medication_administrations
         (medication_id, resident_id, staff_id, scheduled_at, administered_at, status, notes)
         VALUES (?1, ?2, ?3, datetime('now'), datetime('now'), ?4, ?5)",
        rusqlite::params![
            input.medication_id, input.resident_id, input.staff_id,
            input.status, input.notes,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}
