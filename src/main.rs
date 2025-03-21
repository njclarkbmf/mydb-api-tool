use actix_web::{web, App, HttpServer, HttpResponse, Responder, error};
use mysql::{Pool, PooledConn, prelude::Queryable};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use thiserror::Error;

// Import serde_json with macro_use to enable the json! macro
#[macro_use]
extern crate serde_json;

// Configuration module
mod config {
    use serde::Deserialize;
    use std::env;
    use dotenv::dotenv;

    #[derive(Debug, Deserialize, Clone)]
    pub struct Settings {
        pub mysql_host: String,
        pub mysql_port: u16,
        pub mysql_user: String,
        pub mysql_password: String,
        pub mysql_db: String,
        pub app_port: u16,
    }

    impl Settings {
        pub fn new() -> Result<Self, config::ConfigError> {
            // Load environment variables from .env file if it exists
            dotenv().ok();
            
            let mysql_host = env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string());
            let mysql_port = env::var("MYSQL_PORT").unwrap_or_else(|_| "3306".to_string())
                .parse::<u16>().unwrap_or(3306);
            let mysql_user = env::var("MYSQL_USER").expect("MYSQL_USER must be set");
            let mysql_password = env::var("MYSQL_PASSWORD").expect("MYSQL_PASSWORD must be set");
            let mysql_db = env::var("MYSQL_DB").expect("MYSQL_DB must be set");
            let app_port = env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string())
                .parse::<u16>().unwrap_or(8080);
            
            Ok(Settings {
                mysql_host,
                mysql_port,
                mysql_user,
                mysql_password,
                mysql_db,
                app_port,
            })
        }
    }
}

// Define application state - the database connection pool
struct AppState {
    db_pool: Mutex<Pool>,
}

// Custom error type for our application
#[derive(Error, Debug)]
enum AppError {
    #[error("Database error: {0}")]
    DbError(#[from] mysql::Error),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
}

// Implementation to convert our custom error to HTTP responses
impl error::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound(message) => {
                HttpResponse::NotFound().json(json!({
                    "error": "Not Found",
                    "message": message
                }))
            },
            AppError::BadRequest(message) => {
                HttpResponse::BadRequest().json(json!({
                    "error": "Bad Request",
                    "message": message
                }))
            },
            AppError::DbError(err) => {
                log::error!("Database error: {:?}", err);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Internal Server Error",
                    "message": "A database error occurred"
                }))
            },
            AppError::InternalError(message) => {
                log::error!("Internal error: {}", message);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Internal Server Error",
                    "message": message
                }))
            }
        }
    }
}

// Helper function to get a database connection from the pool
fn get_conn(state: &AppState) -> Result<PooledConn, AppError> {
    let pool = state.db_pool.lock().map_err(|e| {
        AppError::InternalError(format!("Failed to acquire DB lock: {}", e))
    })?;
    
    pool.get_conn().map_err(AppError::DbError)
}

// Macro to create our serializable response types
macro_rules! define_response {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Serialize)]
        struct $name {
            $($field: $ty),*
        }
    }
}

// Response type definitions
define_response!(TablesResponse {
    tables: Vec<String>,
});

define_response!(TableColumnsResponse {
    table: String,
    columns: Vec<HashMap<String, String>>,
});

define_response!(ColumnValuesResponse {
    table: String,
    column: String,
    distinct_values: Vec<String>,
    limit: u32,
});

define_response!(TableCountResponse {
    table: String,
    total_count: u64,
});

define_response!(QueryResponse {
    table: String,
    field: String,
    value: String,
    columns: serde_json::Value, // Can be a string "all" or a Vec<String>
    limit: u32,
    results: Vec<HashMap<String, serde_json::Value>>,
});

// Request parameters
#[derive(Deserialize)]
struct ColumnValuesParams {
    limit: Option<u32>,
}

#[derive(Deserialize)]
struct QueryParams {
    field: Option<String>,
    value: Option<String>,
    columns: Option<String>,
    limit: Option<u32>,
}

// Route handlers
async fn list_tables(data: web::Data<AppState>) -> Result<impl Responder, AppError> {
    let mut conn = get_conn(&data)?;
    
    // Execute query to show tables
    let tables: Vec<String> = conn.query("SHOW TABLES")
        .map_err(AppError::DbError)?
        .into_iter()
        .map(|row: mysql::Row| {
            let value: String = mysql::from_row(row);
            value
        })
        .collect();
    
    Ok(HttpResponse::Ok().json(TablesResponse { tables }))
}

async fn table_columns(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, AppError> {
    let table = path.into_inner();
    let mut conn = get_conn(&data)?;
    
    // Execute query to show columns for the table
    let query = format!("SHOW COLUMNS FROM `{}`", table);
    let result = conn.query(query);
    
    match result {
        Ok(rows) => {
            let columns = rows.into_iter().map(|row: mysql::Row| {
                // Convert the row to a HashMap<String, String>
                let mut column_map = HashMap::new();
                for (i, column) in row.columns_ref().iter().enumerate() {
                    let column_name = column.name_str().to_string();
                    // Get the value safely, converting to string if possible, or empty string if null
                    let value = match row.get_opt::<mysql::Value, _>(i) {

                        Some(Ok(mysql::Value::NULL)) => String::new(),
                        Some(Ok(mysql::Value::Bytes(bytes))) => String::from_utf8_lossy(&bytes).to_string(),
                        Some(Ok(mysql::Value::Int(i))) => i.to_string(),
                        Some(Ok(mysql::Value::UInt(i))) => i.to_string(),
                        Some(Ok(mysql::Value::Float(f))) => f.to_string(),
                        Some(Ok(mysql::Value::Date(..))) |
                        Some(Ok(mysql::Value::Time(..))) => {
                            let s: Option<String> = row.get(i);
                            s.unwrap_or_default()
                        },
                        _ => String::new(),
                    };
                    column_map.insert(column_name, value);
                }
                column_map
            }).collect();
            
            Ok(HttpResponse::Ok().json(TableColumnsResponse { table, columns }))
        },
        Err(_) => {
            Err(AppError::NotFound(format!("Table '{}' not found", table)))
        }
    }
}

async fn column_distinct_values(
    data: web::Data<AppState>,
    path: web::Path<(String, String)>,
    query: web::Query<ColumnValuesParams>,
) -> Result<impl Responder, AppError> {
    let (table, column) = path.into_inner();
    let limit = std::cmp::min(query.limit.unwrap_or(20), 1000) as u32;
    let mut conn = get_conn(&data)?;
    
    // First check if the column exists
    let column_check_query = format!("SHOW COLUMNS FROM `{}` LIKE '{}'", table, column);
    let columns: Vec<mysql::Row> = conn.query(column_check_query)
        .map_err(|_| AppError::BadRequest(format!("Column '{}' not found in table '{}'", column, table)))?;
    
    if columns.is_empty() {
        return Err(AppError::BadRequest(format!("Column '{}' not found in table '{}'", column, table)));
    }
    
    // Get distinct values from the column
    let query = format!(
        "SELECT DISTINCT `{}` AS value FROM `{}` WHERE `{}` IS NOT NULL LIMIT {}",
        column, table, column, limit
    );
    
    let rows: Vec<mysql::Row> = conn.query(query)
        .map_err(AppError::DbError)?;
    
    let values: Vec<String> = rows.into_iter()
        .map(|row: mysql::Row| {
            let value: Option<String> = row.get(0);
            value.unwrap_or_default()
        })
        .collect();
    
    Ok(HttpResponse::Ok().json(ColumnValuesResponse {
        table,
        column,
        distinct_values: values,
        limit,
    }))
}

async fn table_row_count(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, AppError> {
    let table = path.into_inner();
    let mut conn = get_conn(&data)?;
    
    // First check if the table exists
    let table_check_query = format!("SHOW TABLES LIKE '{}'", table);
    let tables: Vec<mysql::Row> = conn.query(table_check_query)
        .map_err(AppError::DbError)?;
    
    if tables.is_empty() {
        return Err(AppError::BadRequest(format!("Table '{}' does not exist", table)));
    }
    
    // Get the row count
    let query = format!("SELECT COUNT(*) AS count FROM `{}`", table);
    let result: Vec<(u64,)> = conn.query(query)
        .map_err(AppError::DbError)?;
    
    let count = result.first().map(|r| r.0).unwrap_or(0);
    
    Ok(HttpResponse::Ok().json(TableCountResponse {
        table,
        total_count: count,
    }))
}

async fn query_table(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<QueryParams>,
) -> Result<impl Responder, AppError> {
    let table = path.into_inner();
    let field = query.field.clone().ok_or_else(|| 
        AppError::BadRequest("Please provide 'field' query parameter".to_string()))?;
    
    let value = query.value.clone().ok_or_else(|| 
        AppError::BadRequest("Please provide 'value' query parameter".to_string()))?;
    
    let limit = std::cmp::min(query.limit.unwrap_or(20), 1000);
    let mut conn = get_conn(&data)?;
    
    // Check if the field exists
    let field_check_query = format!("SHOW COLUMNS FROM `{}` LIKE '{}'", table, field);
    let fields: Vec<mysql::Row> = conn.query(field_check_query)
        .map_err(|_| AppError::BadRequest(format!("Field '{}' not found in table '{}'", field, table)))?;
    
    if fields.is_empty() {
        return Err(AppError::BadRequest(format!("Field '{}' not found in table '{}'", field, table)));
    }
    
    // Handle requested columns
    let columns_json;
    let columns_sql;
    
    if let Some(columns) = &query.columns {
        let requested_cols: Vec<String> = columns.split(',')
            .map(|col| col.trim().to_string())
            .collect();
        
        // Verify all columns exist
        let cols_query = format!("SHOW COLUMNS FROM `{}`", table);
        let available_columns: Vec<String> = conn.query(cols_query)
            .map_err(AppError::DbError)?
            .into_iter()
            .map(|row: mysql::Row| {
                // Extract just the Field column which contains the column name
                row.get::<String, _>("Field").unwrap_or_default()
            })
            .collect();
        
        let invalid_columns: Vec<&String> = requested_cols.iter()
            .filter(|col| !available_columns.contains(col))
            .collect();
        
        if !invalid_columns.is_empty() {
            return Err(AppError::BadRequest(format!(
                "Invalid columns requested: {:?}", 
                invalid_columns
            )));
        }
        
        columns_sql = requested_cols.iter()
            .map(|col| format!("`{}`", col))
            .collect::<Vec<String>>()
            .join(", ");
        
        columns_json = serde_json::Value::Array(
            requested_cols.into_iter()
                .map(|c| serde_json::Value::String(c))
                .collect()
        );
    } else {
        columns_sql = "*".to_string();
        columns_json = serde_json::Value::String("all".to_string());
    }
    
    // Execute the main query
    let query = format!(
        "SELECT {} FROM `{}` WHERE `{}` = ? LIMIT {}",
        columns_sql, table, field, limit
    );
    
    let prepared = conn.prep(query)
        .map_err(AppError::DbError)?;
    
    let rows = conn.exec_iter(prepared, (value.clone(),))
        .map_err(AppError::DbError)?;
    
    // Convert results to Vec<HashMap<String, Value>>
    let results = rows.into_iter().map(|row_result| {
        let row = row_result.unwrap();
        let mut map = HashMap::new();
        
        for (i, column) in row.columns_ref().iter().enumerate() {
            let column_name = column.name_str().to_string();
            
            // Handle different types of values
            let value: serde_json::Value = match row.get_opt::<mysql::Value, _>(i) {
                Some(Ok(mysql::Value::NULL)) => serde_json::Value::Null,
                Some(Ok(mysql::Value::Bytes(bytes))) => {
                    if let Ok(s) = String::from_utf8(bytes.clone()) {
                        serde_json::Value::String(s)
                    } else {
                        serde_json::Value::Array(
                            bytes.into_iter()
                                .map(|b| serde_json::Value::Number(b.into()))
                                .collect()
                        )
                    }
                },
                Some(Ok(mysql::Value::Int(i))) => serde_json::Value::Number(i.into()),
                Some(Ok(mysql::Value::UInt(i))) => {
                    if let Some(num) = serde_json::Number::from_u128(i as u128) {
                        serde_json::Value::Number(num)
                    } else {
                        serde_json::Value::String(i.to_string())
                    }
                },
                Some(Ok(mysql::Value::Float(f))) => {
                    if let Some(num) = serde_json::Number::from_f64(f.into()) {
                        serde_json::Value::Number(num)
                    } else {
                        serde_json::Value::String(f.to_string())
                    }
                },
                Some(Ok(mysql::Value::Date(..))) | 
                Some(Ok(mysql::Value::Time(..))) => {
                    // Convert dates to strings
                    let s: Option<String> = row.get(i);
                    serde_json::Value::String(s.unwrap_or_default())
                },
                _ => serde_json::Value::Null,
            };
            
            map.insert(column_name, value);
        }
        
        map
    }).collect();
    
    Ok(HttpResponse::Ok().json(QueryResponse {
        table,
        field,
        value,
        columns: columns_json,
        limit,
        results,
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Load configuration
    let settings = config::Settings::new()
        .expect("Failed to load configuration");
    
    // Create database connection pool
    let opts = mysql::OptsBuilder::new()
        .ip_or_hostname(Some(&settings.mysql_host))
        .tcp_port(settings.mysql_port)
        .user(Some(&settings.mysql_user))
        .pass(Some(&settings.mysql_password))
        .db_name(Some(&settings.mysql_db));
    
    let pool = mysql::Pool::new(opts)
        .expect("Failed to create database connection pool");
    
    // Test the connection
    let mut conn = pool.get_conn()
        .expect("Failed to connect to database");
    
    // Check connection by executing a simple query
    conn.query_drop("SELECT 1")
        .expect("Database connection test failed");
    
    log::info!("Successfully connected to database");
    
    // Create application state
    let state = web::Data::new(AppState {
        db_pool: Mutex::new(pool),
    });
    
    // Start the HTTP server
    log::info!("Starting server at http://0.0.0.0:{}", settings.app_port);
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            // Define routes
            .route("/tables", web::get().to(list_tables))
            .route("/tables/{table}/columns", web::get().to(table_columns))
            .route("/tables/{table}/columns/{column}/values", web::get().to(column_distinct_values))
            .route("/tables/{table}/count", web::get().to(table_row_count))
            .route("/query/{table}", web::get().to(query_table))
    })
    .bind(("0.0.0.0", settings.app_port))?
    .run()
    .await
}
