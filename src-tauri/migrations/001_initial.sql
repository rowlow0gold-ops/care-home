-- ─────────────────────────────────────────────
-- Care Home Management System — Initial Schema
-- ─────────────────────────────────────────────

-- Settings
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Users (staff auth)
CREATE TABLE IF NOT EXISTS users (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    username     TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    full_name    TEXT NOT NULL,
    role         TEXT NOT NULL CHECK(role IN ('staff','manager','admin')),
    is_active    INTEGER NOT NULL DEFAULT 1,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Residents
CREATE TABLE IF NOT EXISTS residents (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    first_name       TEXT NOT NULL,
    last_name        TEXT NOT NULL,
    date_of_birth    TEXT NOT NULL,
    gender           TEXT NOT NULL CHECK(gender IN ('male','female','other')),
    room_number      TEXT,
    admission_date   TEXT NOT NULL DEFAULT (date('now')),
    discharge_date   TEXT,
    care_grade       INTEGER CHECK(care_grade BETWEEN 1 AND 5),
    cognitive_support INTEGER NOT NULL DEFAULT 0,
    primary_diagnosis TEXT,
    allergies        TEXT,
    dietary_restrictions TEXT,
    insurance_number TEXT,
    notes            TEXT,
    is_active        INTEGER NOT NULL DEFAULT 1,
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Emergency Contacts
CREATE TABLE IF NOT EXISTS emergency_contacts (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id  INTEGER NOT NULL REFERENCES residents(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    relationship TEXT NOT NULL,
    phone        TEXT NOT NULL,
    email        TEXT,
    is_primary   INTEGER NOT NULL DEFAULT 0
);

-- Care Logs
CREATE TABLE IF NOT EXISTS care_logs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id  INTEGER NOT NULL REFERENCES residents(id) ON DELETE CASCADE,
    staff_id     INTEGER REFERENCES users(id),
    shift        TEXT NOT NULL CHECK(shift IN ('morning','afternoon','day','night','visit')),
    category     TEXT NOT NULL CHECK(category IN ('bathing','meals','medication','mood','incident','note')),
    content      TEXT NOT NULL,
    is_incident  INTEGER NOT NULL DEFAULT 0,
    logged_at    TEXT NOT NULL DEFAULT (datetime('now')),
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Medications
CREATE TABLE IF NOT EXISTS medications (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id     INTEGER NOT NULL REFERENCES residents(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    dosage          TEXT NOT NULL,
    frequency       TEXT NOT NULL,
    route           TEXT NOT NULL DEFAULT 'oral',
    start_date      TEXT NOT NULL,
    end_date        TEXT,
    prescriber      TEXT,
    instructions    TEXT,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Medication Administrations
CREATE TABLE IF NOT EXISTS medication_administrations (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    medication_id INTEGER NOT NULL REFERENCES medications(id) ON DELETE CASCADE,
    resident_id   INTEGER NOT NULL REFERENCES residents(id) ON DELETE CASCADE,
    staff_id      INTEGER REFERENCES users(id),
    scheduled_at  TEXT NOT NULL,
    administered_at TEXT,
    status        TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','given','missed','refused')),
    notes         TEXT
);

-- Vitals
CREATE TABLE IF NOT EXISTS vitals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id     INTEGER NOT NULL REFERENCES residents(id) ON DELETE CASCADE,
    staff_id        INTEGER REFERENCES users(id),
    bp_systolic     INTEGER,
    bp_diastolic    INTEGER,
    heart_rate      INTEGER,
    temperature     REAL,
    weight          REAL,
    blood_sugar     INTEGER,
    spo2            INTEGER,
    notes           TEXT,
    measured_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Staff
CREATE TABLE IF NOT EXISTS staff (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER REFERENCES users(id),
    employee_id  TEXT UNIQUE,
    department   TEXT,
    position     TEXT,
    hire_date    TEXT,
    phone        TEXT,
    email        TEXT,
    hourly_rate  REAL,
    is_active    INTEGER NOT NULL DEFAULT 1,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Shifts
CREATE TABLE IF NOT EXISTS shifts (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    staff_id     INTEGER NOT NULL REFERENCES staff(id) ON DELETE CASCADE,
    shift_date   TEXT NOT NULL,
    shift_type   TEXT NOT NULL CHECK(shift_type IN ('morning','afternoon','day','night','visit')),
    start_time   TEXT,
    end_time     TEXT,
    status       TEXT NOT NULL DEFAULT 'scheduled' CHECK(status IN ('scheduled','completed','absent','leave'))
);

-- Attendance
CREATE TABLE IF NOT EXISTS attendance (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    staff_id     INTEGER NOT NULL REFERENCES staff(id) ON DELETE CASCADE,
    clock_in     TEXT,
    clock_out    TEXT,
    date         TEXT NOT NULL,
    notes        TEXT
);

-- Meal Plans
CREATE TABLE IF NOT EXISTS meal_plans (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    week_start   TEXT NOT NULL,
    day_of_week  INTEGER NOT NULL CHECK(day_of_week BETWEEN 0 AND 6),
    meal_type    TEXT NOT NULL CHECK(meal_type IN ('breakfast','lunch','dinner','snack')),
    menu         TEXT NOT NULL,
    calories     INTEGER,
    notes        TEXT,
    UNIQUE(week_start, day_of_week, meal_type)
);

-- Media
CREATE TABLE IF NOT EXISTS media (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id  INTEGER REFERENCES residents(id) ON DELETE CASCADE,
    staff_id     INTEGER REFERENCES users(id),
    file_name    TEXT NOT NULL,
    file_path    TEXT NOT NULL,
    media_type   TEXT NOT NULL CHECK(media_type IN ('photo','video')),
    caption      TEXT,
    event_tag    TEXT,
    taken_at     TEXT NOT NULL DEFAULT (datetime('now')),
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Notification Templates
CREATE TABLE IF NOT EXISTS notification_templates (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL,
    subject      TEXT NOT NULL,
    body         TEXT NOT NULL,
    trigger_type TEXT NOT NULL CHECK(trigger_type IN ('incident','monthly','emergency','custom')),
    is_active    INTEGER NOT NULL DEFAULT 1
);

-- Notifications Log
CREATE TABLE IF NOT EXISTS notifications (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id     INTEGER REFERENCES residents(id),
    template_id     INTEGER REFERENCES notification_templates(id),
    recipient_email TEXT NOT NULL,
    subject         TEXT NOT NULL,
    body            TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','sent','failed')),
    sent_at         TEXT,
    error_message   TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Invoices
CREATE TABLE IF NOT EXISTS invoices (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id    INTEGER NOT NULL REFERENCES residents(id),
    invoice_number TEXT NOT NULL UNIQUE,
    billing_period TEXT NOT NULL,
    base_fee       REAL NOT NULL DEFAULT 0,
    care_fee       REAL NOT NULL DEFAULT 0,
    extra_charges  REAL NOT NULL DEFAULT 0,
    total_amount   REAL NOT NULL DEFAULT 0,
    status         TEXT NOT NULL DEFAULT 'unpaid' CHECK(status IN ('unpaid','paid','partial','cancelled')),
    due_date       TEXT,
    issued_at      TEXT NOT NULL DEFAULT (datetime('now')),
    notes          TEXT
);

-- Payments
CREATE TABLE IF NOT EXISTS payments (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id   INTEGER NOT NULL REFERENCES invoices(id),
    amount       REAL NOT NULL,
    method       TEXT NOT NULL CHECK(method IN ('cash','bank_transfer','card','insurance')),
    paid_at      TEXT NOT NULL DEFAULT (datetime('now')),
    notes        TEXT
);

-- Expenses
CREATE TABLE IF NOT EXISTS expenses (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    category     TEXT NOT NULL,
    description  TEXT NOT NULL,
    amount       REAL NOT NULL,
    vendor       TEXT,
    expense_date TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Payroll
CREATE TABLE IF NOT EXISTS payroll (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    staff_id     INTEGER NOT NULL REFERENCES staff(id),
    pay_period   TEXT NOT NULL,
    base_pay     REAL NOT NULL DEFAULT 0,
    overtime_pay REAL NOT NULL DEFAULT 0,
    deductions   REAL NOT NULL DEFAULT 0,
    net_pay      REAL NOT NULL DEFAULT 0,
    status       TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','paid')),
    paid_at      TEXT
);

-- Insurance Claims
CREATE TABLE IF NOT EXISTS insurance_claims (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id     INTEGER NOT NULL REFERENCES residents(id),
    billing_period  TEXT NOT NULL,
    care_grade      INTEGER NOT NULL,
    service_days    INTEGER NOT NULL DEFAULT 0,
    base_amount     REAL NOT NULL DEFAULT 0,
    copay_amount    REAL NOT NULL DEFAULT 0,
    insurance_amount REAL NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','submitted','approved','rejected')),
    submitted_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Incident Reports
CREATE TABLE IF NOT EXISTS incident_reports (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    resident_id     INTEGER NOT NULL REFERENCES residents(id),
    staff_id        INTEGER REFERENCES users(id),
    incident_type   TEXT NOT NULL CHECK(incident_type IN ('fall','injury','complaint','death','medication_error','other')),
    description     TEXT NOT NULL,
    location        TEXT,
    witnesses       TEXT,
    immediate_action TEXT,
    follow_up       TEXT,
    severity        TEXT NOT NULL DEFAULT 'low' CHECK(severity IN ('low','medium','high','critical')),
    occurred_at     TEXT NOT NULL,
    reported_at     TEXT NOT NULL DEFAULT (datetime('now')),
    pdf_path        TEXT
);

-- Audit Log
CREATE TABLE IF NOT EXISTS audit_log (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER REFERENCES users(id),
    action       TEXT NOT NULL,
    table_name   TEXT,
    record_id    INTEGER,
    old_value    TEXT,
    new_value    TEXT,
    ip_address   TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Help Docs
CREATE TABLE IF NOT EXISTS help_docs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    page_key     TEXT NOT NULL UNIQUE,
    title        TEXT NOT NULL,
    content      TEXT NOT NULL,
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ─── Seed default admin user ────────────────────────────────────────────────
-- password: admin123 (bcrypt hashed — will be replaced on first run)
INSERT OR IGNORE INTO users (username, password_hash, full_name, role)
VALUES ('admin', '$2b$12$placeholder_replaced_at_runtime', 'Administrator', 'admin');

-- ─── Seed default settings ──────────────────────────────────────────────────
INSERT OR IGNORE INTO settings (key, value) VALUES
    ('facility_name', 'Sunshine Care Home'),
    ('shift_model', '12h'),
    ('smtp_host', ''),
    ('smtp_port', '587'),
    ('smtp_user', ''),
    ('smtp_password', ''),
    ('smtp_from', ''),
    ('db_version', '1');
