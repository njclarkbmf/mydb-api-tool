# MySQL Database API Tool

A high-performance REST API for interacting with MySQL databases, written in Rust using the Actix-web framework. This API provides endpoints for exploring database structure and querying data with minimal configuration.

## Features

- List all tables in a database
- View columns for a specific table
- Get distinct values from table columns
- Count rows in tables
- Query tables with field-value filtering and column selection
- Efficient connection pooling
- Environment-based configuration
- Comprehensive error handling

## Setup Guide

### Prerequisites

- Rust (1.56.0 or later)
- Cargo (comes with Rust)
- MySQL or MariaDB server
- Ubuntu or Debian-based Linux (instructions specific to these systems)

### Installing Rust

If you don't have Rust installed, follow these steps:

```bash
# Install required dependencies
sudo apt update
sudo apt install build-essential curl wget git

# Download and run the rustup installer
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the interactive prompts, choosing option 1 for the default installation
# Then configure your current shell
source $HOME/.cargo/env

# Verify the installation
rustc --version
cargo --version
```

### Required System Dependencies

This project requires OpenSSL and pkg-config. Install them with:

```bash
sudo apt update
sudo apt install pkg-config libssl-dev
```

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/yourusername/mysql-db-api-tool.git
cd mysql-db-api-tool

# Build the project
cargo build --release
```

### Configuration

Create a `.env` file in the project root with the following variables:

```
MYSQL_HOST=localhost
MYSQL_PORT=3306
MYSQL_USER=your_username
MYSQL_PASSWORD=your_password
MYSQL_DB=your_database
APP_PORT=5000
```

## Running the Application

```bash
cargo run --release
```

The API will be available at `http://localhost:5000` (or whatever port you configured).

## API Usage

Here are examples of how to use the API endpoints:

### List all tables

```bash
curl http://localhost:5000/tables
```

Response:
```json
{
  "tables": ["users", "products", "orders", "categories"]
}
```

### View columns for a table

```bash
curl http://localhost:5000/tables/users/columns
```

Response:
```json
{
  "table": "users",
  "columns": [
    {
      "Field": "id",
      "Type": "int(11)",
      "Null": "NO",
      "Key": "PRI",
      "Default": null,
      "Extra": "auto_increment"
    },
    {
      "Field": "username",
      "Type": "varchar(255)",
      "Null": "NO",
      "Key": "",
      "Default": null,
      "Extra": ""
    }
  ]
}
```

### Get distinct values from a column

```bash
curl "http://localhost:5000/tables/categories/columns/name/values?limit=5"
```

Response:
```json
{
  "table": "categories",
  "column": "name",
  "distinct_values": ["Electronics", "Books", "Clothing", "Home", "Sports"],
  "limit": 5
}
```

### Count rows in a table

```bash
curl http://localhost:5000/tables/products/count
```

Response:
```json
{
  "table": "products",
  "total_count": 1283
}
```

### Query a table with filters

```bash
curl "http://localhost:5000/query/orders?field=status&value=shipped&columns=id,customer_id,total"
```

Response:
```json
{
  "table": "orders",
  "field": "status",
  "value": "shipped",
  "columns": ["id", "customer_id", "total"],
  "limit": 20,
  "results": [
    {
      "id": 1001,
      "customer_id": 5432,
      "total": 129.99
    },
    {
      "id": 1008,
      "customer_id": 8976,
      "total": 79.50
    }
  ]
}
```

## Troubleshooting

### OpenSSL Issues

If you encounter OpenSSL-related errors during compilation:

```
Could not find directory of OpenSSL installation, and this `-sys` crate cannot proceed without this knowledge.
```

This usually means you're missing the OpenSSL development libraries. Install them with:

```bash
sudo apt install pkg-config libssl-dev
```

If you still have issues, you can try setting the OpenSSL directory explicitly:

```bash
export OPENSSL_DIR=/usr/lib/ssl
cargo build
```

### MySQL Connection Issues

If you see errors connecting to the database:

1. Verify your `.env` file has the correct credentials
2. Ensure the MySQL server is running: `sudo systemctl status mysql`
3. Check that the user has proper permissions to access the database

### Type Conversion Errors

If you encounter errors like:

```
Couldn't convert Row to type alloc::string::String
```

This might happen with certain database structures. The code includes robust error handling for these cases, but if you encounter persistent issues, check for unusual column names or data types in your database schema.

### Handling Tables with Special Characters

For tables or columns with hyphens, spaces, or other special characters, the API properly escapes names with backticks. However, in your curl requests, you might need to URL-encode these characters:

```bash
# For a table named "order-items"
curl http://localhost:5000/tables/order-items/columns
```

## Performance Considerations

- The API uses connection pooling to efficiently manage database connections
- Limits are enforced on result sets to prevent memory issues with large tables
- For production use with high traffic, consider placing behind a reverse proxy like Nginx

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues to improve the project.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
