use serde::{Deserialize, Serialize};
use tauri::command;
use crate::db;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MealPlanDto {
    pub id: i64,
    pub week_start: String,
    pub day_of_week: i64,
    pub meal_type: String,
    pub menu: String,
    pub calories: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertMealPlanInput {
    pub week_start: String,
    pub day_of_week: i64,
    pub meal_type: String,
    pub menu: String,
    pub calories: Option<i64>,
    pub notes: Option<String>,
}

#[command]
pub fn list_meal_plans(week_start: String) -> Result<Vec<MealPlanDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT id, week_start, day_of_week, meal_type, menu, calories, notes
         FROM meal_plans WHERE week_start = ?1
         ORDER BY day_of_week, meal_type"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([&week_start], |row| {
        Ok(MealPlanDto {
            id: row.get(0)?,
            week_start: row.get(1)?,
            day_of_week: row.get(2)?,
            meal_type: row.get(3)?,
            menu: row.get(4)?,
            calories: row.get(5)?,
            notes: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(rows)
}

/// Load all meal plans whose week_start falls between start_date and end_date (inclusive).
/// Used for monthly view and statistics.
#[command]
pub fn list_meal_plans_range(start_date: String, end_date: String) -> Result<Vec<MealPlanDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT id, week_start, day_of_week, meal_type, menu, calories, notes
         FROM meal_plans
         WHERE week_start >= ?1 AND week_start <= ?2
         ORDER BY week_start, day_of_week, meal_type"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([&start_date, &end_date], |row| {
        Ok(MealPlanDto {
            id: row.get(0)?,
            week_start: row.get(1)?,
            day_of_week: row.get(2)?,
            meal_type: row.get(3)?,
            menu: row.get(4)?,
            calories: row.get(5)?,
            notes: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(rows)
}

#[command]
pub fn upsert_meal_plan(input: UpsertMealPlanInput) -> Result<(), String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO meal_plans (week_start, day_of_week, meal_type, menu, calories, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(week_start, day_of_week, meal_type)
         DO UPDATE SET menu=excluded.menu, calories=excluded.calories, notes=excluded.notes",
        rusqlite::params![
            input.week_start, input.day_of_week, input.meal_type,
            input.menu, input.calories, input.notes,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Bulk upsert used by Excel import — same logic, multiple rows at once.
#[command]
pub fn bulk_upsert_meal_plans(plans: Vec<UpsertMealPlanInput>) -> Result<i64, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let mut count = 0i64;
    for input in plans {
        db.execute(
            "INSERT INTO meal_plans (week_start, day_of_week, meal_type, menu, calories, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(week_start, day_of_week, meal_type)
             DO UPDATE SET menu=excluded.menu, calories=excluded.calories, notes=excluded.notes",
            rusqlite::params![
                input.week_start, input.day_of_week, input.meal_type,
                input.menu, input.calories, input.notes,
            ],
        ).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}
