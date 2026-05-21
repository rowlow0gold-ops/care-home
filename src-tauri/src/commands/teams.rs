use serde::Serialize;
use tauri::command;
use crate::db;

#[derive(Debug, Serialize)]
pub struct TeamDto {
    pub id:           i64,
    pub name:         String,
    pub color:        String,
    pub manager_id:   Option<i64>,
    pub manager_name: Option<String>,
    pub staff:        Vec<TeamMember>,
}

#[derive(Debug, Serialize)]
pub struct TeamMember {
    pub user_id:   i64,
    pub full_name: String,
    pub position:  Option<String>,
}

/// Return all teams with their manager and staff member list.
#[command]
pub fn list_teams() -> Result<Vec<TeamDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;

    let mut stmt = db.prepare(
        "SELECT t.id, t.name, t.color, t.manager_id, u.full_name
         FROM teams t
         LEFT JOIN users u ON u.id = t.manager_id
         ORDER BY t.id",
    ).map_err(|e| e.to_string())?;

    let mut teams: Vec<TeamDto> = stmt.query_map([], |row| {
        Ok(TeamDto {
            id:           row.get(0)?,
            name:         row.get(1)?,
            color:        row.get(2)?,
            manager_id:   row.get(3)?,
            manager_name: row.get(4)?,
            staff:        vec![],
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    // Populate staff members for each team
    for team in &mut teams {
        let mut ms = db.prepare(
            "SELECT u.id, u.full_name, st.position
             FROM staff st
             JOIN users u ON u.id = st.user_id
             WHERE st.team_id = ?1
             ORDER BY u.full_name",
        ).map_err(|e| e.to_string())?;

        team.staff = ms.query_map([team.id], |row| {
            Ok(TeamMember {
                user_id:   row.get(0)?,
                full_name: row.get(1)?,
                position:  row.get(2)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    }

    Ok(teams)
}

/// Return the team for a given user_id (None if they're not in any team).
#[command]
pub fn get_user_team(user_id: i64) -> Result<Option<TeamDto>, String> {
    let teams = list_teams()?;
    // Check if the user is a manager of a team
    if let Some(t) = teams.iter().find(|t| t.manager_id == Some(user_id)) {
        return Ok(Some(TeamDto {
            id:           t.id,
            name:         t.name.clone(),
            color:        t.color.clone(),
            manager_id:   t.manager_id,
            manager_name: t.manager_name.clone(),
            staff:        t.staff.iter().map(|s| TeamMember {
                user_id:   s.user_id,
                full_name: s.full_name.clone(),
                position:  s.position.clone(),
            }).collect(),
        }));
    }
    // Check if the user is a staff member of a team
    Ok(teams.into_iter().find(|t| t.staff.iter().any(|s| s.user_id == user_id)))
}
