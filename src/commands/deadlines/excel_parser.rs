use anyhow::{Result, anyhow};
use calamine::{Reader, Xlsx, open_workbook};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ExcelWorkbook {
    pub sheets: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SheetData {
    pub name: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl ExcelWorkbook {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut workbook: Xlsx<_> = open_workbook(path)?;
        let sheets = workbook.sheet_names().to_owned();

        if sheets.is_empty() {
            return Err(anyhow!("Excel file contains no sheets"));
        }

        Ok(ExcelWorkbook { sheets })
    }

    pub fn read_sheet<P: AsRef<Path>>(path: P, sheet_name: &str) -> Result<SheetData> {
        let mut workbook: Xlsx<_> = open_workbook(path)?;

        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e| anyhow!("Error reading sheet '{}': {}", sheet_name, e))?;

        if range.is_empty() {
            return Ok(SheetData {
                name: sheet_name.to_string(),
                headers: Vec::new(),
                rows: Vec::new(),
            });
        }

        let mut rows_data = Vec::new();
        let mut headers = Vec::new();

        for (row_idx, row) in range.rows().enumerate() {
            let row_values: Vec<String> = row
                .iter()
                .map(|cell| cell.to_string())
                .collect();

            if row_idx == 0 {
                headers = row_values;
            } else {
                rows_data.push(row_values);
            }
        }

        Ok(SheetData {
            name: sheet_name.to_string(),
            headers,
            rows: rows_data,
        })
    }
}

impl SheetData {
    pub fn to_csv(&self) -> String {
        let mut csv_output = String::new();

        // Add headers
        csv_output.push_str(&self.headers.join(","));
        csv_output.push('\n');

        // Add rows
        for row in &self.rows {
            csv_output.push_str(&row.join(","));
            csv_output.push('\n');
        }

        csv_output
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.headers.len()
    }
}