use serde::{Deserialize, Serialize};
use tauri::command;
use crate::db;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResidentDto {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: String,
    pub room_number: Option<String>,
    pub admission_date: String,
    pub discharge_date: Option<String>,
    pub care_grade: Option<i64>,
    pub cognitive_support: bool,
    pub primary_diagnosis: Option<String>,
    pub allergies: Option<String>,
    pub dietary_restrictions: Option<String>,
    pub insurance_number: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub is_deceased: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateResidentInput {
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: String,
    pub room_number: Option<String>,
    pub admission_date: Option<String>,
    pub care_grade: Option<i64>,
    pub cognitive_support: Option<bool>,
    pub primary_diagnosis: Option<String>,
    pub allergies: Option<String>,
    pub dietary_restrictions: Option<String>,
    pub insurance_number: Option<String>,
    pub notes: Option<String>,
}

const SELECT_COLS: &str =
    "SELECT id, first_name, last_name, date_of_birth, gender, room_number,
            admission_date, discharge_date, care_grade, cognitive_support,
            primary_diagnosis, allergies, dietary_restrictions, insurance_number,
            notes, is_active, COALESCE(is_deceased, 0) AS is_deceased
     FROM residents";

fn map_resident(row: &rusqlite::Row) -> rusqlite::Result<ResidentDto> {
    Ok(ResidentDto {
        id:                   row.get(0)?,
        first_name:           row.get(1)?,
        last_name:            row.get(2)?,
        date_of_birth:        row.get(3)?,
        gender:               row.get(4)?,
        room_number:          row.get(5)?,
        admission_date:       row.get(6)?,
        discharge_date:       row.get(7)?,
        care_grade:           row.get(8)?,
        cognitive_support:    row.get::<_, i64>(9)? != 0,
        primary_diagnosis:    row.get(10)?,
        allergies:            row.get(11)?,
        dietary_restrictions: row.get(12)?,
        insurance_number:     row.get(13)?,
        notes:                row.get(14)?,
        is_active:            row.get::<_, i64>(15)? != 0,
        is_deceased:          row.get::<_, i64>(16)? != 0,
    })
}

/// status: "active" | "discharged" | "deceased" | null (= all)
#[command]
pub fn list_residents(
    search: Option<String>,
    active_only: Option<bool>,
    status: Option<String>,
) -> Result<Vec<ResidentDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let like = format!("%{}%", search.unwrap_or_default());

    // Build status filter from either old active_only flag or new status string
    let eff_status = status.unwrap_or_else(|| {
        if active_only.unwrap_or(true) { "active".to_string() }
        else { "all".to_string() }
    });

    let where_clause = match eff_status.as_str() {
        "active"     => "is_active = 1 AND COALESCE(is_deceased, 0) = 0",
        "discharged" => "is_active = 0 AND COALESCE(is_deceased, 0) = 0",
        "deceased"   => "COALESCE(is_deceased, 0) = 1",
        _            => "1=1", // all
    };

    let sql = format!(
        "{} WHERE {}
           AND (first_name LIKE ?1 OR last_name LIKE ?1
                OR room_number LIKE ?1 OR insurance_number LIKE ?1)
         ORDER BY last_name, first_name",
        SELECT_COLS, where_clause
    );

    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let residents = stmt.query_map([&like], map_resident)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(residents)
}

// ── Paginated residents list with filters ─────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct PagedResidents {
    pub data:  Vec<ResidentDto>,
    pub total: i64,
}

#[command]
pub fn list_residents_paged(
    search:            Option<String>,
    status:            Option<String>,
    resident_id:       Option<i64>,
    care_grade:        Option<i64>,
    gender:            Option<String>,
    cognitive_support: Option<bool>,
    page:              Option<i64>,
    page_size:         Option<i64>,
    sort_by:           Option<String>,
    sort_desc:         Option<bool>,
) -> Result<PagedResidents, String> {
    let db        = db::get().lock().map_err(|e| e.to_string())?;
    let page      = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(25).max(1);
    let offset    = (page - 1) * page_size;
    let like      = format!("%{}%", search.unwrap_or_default());
    let dir       = if sort_desc.unwrap_or(false) { "DESC" } else { "ASC" };

    let order_col = match sort_by.as_deref() {
        Some("last_name")         => format!("last_name {dir}, first_name {dir}"),
        Some("first_name")        => format!("first_name {dir}"),
        Some("room_number")       => format!("CAST(room_number AS INTEGER) {dir}, room_number {dir}"),
        Some("care_grade")        => format!("care_grade {dir}"),
        Some("admission_date")    => format!("admission_date {dir}"),
        Some("date_of_birth")     => format!("date_of_birth {dir}"),
        Some("primary_diagnosis") => format!("primary_diagnosis {dir}"),
        _                         => "id DESC".to_string(),
    };

    let eff_status = status.unwrap_or_else(|| "active".to_string());
    let status_clause = match eff_status.as_str() {
        "active"     => "is_active = 1 AND COALESCE(is_deceased, 0) = 0",
        "discharged" => "is_active = 0 AND COALESCE(is_deceased, 0) = 0",
        "deceased"   => "COALESCE(is_deceased, 0) = 1",
        _            => "1=1",
    };

    let cog = cognitive_support.map(|b| b as i64);

    let count_sql = format!(
        "SELECT COUNT(*) FROM residents
         WHERE {status_clause}
           AND (?1 IS NULL OR id = ?1)
           AND (first_name LIKE ?2 OR last_name LIKE ?2
                OR room_number LIKE ?2 OR insurance_number LIKE ?2)
           AND (?3 IS NULL OR care_grade = ?3)
           AND (?4 IS NULL OR lower(gender) = lower(?4))
           AND (?5 IS NULL OR cognitive_support = ?5)"
    );
    let total: i64 = db.query_row(
        &count_sql,
        rusqlite::params![resident_id, like, care_grade, gender, cog],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let sql = format!(
        "{} WHERE {status_clause}
           AND (?1 IS NULL OR id = ?1)
           AND (first_name LIKE ?2 OR last_name LIKE ?2
                OR room_number LIKE ?2 OR insurance_number LIKE ?2)
           AND (?3 IS NULL OR care_grade = ?3)
           AND (?4 IS NULL OR lower(gender) = lower(?4))
           AND (?5 IS NULL OR cognitive_support = ?5)
         ORDER BY {order_col}
         LIMIT ?6 OFFSET ?7",
        SELECT_COLS
    );
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let data = stmt.query_map(
        rusqlite::params![resident_id, like, care_grade, gender, cog, page_size, offset],
        map_resident,
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(PagedResidents { data, total })
}

#[command]
pub fn get_resident(id: i64) -> Result<ResidentDto, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let sql = format!("{} WHERE id = ?1", SELECT_COLS);
    db.query_row(&sql, [id], map_resident).map_err(|e| e.to_string())
}

#[command]
pub fn create_resident(input: CreateResidentInput) -> Result<i64, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO residents (first_name, last_name, date_of_birth, gender, room_number,
            admission_date, care_grade, cognitive_support, primary_diagnosis,
            allergies, dietary_restrictions, insurance_number, notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        rusqlite::params![
            input.first_name, input.last_name, input.date_of_birth, input.gender,
            input.room_number,
            input.admission_date.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string()),
            input.care_grade,
            input.cognitive_support.unwrap_or(false) as i64,
            input.primary_diagnosis, input.allergies,
            input.dietary_restrictions, input.insurance_number, input.notes
        ],
    ).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}

#[command]
pub fn update_resident(id: i64, input: CreateResidentInput) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE residents SET
            first_name=?1, last_name=?2, date_of_birth=?3, gender=?4,
            room_number=?5, care_grade=?6, cognitive_support=?7,
            primary_diagnosis=?8, allergies=?9, dietary_restrictions=?10,
            insurance_number=?11, notes=?12, updated_at=datetime('now')
         WHERE id=?13",
        rusqlite::params![
            input.first_name, input.last_name, input.date_of_birth, input.gender,
            input.room_number, input.care_grade,
            input.cognitive_support.unwrap_or(false) as i64,
            input.primary_diagnosis, input.allergies,
            input.dietary_restrictions, input.insurance_number, input.notes, id
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn discharge_resident(id: i64, discharge_date: String) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE residents SET is_active=0, discharge_date=?1, updated_at=datetime('now') WHERE id=?2",
        rusqlite::params![discharge_date, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn mark_deceased(id: i64) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.execute(
        "UPDATE residents SET is_active=0, is_deceased=1, discharge_date=?1, updated_at=datetime('now') WHERE id=?2",
        rusqlite::params![today, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
