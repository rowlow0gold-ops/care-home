mod commands;
mod db;

use chrono::{Duration, NaiveDate};

use tauri::Manager;

/// Open a native Save-As dialog, write data to the chosen path, return the path.
/// Returns Ok(None) if the user cancels.
#[tauri::command]
async fn save_excel(app: tauri::AppHandle, filename: String, data: Vec<u8>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();

    app.dialog()
        .file()
        .set_file_name(&filename)
        .add_filter("Excel Workbook", &["xlsx"])
        .save_file(move |path| {
            let _ = tx.send(path);
        });

    let path = rx.await.map_err(|e| e.to_string())?;

    match path {
        Some(p) => {
            let path_str = p.to_string();
            std::fs::write(&path_str, &data).map_err(|e| e.to_string())?;
            Ok(Some(path_str))
        }
        None => Ok(None),
    }
}

use commands::auth::{login, create_user, delete_user, list_users};
use commands::residents::{list_residents, list_residents_paged, get_resident, create_resident, update_resident, discharge_resident, mark_deceased};
use commands::care_logs::{list_care_logs, list_care_logs_history, create_care_log, update_care_log, flag_care_log, delete_care_log};
use commands::medications::{list_medications, create_medication, update_medication, stop_medication, list_med_administrations, record_med_administration};
use commands::staff::{list_staff, create_staff_member, update_staff_member, deactivate_staff, set_staff_role};
use commands::vitals::{list_vitals, create_vital, archive_vital, delete_vital};
use commands::meals::{list_meal_plans, list_meal_plans_range, upsert_meal_plan, bulk_upsert_meal_plans};
use commands::accounting::{list_invoices, list_expenses, get_accounting_summary};
use commands::settings::{get_setting, set_setting, list_settings};
use commands::schedules::{list_schedules, create_schedule, update_schedule, delete_schedule};
use commands::teams::{list_teams, get_user_team};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_dir)?;
            db::init(app_dir.to_str().unwrap())
                .expect("Failed to initialize database");

            seed_dev_data();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // auth
            login, create_user, delete_user, list_users,
            // residents
            list_residents, list_residents_paged, get_resident, create_resident, update_resident, discharge_resident, mark_deceased,
            // care logs
            list_care_logs, list_care_logs_history, create_care_log, update_care_log, flag_care_log, delete_care_log,
            // medications
            list_medications, create_medication, update_medication, stop_medication,
            list_med_administrations, record_med_administration,
            // staff
            list_staff, create_staff_member, update_staff_member, deactivate_staff, set_staff_role,
            // vitals
            list_vitals, create_vital, archive_vital, delete_vital,
            // meals
            list_meal_plans, list_meal_plans_range, upsert_meal_plan, bulk_upsert_meal_plans,
            // accounting
            list_invoices, list_expenses, get_accounting_summary,
            // settings
            get_setting, set_setting, list_settings,
            // schedules
            list_schedules, create_schedule, update_schedule, delete_schedule,
            // teams
            list_teams, get_user_team,
            // utils
            save_excel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn upsert_dev_user(db: &rusqlite::Connection, username: &str, password: &str, full_name: &str, role: &str) {
    let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
    let exists: i64 = db
        .query_row("SELECT COUNT(*) FROM users WHERE username = ?1", [username], |r| r.get(0))
        .unwrap_or(0);
    if exists == 0 {
        db.execute(
            "INSERT INTO users (username, password_hash, full_name, role) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![username, hash, full_name, role],
        ).unwrap();
    } else {
        db.execute(
            "UPDATE users SET password_hash = ?1, full_name = ?2, role = ?3 WHERE username = ?4",
            rusqlite::params![hash, full_name, role, username],
        ).unwrap();
    }
}

fn seed_dev_data() {
    let db = db::get().lock().unwrap();

    // Dev accounts (always upsert so passwords/names stay current)
    upsert_dev_user(&db, "staff",    "1234", "Jane Cooper",      "staff");
    upsert_dev_user(&db, "staff2",   "1234", "Tom Nguyen",       "staff");
    upsert_dev_user(&db, "staff3",   "1234", "Maria Santos",     "staff");
    upsert_dev_user(&db, "staff4",   "1234", "Kevin Park",       "staff");
    upsert_dev_user(&db, "staff5",   "1234", "Linda Osei",       "staff");
    upsert_dev_user(&db, "staff6",   "1234", "David Tremblay",   "staff");
    upsert_dev_user(&db, "staff7",   "1234", "Emily Wright",     "staff");
    upsert_dev_user(&db, "staff8",   "1234", "Carlos Reyes",     "staff");
    upsert_dev_user(&db, "staff9",   "1234", "Priya Patel",      "staff");
    upsert_dev_user(&db, "staff10",  "1234", "James O'Brien",    "staff");
    upsert_dev_user(&db, "staff11",  "1234", "Sofia Larsson",    "staff");
    upsert_dev_user(&db, "staff12",  "1234", "Ahmed Hassan",     "staff");
    upsert_dev_user(&db, "staff13",  "1234", "Rachel Kim",       "staff");
    upsert_dev_user(&db, "staff14",  "1234", "Bruno Côté",       "staff");
    upsert_dev_user(&db, "staff15",  "1234", "Natalie Bouchard", "staff");
    upsert_dev_user(&db, "manager",  "1234", "Robert Mills",     "manager");
    upsert_dev_user(&db, "manager2", "1234", "Sarah Chen",       "manager");
    upsert_dev_user(&db, "manager3", "1234", "Catherine Dubois", "manager");
    upsert_dev_user(&db, "manager4", "1234", "Michael Torres",   "manager");
    upsert_dev_user(&db, "admin",    "1234", "jj",               "admin");

    // Settings defaults
    db.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('shift_model', '12h')",
        [],
    ).unwrap();

    // Collect staff IDs for distributing logs / vitals (always, not gated)
    let mut staff_ids: Vec<i64> = Vec::new();
    for uname in &[
        "staff","staff2","staff3","staff4","staff5","staff6",
        "staff7","staff8","staff9","staff10","staff11","staff12","staff13","staff14","staff15",
        "manager","manager2","manager3","manager4",
    ] {
        if let Ok(id) = db.query_row(
            "SELECT id FROM users WHERE username = ?1", [uname], |r| r.get::<_,i64>(0)
        ) { staff_ids.push(id); }
    }
    if staff_ids.is_empty() { staff_ids.push(1); }

    // ── One-time static data (residents, staff profiles, meds, meals, accounting) ──
    let resident_count: i64 = db
        .query_row("SELECT COUNT(*) FROM residents", [], |r| r.get(0))
        .unwrap_or(0);
    if resident_count == 0 {

    // (last, first, dob, gender, room, admit, care_grade, cognitive_support, diagnosis)
    let residents: &[(&str, &str, &str, &str, &str, &str, i64, bool, &str)] = &[
        ("Thompson",   "Margaret",  "1938-04-15", "female", "101", "2023-06-01", 3, true,  "Alzheimer's disease"),
        ("Williams",   "Robert",    "1942-11-22", "male",   "102", "2023-08-15", 2, false, "Hypertension, Type 2 Diabetes"),
        ("Johnson",    "Dorothy",   "1936-07-08", "female", "103", "2024-01-10", 4, true,  "Vascular dementia, Hip fracture"),
        ("Davis",      "Harold",    "1944-02-28", "male",   "104", "2024-02-20", 1, false, "Hypertension"),
        ("Martinez",   "Eleanor",   "1935-09-03", "female", "105", "2023-11-05", 5, true,  "Parkinson's disease, Dysphagia"),
        ("Anderson",   "Walter",    "1940-05-17", "male",   "106", "2024-03-01", 2, false, "COPD, Heart failure"),
        ("Wilson",     "Patricia",  "1939-12-30", "female", "107", "2023-07-22", 3, false, "Osteoporosis, Depression"),
        ("Taylor",     "Eugene",    "1937-08-11", "male",   "108", "2024-01-28", 4, true,  "Cerebrovascular accident"),
        ("Brown",      "Ruth",      "1941-03-19", "female", "109", "2023-09-14", 2, false, "Type 2 Diabetes, Arthritis"),
        ("Miller",     "Gerald",    "1943-06-25", "male",   "110", "2024-04-05", 1, false, "Post-surgical recovery"),
        ("Harris",     "Florence",  "1937-01-12", "female", "111", "2023-05-18", 3, true,  "Lewy body dementia"),
        ("Clark",      "Raymond",   "1941-08-03", "male",   "112", "2023-09-22", 2, false, "Heart failure, Atrial fibrillation"),
        ("Lewis",      "Beatrice",  "1936-11-27", "female", "113", "2024-02-14", 4, true,  "Late-stage Alzheimer's"),
        ("Robinson",   "Stanley",   "1945-06-09", "male",   "114", "2024-03-30", 1, false, "Hip replacement recovery"),
        ("Walker",     "Edna",      "1939-03-21", "female", "115", "2023-12-01", 3, false, "Stroke, Left hemiplegia"),
        ("Hall",       "Clarence",  "1943-09-15", "male",   "116", "2024-01-07", 2, false, "COPD, Hypertension"),
        ("Allen",      "Violet",    "1934-05-30", "female", "117", "2023-07-10", 5, true,  "Advanced Parkinson's, Dysphagia"),
        ("Young",      "Chester",   "1940-12-18", "male",   "118", "2024-04-12", 2, false, "Type 2 Diabetes, Neuropathy"),
        ("Hernandez",  "Rosa",      "1938-07-25", "female", "119", "2023-10-05", 3, true,  "Mixed dementia"),
        ("King",       "Leonard",   "1944-02-08", "male",   "120", "2024-05-01", 1, false, "Fractured wrist, Rehabilitation"),
        ("Wright",     "Mildred",   "1936-09-14", "female", "121", "2023-08-28", 4, true,  "Dementia with behavioural symptoms"),
        ("Lopez",      "Manuel",    "1941-04-22", "male",   "122", "2024-01-19", 2, false, "Hypertension, Gout"),
        ("Hill",       "Agnes",     "1935-12-07", "female", "123", "2023-06-15", 5, true,  "LTC — End stage heart failure"),
        ("Scott",      "Howard",    "1942-08-31", "male",   "124", "2024-03-08", 2, false, "Arthritis, Hearing loss"),
        ("Green",      "Hazel",     "1938-01-19", "female", "125", "2023-11-20", 3, false, "Hip fracture, Osteoporosis"),
        ("Adams",      "Calvin",    "1943-07-14", "male",   "126", "2024-04-25", 1, false, "Knee replacement recovery"),
        ("Baker",      "Norma",     "1937-03-28", "female", "127", "2023-09-03", 3, true,  "Alzheimer's, Wandering behaviour"),
        ("Nelson",     "Clifford",  "1940-10-11", "male",   "128", "2024-02-06", 2, false, "Chronic kidney disease, Dialysis"),
        ("Carter",     "Irene",     "1936-06-24", "female", "129", "2023-07-17", 4, true,  "Frontotemporal dementia"),
        ("Mitchell",   "Arnold",    "1944-11-05", "male",   "130", "2024-05-14", 1, false, "Post-op monitoring, Hip surgery"),
        ("Perez",      "Carmen",    "1939-02-16", "female", "131", "2023-10-29", 3, false, "Stroke, Aphasia"),
        ("Roberts",    "Melvin",    "1941-05-07", "male",   "132", "2024-01-30", 2, false, "Heart failure, Edema"),
        ("Turner",     "Phyllis",   "1937-08-19", "female", "133", "2023-08-11", 4, true,  "Severe Alzheimer's, Incontinence"),
        ("Phillips",   "Lloyd",     "1945-01-23", "male",   "134", "2024-03-17", 1, false, "Respiratory infection recovery"),
        ("Campbell",   "Esther",    "1938-10-30", "female", "135", "2023-12-08", 3, false, "Parkinson's, Mild cognitive impairment"),
        ("Parker",     "Vernon",    "1942-04-13", "male",   "136", "2024-04-20", 2, false, "Diabetes, Peripheral vascular disease"),
        ("Evans",      "Loretta",   "1935-07-06", "female", "137", "2023-06-24", 5, true,  "LTC — Advanced dementia"),
        ("Edwards",    "Marvin",    "1940-02-27", "male",   "138", "2024-02-15", 2, false, "Emphysema, Home oxygen"),
        ("Collins",    "Lillian",   "1936-11-09", "female", "139", "2023-09-16", 4, true,  "Dementia, Agitation, Falls risk"),
        ("Stewart",    "Herbert",   "1943-05-21", "male",   "140", "2024-05-07", 1, false, "Short-term rehab post-stroke"),
        ("Sanchez",    "Dolores",   "1939-09-02", "female", "141", "2023-11-12", 3, false, "Osteoarthritis, Depression"),
        ("Morris",     "Lester",    "1941-01-15", "male",   "142", "2024-01-24", 2, false, "Atrial fibrillation, Anticoagulation"),
        ("Rogers",     "Gladys",    "1937-06-28", "female", "143", "2023-07-05", 4, true,  "Vascular dementia, Seizure disorder"),
        ("Reed",       "Wallace",   "1944-10-10", "male",   "144", "2024-04-03", 1, false, "Colostomy care, Wound healing"),
        ("Cook",       "Bertha",    "1938-03-23", "female", "145", "2023-10-17", 3, true,  "Lewy body dementia, Sleep disorder"),
        ("Morgan",     "Ralph",     "1940-07-04", "male",   "146", "2024-02-28", 2, false, "COPD, Cor pulmonale"),
        ("Bell",       "Lucille",   "1935-12-17", "female", "147", "2023-06-03", 5, true,  "LTC — Terminal cancer, Palliative"),
        ("Murphy",     "Dewey",     "1942-02-09", "male",   "148", "2024-03-24", 2, false, "Type 2 Diabetes, Retinopathy"),
        ("Bailey",     "Frances",   "1936-08-21", "female", "149", "2023-09-29", 3, false, "Hip fracture, Osteoporosis, Rehab"),
        ("Rivera",     "Raul",      "1943-11-14", "male",   "150", "2024-05-19", 1, false, "Knee surgery recovery"),
        ("Cooper",     "Gertrude",  "1938-04-26", "female", "151", "2023-12-26", 3, true,  "Early Alzheimer's, Wandering"),
        ("Richardson", "Otis",      "1941-09-08", "male",   "152", "2024-01-13", 2, false, "Hypertension, Chronic back pain"),
        ("Cox",        "Pauline",   "1937-01-31", "female", "153", "2023-08-20", 4, true,  "Mid-stage Alzheimer's"),
        ("Howard",     "Leo",       "1944-06-16", "male",   "154", "2024-04-07", 1, false, "Post-fall monitoring"),
        ("Ward",       "Cora",      "1939-10-28", "female", "155", "2023-11-04", 3, false, "Parkinson's, Dysphagia, Thickened fluids"),
        ("Torres",     "Ignacio",   "1940-03-12", "male",   "156", "2024-03-01", 2, false, "Heart failure, Diuretic therapy"),
        ("Peterson",   "Alma",      "1936-07-25", "female", "157", "2023-07-13", 5, true,  "Advanced dementia, Contractures"),
        ("Gray",       "Hubert",    "1942-12-07", "male",   "158", "2024-02-21", 2, false, "Chronic kidney disease, Anemia"),
        ("Ramirez",    "Consuelo",  "1938-05-19", "female", "159", "2023-10-10", 3, false, "Stroke, Right hemiplegia, Aphasia"),
        ("James",      "Cecil",     "1943-02-01", "male",   "160", "2024-05-25", 1, false, "Pneumonia recovery"),
        ("Watson",     "Minnie",    "1937-09-13", "female", "161", "2023-09-06", 4, true,  "Severe Alzheimer's, Tube feeding"),
        ("Brooks",     "Alvin",     "1940-01-26", "male",   "162", "2024-01-09", 2, false, "Emphysema, Anxiety"),
        ("Kelly",      "Ora",       "1935-06-08", "female", "163", "2023-06-20", 5, true,  "LTC — COPD, Palliative care"),
        ("Sanders",    "Willard",   "1941-10-21", "male",   "164", "2024-04-14", 2, false, "Diabetes, Chronic foot ulcer"),
        ("Price",      "Daisy",     "1938-03-04", "female", "165", "2023-12-15", 3, false, "Depression, Osteoarthritis"),
        ("Bennett",    "Sherman",   "1944-08-16", "male",   "166", "2024-03-11", 1, false, "Hip replacement, Early ambulation"),
        ("Wood",       "Elsie",     "1936-12-29", "female", "167", "2023-08-02", 4, true,  "Mixed dementia, Sundowning"),
        ("Barnes",     "Edgar",     "1942-05-11", "male",   "168", "2024-02-05", 2, false, "Atrial fibrillation, Warfarin therapy"),
        ("Ross",       "Thelma",    "1939-09-23", "female", "169", "2023-11-18", 3, false, "Parkinson's, Rigidity, Falls risk"),
        ("Henderson",  "Roscoe",    "1940-04-06", "male",   "170", "2024-05-02", 1, false, "Cataract surgery recovery"),
        ("Coleman",    "Mabel",     "1937-07-18", "female", "171", "2023-07-30", 4, true,  "Alzheimer's, Weight loss"),
        ("Jenkins",    "Warren",    "1943-01-30", "male",   "172", "2024-01-16", 2, false, "COPD, Recurrent pneumonia"),
        ("Perry",      "Hattie",    "1935-11-12", "female", "173", "2023-06-08", 5, true,  "LTC — Congestive heart failure"),
        ("Powell",     "Chester Jr","1941-04-24", "male",   "174", "2024-04-21", 2, false, "Type 2 Diabetes, Hypertension"),
        ("Long",       "Edith",     "1938-08-07", "female", "175", "2023-10-26", 3, true,  "Dementia, Resistant to personal care"),
        ("Patterson",  "Clifton",   "1944-02-19", "male",   "176", "2024-03-14", 1, false, "Shoulder surgery recovery"),
        ("Hughes",     "Beulah",    "1937-05-31", "female", "177", "2023-09-09", 3, false, "Osteoporosis, Multiple fractures"),
        ("Flores",     "Salvador",  "1940-10-14", "male",   "178", "2024-02-08", 2, false, "Prostate cancer, Palliative chemo"),
        ("Washington", "Ruby",      "1936-02-26", "female", "179", "2023-07-21", 4, true,  "Late-stage vascular dementia"),
        ("Butler",     "Elmer",     "1942-07-09", "male",   "180", "2024-05-16", 1, false, "Fractured tibia, Rehabilitation"),
        ("Simmons",    "Winifred",  "1939-12-21", "female", "181", "2023-12-03", 3, false, "Chronic pain, Fibromyalgia"),
        ("Foster",     "Emmett",    "1941-06-04", "male",   "182", "2024-01-27", 2, false, "Heart bypass recovery, Cardiac rehab"),
        ("Gonzales",   "Esperanza", "1937-10-17", "female", "183", "2023-08-14", 3, true,  "Mild Alzheimer's, Wandering risk"),
        ("Bryant",     "Cornelius", "1943-03-29", "male",   "184", "2024-04-08", 2, false, "Gout, Chronic kidney disease"),
        ("Alexander",  "Ida",       "1935-08-11", "female", "185", "2023-06-26", 5, true,  "Advanced dementia, Contractures, Bedsore"),
        ("Russell",    "Amos",      "1940-01-24", "male",   "186", "2024-03-05", 2, false, "Hypertension, Mild cognitive decline"),
        ("Griffin",    "Celestine", "1938-06-05", "female", "187", "2023-10-12", 3, false, "Stroke, Dysphagia, Rehab ongoing"),
        ("Diaz",       "Ernesto",   "1944-11-17", "male",   "188", "2024-05-09", 1, false, "Hernia repair recovery"),
        ("Hayes",      "Ethel",     "1937-03-30", "female", "189", "2023-09-23", 4, true,  "Alzheimer's, Aggression, Sundowning"),
        ("Myers",      "Lonnie",    "1941-08-12", "male",   "190", "2024-01-31", 2, false, "COPD, Sleep apnea, CPAP"),
        ("Ford",       "Lottie",    "1936-12-24", "female", "191", "2023-07-07", 4, true,  "Frontotemporal dementia, Disinhibition"),
        ("Hamilton",   "Ira",       "1943-04-06", "male",   "192", "2024-04-29", 1, false, "Cataract and fall recovery"),
        ("Graham",     "Naomi",     "1939-09-18", "female", "193", "2023-12-10", 3, false, "Parkinson's, Swallowing difficulty"),
        ("Sullivan",   "Woodrow",   "1942-02-21", "male",   "194", "2024-02-12", 2, false, "Atrial flutter, Rate control"),
        ("Wallace",    "Nettie",    "1938-07-03", "female", "195", "2023-11-27", 3, true,  "Lewy body dementia, Visual hallucinations"),
        ("Woods",      "Fletcher",  "1940-11-15", "male",   "196", "2024-03-22", 2, false, "Post-stroke rehab, Physiotherapy"),
        ("Cole",       "Lela",      "1935-04-28", "female", "197", "2023-06-14", 5, true,  "LTC — End-stage dementia"),
        ("West",       "Rupert",    "1941-09-10", "male",   "198", "2024-01-03", 2, false, "Type 2 Diabetes, Wound care"),
        ("Jordan",     "Mattie",    "1937-01-22", "female", "199", "2023-08-17", 3, false, "Hip osteoarthritis, Awaiting surgery"),
        ("Owens",      "Nathaniel", "1944-05-04", "male",   "200", "2024-04-16", 1, false, "Short-term IV antibiotic therapy"),
        ("Reynolds",   "Effie",     "1938-10-16", "female", "201", "2023-10-21", 3, true,  "Alzheimer's, Elopement risk"),
        ("Fisher",     "Orville",   "1943-03-28", "male",   "202", "2024-05-20", 1, false, "Lumbar surgery recovery"),
        ("Ellis",      "Josephine", "1936-07-10", "female", "203", "2023-09-01", 4, true,  "Advanced Alzheimer's, Restraint-free care"),
        ("Harrison",   "Virgil",    "1940-12-22", "male",   "204", "2024-02-19", 2, false, "Chronic obstructive arterial disease"),
        ("Gibson",     "Stella",    "1939-05-04", "female", "205", "2023-12-27", 3, false, "Stroke, Right-sided weakness"),
        ("Mcdonald",   "Rufus",     "1942-09-16", "male",   "206", "2024-04-02", 2, false, "Hypertensive heart disease"),
        ("Cruz",       "Amparo",    "1937-02-28", "female", "207", "2023-07-16", 4, true,  "Dementia, Wandering, Elopement risk"),
        ("Marshall",   "Sylvester", "1944-07-09", "male",   "208", "2024-05-27", 1, false, "Knee replacement, Day 3 post-op"),
        ("Ortiz",      "Guadalupe", "1938-11-21", "female", "209", "2023-11-07", 3, false, "Osteoporosis, Vertebral compression"),
        ("Gomez",      "Benito",    "1941-04-03", "male",   "210", "2024-01-22", 2, false, "COPD, Chronic cough"),
        ("Murray",     "Opal",      "1936-08-15", "female", "211", "2023-08-24", 4, true,  "Late dementia, Palliative approach"),
        ("Freeman",    "Horace",    "1943-01-27", "male",   "212", "2024-03-19", 2, false, "Heart failure, Fluid restriction"),
        ("Wells",      "Elnora",    "1935-06-09", "female", "213", "2023-06-29", 5, true,  "LTC — Severe Alzheimer's, Bed-bound"),
        ("Webb",       "Otis Jr",   "1940-10-21", "male",   "214", "2024-02-26", 2, false, "Diabetes, Bilateral leg oedema"),
        ("Simpson",    "Lavinia",   "1937-04-03", "female", "215", "2023-10-08", 3, false, "Depression, Hypothyroidism"),
        ("Stevens",    "Roland",    "1942-08-15", "male",   "216", "2024-05-05", 1, false, "Prostatectomy recovery"),
        ("Tucker",     "Harriet",   "1939-01-27", "female", "217", "2023-12-20", 3, true,  "Mild dementia, Social isolation"),
        ("Porter",     "Alvin Jr",  "1944-06-08", "male",   "218", "2024-04-24", 1, false, "Appendectomy recovery, Monitoring"),
        ("Hunter",     "Mamie",     "1938-09-20", "female", "219", "2023-09-28", 4, true,  "Alzheimer's, Agitation, PRN meds"),
        ("Hicks",      "Lowell",    "1941-02-12", "male",   "220", "2024-01-08", 2, false, "Chronic heart failure, Pacemaker"),
        // ── Discharged (5) ──────────────────────────────────────────────────────
        ("Stone",      "Myrtle",    "1943-07-19", "female", "221", "2023-03-12", 2, false, "Hip fracture rehab — discharged home"),
        ("Dixon",      "Leroy",     "1945-11-04", "male",   "222", "2023-05-20", 1, false, "Post-op knee replacement, completed rehab"),
        ("Hawkins",    "Velma",     "1940-09-27", "female", "223", "2023-08-01", 2, false, "Stroke recovery, transitioned to outpatient"),
        ("Garner",     "Douglas",   "1942-04-14", "male",   "224", "2023-10-09", 1, false, "Pneumonia recovery, discharged to family"),
        ("Pierce",     "Laverne",   "1938-12-30", "female", "225", "2024-02-03", 2, false, "Fractured wrist, short-term stay"),
        // ── Deceased (5) ────────────────────────────────────────────────────────
        ("Chandler",   "Rufus",     "1931-05-11", "male",   "226", "2022-11-14", 5, true,  "End-stage heart failure, palliative"),
        ("Norris",     "Mabel",     "1929-08-23", "female", "227", "2022-09-05", 5, true,  "Advanced Alzheimer's, end of life"),
        ("Bates",      "Leroy",     "1933-02-07", "male",   "228", "2023-01-18", 4, true,  "Metastatic cancer, comfort care"),
        ("Watts",      "Eunice",    "1930-11-15", "female", "229", "2022-07-30", 5, true,  "Severe dementia, multi-organ failure"),
        ("Cunningham", "Earl",      "1928-06-29", "male",   "230", "2023-04-22", 5, true,  "COPD end-stage, palliative"),
    ];

    for (last, first, dob, gender, room, admit, grade, cog, diagnosis) in residents {
            db.execute(
                "INSERT INTO residents (first_name, last_name, date_of_birth, gender, room_number,
                 admission_date, care_grade, cognitive_support, primary_diagnosis)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                rusqlite::params![first, last, dob, gender, room, admit, grade, *cog as i64, diagnosis],
            ).unwrap();
        }

        // ── Mark discharged residents ─────────────────────────────────────────
        for room in &["221", "222", "223", "224", "225"] {
            db.execute(
                "UPDATE residents SET is_active=0, discharge_date=?1 WHERE room_number=?2",
                rusqlite::params!["2024-12-31", room],
            ).unwrap();
        }
        // ── Mark deceased residents ───────────────────────────────────────────
        for (room, date) in &[("226","2024-03-15"),("227","2024-05-02"),("228","2024-07-20"),("229","2024-09-08"),("230","2024-11-27")] {
            db.execute(
                "UPDATE residents SET is_active=0, is_deceased=1, discharge_date=?1 WHERE room_number=?2",
                rusqlite::params![date, room],
            ).unwrap();
        }
        seed_staff_profiles(&db);
        seed_medications(&db);
        seed_meals(&db);
        seed_accounting(&db);
    }

    // ── Care logs + vitals: re-seed when seed_version < 2 ──────────────────────
    db.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('seed_version', '0')",
        [],
    ).unwrap();
    let seed_ver: i64 = db.query_row(
        "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'seed_version'",
        [], |r| r.get(0),
    ).unwrap_or(0);

    if seed_ver < 2 {
        db.execute("DELETE FROM care_logs", []).unwrap();
        db.execute("DELETE FROM vitals", []).unwrap();
        seed_care_logs(&db, &staff_ids);
        seed_vitals(&db, &staff_ids);
        db.execute(
            "UPDATE settings SET value = '2' WHERE key = 'seed_version'",
            [],
        ).unwrap();
    }

    // ── History data: stopped meds + archived vitals (seed_version 3) ────────
    if seed_ver < 3 {
        seed_med_history(&db);
        seed_vital_history(&db, &staff_ids);
        db.execute(
            "UPDATE settings SET value = '3' WHERE key = 'seed_version'",
            [],
        ).unwrap();
    }

    // ── Teams: seed once ────────────────────────────────────────────────────────
    let team_count: i64 = db
        .query_row("SELECT COUNT(*) FROM teams", [], |r| r.get(0))
        .unwrap_or(0);
    if team_count == 0 {
        seed_teams(&db);
    }

    // ── Schedules: seed example data once ──────────────────────────────────────
    let sched_count: i64 = db
        .query_row("SELECT COUNT(*) FROM schedules", [], |r| r.get(0))
        .unwrap_or(0);
    if sched_count == 0 {
        seed_schedules(&db, &staff_ids);
    }

    // ── Always: fill in today's care log entries if missing ────────────────────
    seed_today_if_needed(&db, &staff_ids);
}

// ── Stopped medications (history) ────────────────────────────────────────────
fn seed_med_history(db: &rusqlite::Connection) {
    let stopped: i64 = db.query_row(
        "SELECT COUNT(*) FROM medications WHERE is_active = 0", [], |r| r.get(0),
    ).unwrap_or(0);
    if stopped > 0 { return; }   // already have history entries

    // (resident_id, name, dosage, frequency, route, start_date, end_date, prescriber)
    let meds: &[(i64, &str, &str, &str, &str, &str, &str, &str)] = &[
        (1,  "Haloperidol",          "0.5mg",   "Once daily at bedtime",    "oral",    "2023-07-01", "2023-09-15", "Dr. Sarah Chen"),
        (2,  "Atenolol",             "25mg",    "Once daily",               "oral",    "2023-09-01", "2024-01-10", "Dr. James Patel"),
        (3,  "Gabapentin",           "100mg",   "Three times daily",        "oral",    "2024-02-01", "2024-06-30", "Dr. Sarah Chen"),
        (4,  "Hydrochlorothiazide",  "12.5mg",  "Once daily",               "oral",    "2024-03-15", "2024-08-20", "Dr. James Patel"),
        (5,  "Amantadine",           "100mg",   "Twice daily",              "oral",    "2023-12-01", "2024-03-01", "Dr. Michael Ross"),
        (6,  "Prednisolone",         "5mg",     "Once daily",               "oral",    "2024-04-01", "2024-06-15", "Dr. Michael Ross"),
        (7,  "Calcium + Vit D",      "500mg",   "Once daily",               "oral",    "2023-09-01", "2024-02-28", "Dr. Sarah Chen"),
        (8,  "Digoxin",              "0.125mg", "Once daily",               "oral",    "2024-03-01", "2024-07-31", "Dr. James Patel"),
        (9,  "Gliclazide",           "30mg",    "Once daily with breakfast", "oral",   "2023-10-15", "2024-04-30", "Dr. Michael Ross"),
        (10, "Pantoprazole",         "40mg",    "Once daily before meals",  "oral",    "2024-05-01", "2024-10-31", "Dr. Sarah Chen"),
        (11, "Risperidone",          "0.25mg",  "Once daily at bedtime",    "oral",    "2023-06-01", "2023-12-31", "Dr. Sarah Chen"),
        (12, "Bisoprolol",           "2.5mg",   "Once daily",               "oral",    "2023-10-01", "2024-05-15", "Dr. James Patel"),
        (13, "Lorazepam",            "0.5mg",   "As needed (PRN)",          "oral",    "2024-01-01", "2024-04-30", "Dr. Sarah Chen"),
        (14, "Codeine",              "30mg",    "As needed for pain",       "oral",    "2024-03-20", "2024-07-01", "Dr. James Patel"),
        (15, "Tramadol",             "50mg",    "Three times daily",        "oral",    "2023-12-15", "2024-03-20", "Dr. James Patel"),
        (16, "Dexamethasone",        "2mg",     "Once daily",               "oral",    "2024-02-10", "2024-05-10", "Dr. Michael Ross"),
        (17, "Clonazepam",           "0.5mg",   "Twice daily",              "oral",    "2023-11-01", "2024-06-30", "Dr. Sarah Chen"),
        (18, "Metoclopramide",       "10mg",    "Three times daily",        "oral",    "2024-01-20", "2024-04-20", "Dr. Michael Ross"),
        (20, "Cetirizine",           "10mg",    "Once daily",               "oral",    "2024-02-01", "2024-05-01", "Dr. Sarah Chen"),
        (21, "Mirtazapine",          "15mg",    "Once daily at bedtime",    "oral",    "2023-08-15", "2024-01-31", "Dr. Sarah Chen"),
    ];
    for (rid, name, dose, freq, route, start, end, prescriber) in meds {
        db.execute(
            "INSERT INTO medications
             (resident_id, name, dosage, frequency, route, start_date, end_date, prescriber, is_active)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0)",
            rusqlite::params![rid, name, dose, freq, route, start, end, prescriber],
        ).unwrap();
    }
}

// ── Archived vitals (health chart history) ────────────────────────────────────
fn seed_vital_history(db: &rusqlite::Connection, staff_ids: &[i64]) {
    let archived: i64 = db.query_row(
        "SELECT COUNT(*) FROM vitals WHERE COALESCE(is_archived, 0) = 1", [], |r| r.get(0),
    ).unwrap_or(0);
    if archived > 0 { return; }

    let mut stmt = db.prepare(
        "SELECT id FROM residents WHERE is_active = 1 ORDER BY id LIMIT 30"
    ).unwrap();
    let resident_ids: Vec<i64> = stmt
        .query_map([], |r| r.get(0)).unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Three sessions from 6–8 months ago — all inserted as archived
    for &days_ago in &[240i64, 210i64, 180i64] {
        for (res_idx, &resident_id) in resident_ids.iter().enumerate() {
            let sid = staff_ids[res_idx % staff_ids.len()];
            let h = ((resident_id as usize) * 17).wrapping_add((days_ago as usize) * 11);

            let base_weight = 47.0_f64 + (res_idx % 55) as f64 * 0.9;
            let weight      = ((base_weight * 10.0) + (h % 5) as f64) / 10.0;
            let bp_sys = 110i64 + (h % 58) as i64;
            let bp_dia = 64i64  + (h % 34) as i64;
            let hr     = 55i64  + (h % 46) as i64;
            let temp   = 36.1_f64 + (h % 14) as f64 * 0.1;
            let spo2   = 93i64  + (h % 6)  as i64;
            let blood_sugar: Option<i64> = if resident_id % 6 == 2 {
                Some(65 + (h % 75) as i64)
            } else {
                None
            };

            let measured_h = 8i64 + (h % 4) as i64;   // measured 08:00–11:00
            let archived_h = measured_h + 2;           // archived same day ~2h later

            db.execute(
                "INSERT INTO vitals
                 (resident_id, staff_id, bp_systolic, bp_diastolic,
                  heart_rate, temperature, weight, blood_sugar, spo2,
                  measured_at, is_archived, archived_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,
                         datetime('now','-'||?10||' days','+'||?11||' hours'),
                         1,
                         datetime('now','-'||?10||' days','+'||?12||' hours'))",
                rusqlite::params![
                    resident_id, sid, bp_sys, bp_dia, hr, temp, weight,
                    blood_sugar, spo2,
                    days_ago, measured_h, archived_h,
                ],
            ).ok();
        }
    }
}

fn seed_staff_profiles(db: &rusqlite::Connection) {
    // (username, employee_id, department, position, hire_date, phone, email)
    let profiles: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
        ("staff",    "0000001", "Personal Care",  "Health Care Aide",         "2022-03-14", "780-555-0101", "jcooper@sunshinecare.ca"),
        ("staff2",   "0000002", "Personal Care",  "Health Care Aide",         "2021-07-01", "780-555-0102", "tnguyen@sunshinecare.ca"),
        ("staff3",   "0000003", "Nursing",        "Licensed Practical Nurse", "2020-11-20", "780-555-0103", "msantos@sunshinecare.ca"),
        ("staff4",   "0000004", "Personal Care",  "Health Care Aide",         "2023-01-10", "780-555-0104", "kpark@sunshinecare.ca"),
        ("staff5",   "0000005", "Personal Care",  "Health Care Aide",         "2022-08-15", "780-555-0105", "losei@sunshinecare.ca"),
        ("staff6",   "0000006", "Nursing",        "Licensed Practical Nurse", "2021-04-05", "780-555-0106", "dtremblay@sunshinecare.ca"),
        ("staff7",   "0000007", "Personal Care",  "Health Care Aide",         "2023-06-12", "780-555-0107", "ewright@sunshinecare.ca"),
        ("staff8",   "0000008", "Nursing",        "Registered Nurse",         "2020-09-01", "780-555-0108", "creyes@sunshinecare.ca"),
        ("staff9",   "0000009", "Personal Care",  "Health Care Aide",         "2024-02-01", "780-555-0109", "ppatel@sunshinecare.ca"),
        ("staff10",  "0000010", "Nursing",        "Licensed Practical Nurse", "2021-11-15", "780-555-0110", "jobrien@sunshinecare.ca"),
        ("staff11",  "0000011", "Personal Care",  "Health Care Aide",         "2022-05-20", "780-555-0111", "slarsson@sunshinecare.ca"),
        ("staff12",  "0000012", "Nursing",        "Registered Nurse",         "2019-03-10", "780-555-0112", "ahassan@sunshinecare.ca"),
        ("staff13",  "0000013", "Personal Care",  "Health Care Aide",         "2023-09-05", "780-555-0113", "rkim@sunshinecare.ca"),
        ("staff14",  "0000014", "Personal Care",  "Health Care Aide",         "2022-12-01", "780-555-0114", "bcote@sunshinecare.ca"),
        ("staff15",  "0000015", "Nursing",        "Licensed Practical Nurse", "2021-08-22", "780-555-0115", "nbouchard@sunshinecare.ca"),
        ("manager",  "0000016", "Administration", "Care Manager",             "2019-06-01", "780-555-0201", "rmills@sunshinecare.ca"),
        ("manager2", "0000017", "Nursing",        "Director of Care",         "2018-02-14", "780-555-0202", "schen@sunshinecare.ca"),
        ("manager3", "0000018", "Administration", "Unit Manager",             "2020-04-07", "780-555-0203", "cdubois@sunshinecare.ca"),
        ("manager4", "0000019", "Nursing",        "Charge Nurse",             "2017-11-30", "780-555-0204", "mtorres@sunshinecare.ca"),
        ("admin",    "0000020", "Administration", "Administrator",            "2017-09-01", "780-555-0301", "jj@sunshinecare.ca"),
    ];
    for (uname, emp_id, dept, pos, hire, phone, email) in profiles {
        let user_id: i64 = match db.query_row(
            "SELECT id FROM users WHERE username = ?1", [uname], |r| r.get(0)
        ) {
            Ok(id) => id,
            Err(_) => continue,
        };
        // Only insert if not already in staff table
        let exists: i64 = db.query_row(
            "SELECT COUNT(*) FROM staff WHERE user_id = ?1", [user_id], |r| r.get(0)
        ).unwrap_or(0);
        if exists > 0 { continue; }
        db.execute(
            "INSERT INTO staff (user_id, employee_id, department, position, hire_date, phone, email)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![user_id, emp_id, dept, pos, hire, phone, email],
        ).unwrap();
    }
}

fn seed_medications(db: &rusqlite::Connection) {
    let count: i64 = db.query_row("SELECT COUNT(*) FROM medications", [], |r| r.get(0)).unwrap_or(0);
    if count > 5 { return; }
    // (resident_id, name, dosage, frequency, route, start_date, prescriber)
    let meds: &[(i64, &str, &str, &str, &str, &str, &str)] = &[
        (1, "Donepezil",        "10mg",   "Once daily at bedtime",   "oral", "2023-06-10", "Dr. Sarah Chen"),
        (1, "Memantine",        "10mg",   "Twice daily",             "oral", "2023-06-10", "Dr. Sarah Chen"),
        (2, "Amlodipine",       "5mg",    "Once daily",              "oral", "2023-08-20", "Dr. James Patel"),
        (2, "Metformin",        "500mg",  "Twice daily with meals",  "oral", "2023-08-20", "Dr. James Patel"),
        (2, "Lisinopril",       "10mg",   "Once daily",              "oral", "2023-08-20", "Dr. James Patel"),
        (3, "Rivastigmine",     "4.6mg",  "Once daily (patch)",      "topical", "2024-01-15", "Dr. Sarah Chen"),
        (3, "Aspirin",          "81mg",   "Once daily",              "oral", "2024-01-15", "Dr. Sarah Chen"),
        (4, "Ramipril",         "5mg",    "Once daily",              "oral", "2024-03-01", "Dr. James Patel"),
        (5, "Levodopa/Carbidopa","25/100mg","Three times daily",     "oral", "2023-11-10", "Dr. Michael Ross"),
        (5, "Baclofen",         "10mg",   "Three times daily",       "oral", "2023-11-10", "Dr. Michael Ross"),
        (6, "Salbutamol",       "100mcg", "As needed (max 4x/day)",  "inhaled", "2024-03-05", "Dr. Michael Ross"),
        (6, "Furosemide",       "40mg",   "Once daily in morning",   "oral", "2024-03-05", "Dr. Michael Ross"),
        (7, "Alendronate",      "70mg",   "Once weekly",             "oral", "2023-08-01", "Dr. Sarah Chen"),
        (7, "Sertraline",       "50mg",   "Once daily",              "oral", "2023-08-01", "Dr. Sarah Chen"),
        (8, "Warfarin",         "5mg",    "Once daily",              "oral", "2024-02-01", "Dr. James Patel"),
        (8, "Atorvastatin",     "20mg",   "Once daily at bedtime",   "oral", "2024-02-01", "Dr. James Patel"),
        (9, "Metformin",        "1000mg", "Twice daily with meals",  "oral", "2023-09-20", "Dr. Michael Ross"),
        (9, "Naproxen",         "250mg",  "Twice daily with food",   "oral", "2023-09-20", "Dr. Michael Ross"),
        (10,"Omeprazole",       "20mg",   "Once daily before meals", "oral", "2024-04-10", "Dr. Sarah Chen"),
    ];
    for (rid, name, dose, freq, route, start, prescriber) in meds {
        db.execute(
            "INSERT INTO medications (resident_id, name, dosage, frequency, route, start_date, prescriber)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![rid, name, dose, freq, route, start, prescriber],
        ).unwrap();
    }
}

/// Insert care log entries for one specific day (days_ago=0 means today).
/// `offset` shifts the content-rotation index so repeated calls produce different content.
fn seed_care_log_day(db: &rusqlite::Connection, staff_ids: &[i64], resident_ids: &[i64], days_ago: i64, offset: usize) {
    const BATHING: &[&str] = &[
        "Full assistance with morning shower. Skin intact, no pressure areas. Moisturiser applied to dry areas.",
        "Bed bath completed — resident declined shower. Oral care and hair styling done. No skin breakdown.",
        "Assisted shower — resident managed upper body independently. Compression stockings reapplied post-shower.",
        "Morning hygiene with full assist. Dentures cleaned. Perineal care completed. No concerns.",
        "Two-staff assisted shower. Resident tolerated well. Skin assessment done — no redness or concerns.",
    ];
    const MEALS: &[&str] = &[
        "Ate 80% of breakfast. Encouraged fluids — 300ml water and 150ml juice consumed.",
        "Full lunch consumed. Appetite excellent. Fluid intake 450ml this shift.",
        "Ate 60% of dinner. Mild appetite decrease noted — dietitian to be informed if it continues.",
        "Pureed diet consumed in full. No coughing episodes. Good fluid intake throughout meal.",
        "Refused breakfast initially; accepted after 30-minute wait. Ate ~50% of portion.",
        "Appetite good. Requested additional fruit. Fluids 400ml for the shift.",
    ];
    const MEDICATION: &[&str] = &[
        "Morning medications administered as prescribed. Resident cooperative, no refusals.",
        "All medications taken without difficulty. BP check completed — within expected range.",
        "Medications given as scheduled. Resident required gentle prompting for second dose.",
        "Evening medications administered. Pain rated 2/10; PRN analgesia given as prescribed.",
        "PRN medication given at 14:30 for reported discomfort. Resident settled after 20 minutes.",
        "Medications administered. One tablet initially refused — given with apple juice successfully.",
    ];
    const MOOD: &[&str] = &[
        "Resident in good spirits. Participated in morning group exercise for 20 minutes.",
        "Appeared slightly anxious. Redirected with familiar music — settled after 15 minutes.",
        "Calm and engaged. Positive interaction with fellow residents at lunch.",
        "Good mood. Resident requested phone call with family — assisted.",
        "Resident tearful in the afternoon. Spent time with resident; comforted and appeared settled.",
        "Cheerful and talkative. Participated enthusiastically in afternoon reminiscence group.",
        "Mild agitation around 15:00. Redirected successfully. Sundowning pattern monitored.",
    ];
    const NOTE: &[&str] = &[
        "Repositioned every 2 hours as per care plan. Skin integrity maintained throughout shift.",
        "Resident ambulating in corridor with walker. Steady gait, no falls observed.",
        "Physiotherapy exercises completed as per care plan. Resident tolerated activity well.",
        "Incident-free shift. Resident cooperative and settled throughout.",
        "Night: resident called for assistance at 02:00 for toileting. Resettled promptly.",
        "Sleep pattern good. Two position changes overnight. No skin breakdown observed.",
    ];
    const VISIT: &[&str] = &[
        "Family visit — daughter present for ~1 hour. Resident appeared happy and engaged.",
        "Dr. Chen visited: medication review completed. No changes to current care plan.",
        "Physiotherapy session completed. Good progress with mobility and balance noted.",
        "Family visit — son and grandchildren present for 2 hours. Resident visibly delighted.",
        "Registered dietitian review. Dietary plan updated to address caloric intake goals.",
        "Social worker visit. Family care conference notes reviewed and updated.",
        "Nurse practitioner assessment. Wound care reviewed — healing progressing as expected.",
        "Occupational therapist assessment. Adaptive equipment recommendations discussed with family.",
    ];
    const INCIDENT: &[&str] = &[
        "Resident found on floor beside bed. Assisted up. No visible injuries. Physician notified. Bed alarm reactivated.",
        "Resident attempted to exit building unassisted. Redirected safely. Family notified. Incident report completed.",
        "Choking episode during meal. Back blows administered. Resident recovered. Physician notified.",
        "Resident struck care aide during personal care. De-escalation used. Behaviour documented.",
        "Unexplained bruising on left forearm found during assessment. Physician notified. Investigation ongoing.",
    ];

    const SHIFTS:    &[&str] = &["day", "morning", "afternoon", "night"];
    const CAT_NAMES: &[&str] = &["bathing", "meals", "medication", "mood", "note"];

    let get_content = |cat_idx: usize, h: usize| -> &'static str {
        match cat_idx {
            0 => BATHING[h % BATHING.len()],
            1 => MEALS[h % MEALS.len()],
            2 => MEDICATION[h % MEDICATION.len()],
            3 => MOOD[h % MOOD.len()],
            _ => NOTE[h % NOTE.len()],
        }
    };

    for (res_idx, &resident_id) in resident_ids.iter().enumerate() {
        // Derive a stable "focus" category from the resident's ID
        let focus = ((resident_id - 1) % 5) as usize;
        let h = (days_ago as usize + offset) * 17 + res_idx * 7;

        // Primary daily entry
        let cat_idx = if h % 4 == 0 { focus } else { h % 5 };
        db.execute(
            "INSERT INTO care_logs (resident_id,staff_id,shift,category,content,is_incident,logged_at)
             VALUES (?1,?2,?3,?4,?5,0,datetime('now','-'||?6||' days','+'||?7||' hours'))",
            rusqlite::params![
                resident_id, staff_ids[h % staff_ids.len()],
                SHIFTS[h % 4], CAT_NAMES[cat_idx], get_content(cat_idx, h),
                days_ago, 6i64 + (h % 14) as i64,
            ],
        ).ok();

        // Second entry on ~⅓ of days
        if h % 3 == 0 {
            let h2 = h + 3;
            let cat2 = h2 % 5;
            db.execute(
                "INSERT INTO care_logs (resident_id,staff_id,shift,category,content,is_incident,logged_at)
                 VALUES (?1,?2,?3,?4,?5,0,datetime('now','-'||?6||' days','+'||?7||' hours'))",
                rusqlite::params![
                    resident_id, staff_ids[h2 % staff_ids.len()],
                    SHIFTS[h2 % 4], CAT_NAMES[cat2], get_content(cat2, h2),
                    days_ago, 14i64 + (h2 % 6) as i64,
                ],
            ).ok();
        }

        // Visit entry — once per week, staggered across residents
        if days_ago % 7 == (res_idx % 7) as i64 {
            let vh = h + 5;
            db.execute(
                "INSERT INTO care_logs (resident_id,staff_id,shift,category,content,is_incident,logged_at)
                 VALUES (?1,?2,'visit','note',?3,0,datetime('now','-'||?4||' days','+14 hours'))",
                rusqlite::params![
                    resident_id, staff_ids[vh % staff_ids.len()],
                    VISIT[vh % VISIT.len()], days_ago,
                ],
            ).ok();
        }

        // Incident — ~1 in 30
        if h % 30 == 0 {
            let ih = h + 1;
            db.execute(
                "INSERT INTO care_logs (resident_id,staff_id,shift,category,content,is_incident,logged_at)
                 VALUES (?1,?2,'day','incident',?3,1,datetime('now','-'||?4||' days','+10 hours'))",
                rusqlite::params![
                    resident_id, staff_ids[ih % staff_ids.len()],
                    INCIDENT[ih % INCIDENT.len()], days_ago,
                ],
            ).ok();
        }
    }
}

fn seed_care_logs(db: &rusqlite::Connection, staff_ids: &[i64]) {
    // Get all resident IDs
    let mut stmt = db.prepare("SELECT id FROM residents ORDER BY id").unwrap();
    let resident_ids: Vec<i64> = stmt
        .query_map([], |r| r.get(0)).unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Generate entries from 2026-04-01 to 2026-05-07 (36 days back from today)
    for days_ago in 0i64..=36 {
        seed_care_log_day(db, staff_ids, &resident_ids, days_ago, 0);
    }
}

/// Called every startup — inserts today's care log entries if the day hasn't been seeded yet.
fn seed_today_if_needed(db: &rusqlite::Connection, staff_ids: &[i64]) {
    let today_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM care_logs WHERE date(logged_at) = date('now')",
        [], |r| r.get(0),
    ).unwrap_or(0);
    if today_count >= 20 { return; }

    let mut stmt = db.prepare("SELECT id FROM residents ORDER BY id").unwrap();
    let resident_ids: Vec<i64> = stmt
        .query_map([], |r| r.get(0)).unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Use offset=200 so daily content rotates differently from the historical seed
    seed_care_log_day(db, staff_ids, &resident_ids, 0, 200);
}

fn seed_teams(db: &rusqlite::Connection) {
    // staff_ids order: staff(0)..staff15(14), manager(15), manager2(16), manager3(17), manager4(18)
    // Team A: manager=Robert Mills,    staff: Jane, Tom, Maria, Kevin        (idx 0–3)
    // Team B: manager=Sarah Chen,      staff: Linda, David, Emily, Carlos    (idx 4–7)
    // Team C: manager=Catherine Dubois,staff: Priya, James, Sofia, Ahmed     (idx 8–11)
    // Team D: manager=Michael Torres,  staff: Rachel, Bruno, Natalie         (idx 12–14)

    let teams: &[(&str, &str, &str, &[&str])] = &[
        ("Team A", "#1976D2", "manager",  &["staff",  "staff2", "staff3", "staff4"]),
        ("Team B", "#388E3C", "manager2", &["staff5", "staff6", "staff7", "staff8"]),
        ("Team C", "#F57C00", "manager3", &["staff9", "staff10","staff11","staff12"]),
        ("Team D", "#7B1FA2", "manager4", &["staff13","staff14","staff15"]),
    ];

    for (name, color, mgr_uname, members) in teams {
        let mgr_id: Option<i64> = db.query_row(
            "SELECT id FROM users WHERE username = ?1", [mgr_uname], |r| r.get(0)
        ).ok();

        db.execute(
            "INSERT INTO teams (name, color, manager_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, color, mgr_id],
        ).unwrap();

        let team_id = db.last_insert_rowid();

        // Assign team lead (manager) to the team in staff table
        if let Some(mid) = mgr_id {
            db.execute(
                "UPDATE staff SET team_id = ?1 WHERE user_id = ?2",
                rusqlite::params![team_id, mid],
            ).ok();
        }

        // Assign each staff member
        for uname in *members {
            db.execute(
                "UPDATE staff SET team_id = ?1
                 WHERE user_id = (SELECT id FROM users WHERE username = ?2)",
                rusqlite::params![team_id, uname],
            ).ok();
        }
    }
}

fn seed_schedules(db: &rusqlite::Connection, staff_ids: &[i64]) {
    if staff_ids.len() < 4 { return; }

    // ── Staff index map (matches order in staff_ids collection above) ──────────
    // idx  0 = staff   (Jane Cooper,      HCA)
    // idx  1 = staff2  (Tom Nguyen,       HCA)
    // idx  2 = staff3  (Maria Santos,     LPN)
    // idx  3 = staff4  (Kevin Park,       HCA)
    // idx  4 = staff5  (Linda Osei,       HCA)
    // idx  5 = staff6  (David Tremblay,   LPN)
    // idx  6 = staff7  (Emily Wright,     HCA)
    // idx  7 = staff8  (Carlos Reyes,     RN)
    // idx  8 = staff9  (Priya Patel,      HCA)
    // idx  9 = staff10 (James O'Brien,    LPN)
    // idx 10 = staff11 (Sofia Larsson,    HCA)
    // idx 11 = staff12 (Ahmed Hassan,     RN)
    // idx 12 = staff13 (Rachel Kim,       HCA)
    // idx 13 = staff14 (Bruno Côté,       HCA)
    // idx 14 = staff15 (Natalie Bouchard, LPN)
    // idx 15 = manager  (Robert Mills,    Care Manager)
    // idx 16 = manager2 (Sarah Chen,      Director of Care)
    // idx 17 = manager3 (Catherine Dubois,Unit Manager)
    // idx 18 = manager4 (Michael Torres,  Charge Nurse)
    //
    // Days: 0=Mon 1=Tue 2=Wed 3=Thu 4=Fri 5=Sat 6=Sun
    // Today = Thu May 7 → Mon Apr 20 = -17, Mon Apr 27 = -10, Mon May 4 = -3, Mon May 11 = +4

    struct ShiftSpec { staff_idx: usize, day: i64, start: &'static str, end_t: &'static str, hours: f64 }

    // ── Week A: 12-hour rotation ───────────────────────────────────────────────
    // DAY shift  07:00–19:00 (6 staff cover Mon–Sun in pairs)
    // NIGHT shift 19:00–07:00 (4 staff cover nights, 3 days each)
    // Managers: Mon–Fri 08:00–16:00
    let week_a: &[ShiftSpec] = &[
        // Day shift group 1 (HCAs): Mon/Wed/Fri
        ShiftSpec { staff_idx:  0, day: 0, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  0, day: 2, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  0, day: 4, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  3, day: 0, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  3, day: 2, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  3, day: 4, start: "07:00", end_t: "19:00", hours: 12.0 },
        // Day shift group 2 (HCAs): Tue/Thu/Sat
        ShiftSpec { staff_idx:  1, day: 1, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  1, day: 3, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  1, day: 5, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  6, day: 1, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  6, day: 3, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  6, day: 5, start: "07:00", end_t: "19:00", hours: 12.0 },
        // Day shift (LPN/RN): mixed days for clinical coverage
        ShiftSpec { staff_idx:  2, day: 0, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  2, day: 3, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  2, day: 6, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  7, day: 1, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  7, day: 4, start: "07:00", end_t: "19:00", hours: 12.0 },
        ShiftSpec { staff_idx:  7, day: 6, start: "07:00", end_t: "19:00", hours: 12.0 },
        // Night shift group (HCAs + LPN): 3 nights each
        ShiftSpec { staff_idx:  4, day: 0, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx:  4, day: 2, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx:  4, day: 4, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx:  8, day: 1, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx:  8, day: 3, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx:  8, day: 5, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx:  5, day: 0, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx:  5, day: 3, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx:  5, day: 6, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx: 14, day: 2, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx: 14, day: 4, start: "19:00", end_t: "07:00", hours: 12.0 },
        ShiftSpec { staff_idx: 14, day: 6, start: "19:00", end_t: "07:00", hours: 12.0 },
        // Managers: Mon–Fri 08:00–16:00
        ShiftSpec { staff_idx: 15, day: 0, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 15, day: 1, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 15, day: 2, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 15, day: 3, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 15, day: 4, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 0, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 1, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 2, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 3, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 4, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 0, start: "09:00", end_t: "17:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 1, start: "09:00", end_t: "17:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 2, start: "09:00", end_t: "17:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 3, start: "09:00", end_t: "17:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 4, start: "09:00", end_t: "17:00", hours: 8.0 },
        // Charge nurse: Mon–Fri days + one weekend
        ShiftSpec { staff_idx: 18, day: 0, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 1, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 2, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 3, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 4, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 5, start: "07:00", end_t: "15:00", hours: 8.0 },
    ];

    // ── Week B: 8-hour rotation ────────────────────────────────────────────────
    // Morning 07–15, Afternoon 15–23, Night 23–07
    let week_b: &[ShiftSpec] = &[
        // Morning 07–15 (HCAs + LPN)
        ShiftSpec { staff_idx:  0, day: 0, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  0, day: 1, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  0, day: 2, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  0, day: 3, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  0, day: 4, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  9, day: 0, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  9, day: 1, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  9, day: 2, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  9, day: 3, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx:  9, day: 4, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 10, day: 0, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 10, day: 2, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 10, day: 4, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 10, day: 5, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 11, day: 1, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 11, day: 3, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 11, day: 5, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 11, day: 6, start: "07:00", end_t: "15:00", hours: 8.0 },
        // Afternoon 15–23 (HCAs + LPN)
        ShiftSpec { staff_idx:  1, day: 0, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx:  1, day: 1, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx:  1, day: 2, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx:  1, day: 3, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx:  1, day: 4, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 12, day: 0, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 12, day: 1, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 12, day: 2, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 12, day: 3, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 12, day: 4, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 13, day: 1, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 13, day: 3, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 13, day: 5, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx: 13, day: 6, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx:  5, day: 2, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx:  5, day: 4, start: "15:00", end_t: "23:00", hours: 8.0 },
        ShiftSpec { staff_idx:  5, day: 6, start: "15:00", end_t: "23:00", hours: 8.0 },
        // Night 23–07
        ShiftSpec { staff_idx:  4, day: 0, start: "23:00", end_t: "07:00", hours: 8.0 },
        ShiftSpec { staff_idx:  4, day: 2, start: "23:00", end_t: "07:00", hours: 8.0 },
        ShiftSpec { staff_idx:  4, day: 4, start: "23:00", end_t: "07:00", hours: 8.0 },
        ShiftSpec { staff_idx:  8, day: 1, start: "23:00", end_t: "07:00", hours: 8.0 },
        ShiftSpec { staff_idx:  8, day: 3, start: "23:00", end_t: "07:00", hours: 8.0 },
        ShiftSpec { staff_idx:  8, day: 5, start: "23:00", end_t: "07:00", hours: 8.0 },
        ShiftSpec { staff_idx: 14, day: 0, start: "23:00", end_t: "07:00", hours: 8.0 },
        ShiftSpec { staff_idx: 14, day: 2, start: "23:00", end_t: "07:00", hours: 8.0 },
        ShiftSpec { staff_idx: 14, day: 6, start: "23:00", end_t: "07:00", hours: 8.0 },
        // Managers: Mon–Fri 08:00–16:00
        ShiftSpec { staff_idx: 15, day: 0, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 15, day: 1, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 15, day: 2, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 15, day: 3, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 15, day: 4, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 0, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 1, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 2, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 3, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 16, day: 4, start: "08:00", end_t: "16:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 0, start: "09:00", end_t: "17:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 1, start: "09:00", end_t: "17:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 2, start: "09:00", end_t: "17:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 3, start: "09:00", end_t: "17:00", hours: 8.0 },
        ShiftSpec { staff_idx: 17, day: 4, start: "09:00", end_t: "17:00", hours: 8.0 },
        // Charge nurse: Mon–Sat days
        ShiftSpec { staff_idx: 18, day: 0, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 1, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 2, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 3, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 4, start: "07:00", end_t: "15:00", hours: 8.0 },
        ShiftSpec { staff_idx: 18, day: 5, start: "07:00", end_t: "15:00", hours: 8.0 },
    ];

    // Mon Apr 20 = -17, Mon Apr 27 = -10, Mon May 4 = -3, Mon May 11 = +4
    let week_mondays: &[(i64, &[ShiftSpec])] = &[
        (-17, week_a),
        (-10, week_b),
        ( -3, week_a),
        (  4, week_b),
    ];

    for &(monday_offset, pattern) in week_mondays {
        for spec in pattern {
            let sid_idx = spec.staff_idx.min(staff_ids.len() - 1);
            let sid = staff_ids[sid_idx];
            let day_offset = monday_offset + spec.day;
            db.execute(
                "INSERT INTO schedules (staff_id, shift_date, shift_start, shift_end, shift_hours, created_by)
                 VALUES (?1, date('now', ?2 || ' days'), ?3, ?4, ?5, ?1)",
                rusqlite::params![
                    sid,
                    day_offset.to_string(),
                    spec.start,
                    spec.end_t,
                    spec.hours,
                ],
            ).ok();
        }
    }
}

fn seed_vitals(db: &rusqlite::Connection, staff_ids: &[i64]) {
    // Get all resident IDs
    let mut stmt = db.prepare("SELECT id FROM residents ORDER BY id").unwrap();
    let resident_ids: Vec<i64> = stmt
        .query_map([], |r| r.get(0)).unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Tuesday dates relative to 2026-05-07:
    //   2026-04-07 = 30 days ago — 1st Tuesday of April  → 80% of residents
    //   2026-04-14 = 23 days ago — 2nd Tuesday of April  → 20% of residents
    //   2026-05-05 =  2 days ago — 1st Tuesday of May    → 80% of residents
    //
    // Partition: resident_id % 5 != 0  → "Group A" (≈80%)
    //            resident_id % 5 == 0  → "Group B" (≈20%)
    let sessions: &[(i64, bool)] = &[
        (30, true),  // April  7 – 1st Tuesday – Group A (80%)
        (23, false), // April 14 – 2nd Tuesday – Group B (20%)
        ( 2, true),  // May    5 – 1st Tuesday – Group A (80%)
    ];

    for &(days_ago, is_first_tuesday) in sessions {
        for (res_idx, &resident_id) in resident_ids.iter().enumerate() {
            let in_group_a = resident_id % 5 != 0;
            if is_first_tuesday != in_group_a { continue; }

            let sid = staff_ids[res_idx % staff_ids.len()];
            // Deterministic hash for this resident × session
            let h = ((resident_id as usize) * 13).wrapping_add((days_ago as usize) * 7);

            // ── Vital signs: realistic ranges, deterministic per resident ──────
            // Weight stays consistent per-resident (anchored to res_idx)
            let base_weight = 47.0_f64 + (res_idx % 55) as f64 * 0.85;
            let wt_var      = (h % 6) as f64 * 0.1 - 0.2;    // ±0.2 kg variance
            let weight      = (base_weight + wt_var * 10.0).round() / 10.0;

            let bp_sys = 108i64 + (h % 62) as i64;            // 108–169 mmHg
            let bp_dia = 62i64  + (h % 38) as i64;            // 62–99  mmHg
            let hr     = 54i64  + (h % 48) as i64;            // 54–101 bpm
            let temp   = 36.0_f64 + (h % 16) as f64 * 0.1;   // 36.0–37.5 °C
            let spo2   = 92i64  + (h % 8)  as i64;            // 92–99 %

            // Blood sugar only for ~every 6th resident (approximate diabetics)
            let blood_sugar: Option<i64> = if resident_id % 6 == 2 {
                Some(60 + (h % 80) as i64) // 60–139 mg/dL
            } else {
                None
            };

            db.execute(
                "INSERT INTO vitals
                 (resident_id, staff_id, bp_systolic, bp_diastolic,
                  heart_rate, temperature, weight, blood_sugar, spo2, measured_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,
                         datetime('now','-'||?10||' days','+'||?11||' hours'))",
                rusqlite::params![
                    resident_id, sid, bp_sys, bp_dia, hr, temp, weight,
                    blood_sugar, spo2,
                    days_ago,
                    8i64 + (h % 4) as i64, // measured 08:00–11:00
                ],
            ).ok();
        }
    }
}

fn seed_meals(db: &rusqlite::Connection) {
    // 14 rotating weekly patterns (day 0=Mon..6=Sun)
    // Format: (day, meal_type, menu, calories)
    let patterns: &[&[(i64, &str, &str, i64)]] = &[
        // Pattern A
        &[
            (0,"breakfast","Scrambled eggs, whole wheat toast, orange juice, yogurt", 420),
            (0,"lunch",    "Chicken noodle soup, grilled cheese sandwich, apple slices", 550),
            (0,"dinner",   "Baked salmon, mashed potatoes, steamed broccoli, dinner roll", 680),
            (0,"snack",    "Graham crackers with peanut butter, warm milk", 210),
            (1,"breakfast","Oatmeal with berries, hard-boiled egg, cranberry juice", 390),
            (1,"lunch",    "Tomato soup, tuna sandwich on white bread, banana", 520),
            (1,"dinner",   "Roast chicken, rice pilaf, glazed carrots, dinner roll", 660),
            (1,"snack",    "Applesauce, vanilla wafers", 180),
            (2,"breakfast","French toast, turkey bacon, apple juice", 450),
            (2,"lunch",    "Beef vegetable soup, whole grain crackers, peach cup", 490),
            (2,"dinner",   "Meatloaf, scalloped potatoes, green beans, dinner roll", 700),
            (2,"snack",    "Cheese and crackers, orange juice", 220),
            (3,"breakfast","Pancakes with maple syrup, turkey sausage, orange juice", 480),
            (3,"lunch",    "Cream of mushroom soup, egg salad sandwich, grapes", 530),
            (3,"dinner",   "Baked cod, sweet potato, asparagus, dinner roll", 640),
            (3,"snack",    "Banana, peanut butter on crackers", 200),
            (4,"breakfast","Bagel with cream cheese, fresh fruit cup, apple juice", 410),
            (4,"lunch",    "Minestrone soup, grilled chicken wrap, melon slices", 560),
            (4,"dinner",   "Beef stew with vegetables, mashed potatoes, dinner roll", 720),
            (4,"snack",    "Pudding cup, vanilla wafers", 230),
            (5,"breakfast","Waffles, scrambled eggs, cranberry juice", 460),
            (5,"lunch",    "Clam chowder, BLT sandwich, apple", 580),
            (5,"dinner",   "Roast pork, stuffing, corn, dinner roll", 690),
            (5,"snack",    "Rice pudding, warm tea", 195),
            (6,"breakfast","Eggs Benedict, home fries, orange juice", 510),
            (6,"lunch",    "French onion soup, turkey club sandwich, fruit cup", 600),
            (6,"dinner",   "Roast beef, Yorkshire pudding, roasted vegetables, gravy", 750),
            (6,"snack",    "Ice cream, shortbread cookie", 260),
        ],
        // Pattern B
        &[
            (0,"breakfast","Porridge with honey and banana, orange juice", 380),
            (0,"lunch",    "Lentil soup, whole grain roll, pear", 510),
            (0,"dinner",   "Chicken pot pie, garden salad, dinner roll", 670),
            (0,"snack",    "Yogurt parfait, granola bar", 220),
            (1,"breakfast","Boiled eggs, rye toast, grapefruit juice", 370),
            (1,"lunch",    "Split pea soup, grilled ham sandwich, apple", 540),
            (1,"dinner",   "Pork tenderloin, roasted potatoes, steamed peas", 660),
            (1,"snack",    "Crackers and hummus, warm tea", 190),
            (2,"breakfast","Buttermilk pancakes, sausage links, orange juice", 490),
            (2,"lunch",    "Potato leek soup, egg salad on rye, orange slices", 520),
            (2,"dinner",   "Baked chicken thighs, wild rice, broccoli", 650),
            (2,"snack",    "Banana bread slice, milk", 230),
            (3,"breakfast","Granola with milk, sliced strawberries, apple juice", 400),
            (3,"lunch",    "Chicken tortilla soup, cheese quesadilla, banana", 560),
            (3,"dinner",   "Fish and chips, coleslaw, tartar sauce", 710),
            (3,"snack",    "Apple slices with caramel dip", 180),
            (4,"breakfast","Veggie omelette, toast, cranberry juice", 420),
            (4,"lunch",    "Cream of tomato soup, grilled cheese, fruit cup", 530),
            (4,"dinner",   "Beef tenderloin, mashed sweet potato, green beans", 700),
            (4,"snack",    "Pudding with whipped cream", 210),
            (5,"breakfast","French toast with berries, turkey sausage, OJ", 470),
            (5,"lunch",    "Corn chowder, chicken sandwich, peach cup", 570),
            (5,"dinner",   "Lamb stew, crusty bread, mixed vegetables", 730),
            (5,"snack",    "Cheese and grapes, sparkling water", 190),
            (6,"breakfast","Smoked salmon bagel, cream cheese, tomato juice", 490),
            (6,"lunch",    "Minestrone, focaccia, garden salad", 580),
            (6,"dinner",   "Prime rib, Yorkshire pudding, roasted root veg, gravy", 780),
            (6,"snack",    "Creme brûlée, herbal tea", 270),
        ],
    ];

    // Seed all Mondays from 2025-01-06 (first Monday ≥ 2025-01-01) to 2026-06-01
    let start_ts = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
    let end_ts   = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

    let mut cur = start_ts;
    let mut week_idx: usize = 0;
    while cur <= end_ts {
        let week_start = cur.format("%Y-%m-%d").to_string();
        let pattern    = patterns[week_idx % patterns.len()];
        for (day, meal, menu, cal) in pattern {
            db.execute(
                "INSERT OR IGNORE INTO meal_plans (week_start, day_of_week, meal_type, menu, calories)
                 VALUES (?1,?2,?3,?4,?5)",
                rusqlite::params![week_start, day, meal, menu, cal],
            ).unwrap();
        }
        cur += Duration::weeks(1);
        week_idx += 1;
    }
}

fn seed_accounting(db: &rusqlite::Connection) {
    // Invoices
    let invoices: &[(&str, i64, &str, f64, f64, &str, &str)] = &[
        ("INV-2026-001", 1, "2026-03", 3200.0, 450.0, "unpaid",  "2026-04-01"),
        ("INV-2026-002", 2, "2026-03", 2800.0, 380.0, "paid",    "2026-04-01"),
        ("INV-2026-003", 3, "2026-03", 3500.0, 620.0, "paid",    "2026-04-01"),
        ("INV-2026-004", 4, "2026-03", 2600.0, 320.0, "partial", "2026-04-01"),
        ("INV-2026-005", 5, "2026-03", 3800.0, 750.0, "unpaid",  "2026-04-01"),
        ("INV-2026-006", 6, "2026-03", 3100.0, 410.0, "paid",    "2026-04-01"),
        ("INV-2026-007", 1, "2026-04", 3200.0, 450.0, "unpaid",  "2026-05-01"),
        ("INV-2026-008", 2, "2026-04", 2800.0, 380.0, "unpaid",  "2026-05-01"),
    ];
    for (inv_no, rid, period, base, extra, status, due) in invoices {
        let total = base + extra;
        db.execute(
            "INSERT OR IGNORE INTO invoices
             (invoice_number, resident_id, billing_period, base_fee, care_fee, extra_charges, total_amount, status, due_date)
             VALUES (?1,?2,?3,?4,0,?5,?6,?7,?8)",
            rusqlite::params![inv_no, rid, period, base, extra, total, status, due],
        ).unwrap();
    }

    // Expenses
    let expenses: &[(&str, &str, f64, &str, &str)] = &[
        ("Medical Supplies", "Incontinence pads, gloves, masks — monthly stock",  1840.0, "Alberta Medical Supply Co.",  "2026-04-01"),
        ("Food & Catering",  "Weekly grocery delivery and meal prep services",     3200.0, "FreshFarm Catering Ltd.",      "2026-04-01"),
        ("Maintenance",      "HVAC filter replacement and quarterly inspection",    620.0, "ProCare Building Services",    "2026-04-02"),
        ("Medical Supplies", "Wound care dressings and IV supplies",               490.0, "Alberta Medical Supply Co.",   "2026-04-05"),
        ("Utilities",        "Electricity and natural gas — April billing",        1150.0, "ATCO Energy",                 "2026-04-10"),
        ("Food & Catering",  "Weekly grocery delivery and meal prep services",     3200.0, "FreshFarm Catering Ltd.",      "2026-04-08"),
        ("Equipment",        "Wheelchair repair and walker replacement parts",      380.0, "MedEquip Alberta",            "2026-04-12"),
        ("Medications",      "Pharmacy bulk order — resident prescriptions",       2740.0, "Shoppers Drug Mart",          "2026-04-15"),
        ("Cleaning",         "Commercial laundry service — weekly linens",         860.0, "CleanCare Services",          "2026-04-15"),
        ("Administrative",   "Office supplies and printer cartridges",             145.0, "Staples Business Depot",      "2026-04-18"),
    ];
    for (cat, desc, amt, vendor, date) in expenses {
        db.execute(
            "INSERT OR IGNORE INTO expenses (category, description, amount, vendor, expense_date)
             VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![cat, desc, amt, vendor, date],
        ).unwrap();
    }
}
