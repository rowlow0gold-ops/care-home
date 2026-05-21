use serde::{Deserialize, Serialize};
use tauri::command;
use crate::db;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoiceDto {
    pub id: i64,
    pub invoice_number: String,
    pub resident_id: i64,
    pub resident_name: String,
    pub billing_period: String,
    pub base_fee: f64,
    pub care_fee: f64,
    pub extra_charges: f64,
    pub total_amount: f64,
    pub status: String,
    pub due_date: Option<String>,
    pub issued_at: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExpenseDto {
    pub id: i64,
    pub category: String,
    pub description: String,
    pub amount: f64,
    pub vendor: Option<String>,
    pub expense_date: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountingSummary {
    pub total_invoiced: f64,
    pub total_collected: f64,
    pub total_outstanding: f64,
    pub total_expenses: f64,
}

#[command]
pub fn list_invoices(status: Option<String>) -> Result<Vec<InvoiceDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT i.id, i.invoice_number, i.resident_id,
                (r.first_name || ' ' || r.last_name),
                i.billing_period, i.base_fee, i.care_fee, i.extra_charges,
                i.total_amount, i.status, i.due_date, i.issued_at, i.notes
         FROM invoices i
         JOIN residents r ON r.id = i.resident_id
         WHERE (?1 IS NULL OR i.status = ?1)
         ORDER BY i.issued_at DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([status], |row| {
        Ok(InvoiceDto {
            id: row.get(0)?,
            invoice_number: row.get(1)?,
            resident_id: row.get(2)?,
            resident_name: row.get(3)?,
            billing_period: row.get(4)?,
            base_fee: row.get(5)?,
            care_fee: row.get(6)?,
            extra_charges: row.get(7)?,
            total_amount: row.get(8)?,
            status: row.get(9)?,
            due_date: row.get(10)?,
            issued_at: row.get(11)?,
            notes: row.get(12)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    Ok(rows)
}

#[command]
pub fn list_expenses() -> Result<Vec<ExpenseDto>, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT id, category, description, amount, vendor, expense_date
         FROM expenses ORDER BY expense_date DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        Ok(ExpenseDto {
            id: row.get(0)?,
            category: row.get(1)?,
            description: row.get(2)?,
            amount: row.get(3)?,
            vendor: row.get(4)?,
            expense_date: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    Ok(rows)
}

#[command]
pub fn get_accounting_summary() -> Result<AccountingSummary, String> {
    let db = db::get().lock().map_err(|e| e.to_string())?;

    let total_invoiced: f64 = db.query_row(
        "SELECT COALESCE(SUM(total_amount), 0) FROM invoices", [], |r| r.get(0)
    ).unwrap_or(0.0);

    let total_collected: f64 = db.query_row(
        "SELECT COALESCE(SUM(total_amount), 0) FROM invoices WHERE status = 'paid'", [], |r| r.get(0)
    ).unwrap_or(0.0);

    let total_outstanding: f64 = db.query_row(
        "SELECT COALESCE(SUM(total_amount), 0) FROM invoices WHERE status IN ('unpaid','partial')", [], |r| r.get(0)
    ).unwrap_or(0.0);

    let total_expenses: f64 = db.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM expenses", [], |r| r.get(0)
    ).unwrap_or(0.0);

    Ok(AccountingSummary { total_invoiced, total_collected, total_outstanding, total_expenses })
}
