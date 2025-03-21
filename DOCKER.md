# Docker Guide for MySQL Database API Tool

This document provides comprehensive instructions for building, running, and troubleshooting the MySQL Database API Tool using Docker. Docker simplifies deployment by packaging the application with all its dependencies in a standardized environment.

## Container Overview

The Docker setup for this project uses a multi-stage build approach to create a slim and efficient container:

1. **Builder Stage**: Uses `rust:latest` to compile the application
2. **Runtime Stage**: Uses `debian:bullseye-slim` to run the compiled binary

This approach results in a much smaller final image, as it doesn't include the Rust compiler and build tools in the production container.

## Prerequisites

Before starting, ensure you have the following installed:

- Docker Engine (20.10.0 or newer)
- Docker Compose (optional, but recommended for easier configuration)
- Access to a MySQL server (either on your host machine or as another container)

## Building the Docker Image

You can build the Docker image using the following command from the project root:

```bash
docker build -t mysql-api-tool .
```

This command builds the image and tags it as `mysql-api-tool`.

## Running the Container

### Basic Run Command

You can run the container using the following command:

```bash
docker run -d --name mysql-api \
  -p 5000:5000 \
  -e MYSQL_HOST=host.docker.internal \
  -e MYSQL_PORT=3306 \
  -e MYSQL_USER=yourusername \
  -e MYSQL_PASSWORD=yourpassword \
  -e MYSQL_DB=yourdatabase \
  mysql-api-tool
```

### Environment Variables

The container can be configured using the following environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| MYSQL_HOST | MySQL server hostname | db |
| MYSQL_PORT | MySQL server port | 3306 |
| MYSQL_USER | MySQL username | root |
| MYSQL_PASSWORD | MySQL password | password |
| MYSQL_DB | MySQL database name | mydatabase |
| APP_PORT | Port the API will listen on | 5000 |

## Working with .env Files

### Understanding .env Files and Docker

One of the most convenient ways to configure the MySQL Database API Tool is by using an `.env` file. This approach lets you keep all your configuration in one place and use the same settings for both local development and containerized deployments.

Docker containers run in isolated environments and can't directly access files on your host machine unless you explicitly share them using volume mounts.

### Detailed .env File Setup

1. Create a `.env` file in your project directory if you don't already have one:

```
MYSQL_HOST=host.docker.internal
MYSQL_PORT=3306
MYSQL_USER=yourusername
MYSQL_PASSWORD=yourpassword
MYSQL_DB=yourdatabase
APP_PORT=5000
```

2. Start the container with a volume mount that makes your `.env` file available inside the container:

```bash
docker run -d --name mysql-api \
  -p 5000:5000 \
  -v $(pwd)/.env:/app/.env \
  mysql-api-tool
```

The `-v $(pwd)/.env:/app/.env` part creates a mapping between:
- Your local `.env` file in the current directory (`$(pwd)/.env`)
- The `/app/.env` path inside the container, where the application looks for configuration

3. Verify that your container is using the correct settings:

```bash
# Check container logs
docker logs mysql-api
```

### With Docker Compose

If using Docker Compose, you can add the volume mount to your `docker-compose.yml`:

```yaml
version: '3.8'

services:
  api:
    build: .
    ports:
      - "5000:5000"
    volumes:
      - ./.env:/app/.env
    # Other settings...
```

### Important .env Considerations

- **File Location**: The `.env` file must be mapped to `/app/.env` inside the container
- **Permissions**: Ensure your `.env` file has appropriate read permissions
- **Priority Order**: Environment variables passed via `-e` flags take precedence over those in the `.env` file
- **Updates**: If you modify your `.env` file, you must restart the container for changes to take effect:
  ```bash
  docker restart mysql-api
  ```

### Troubleshooting .env Files

If the container doesn't seem to be using your `.env` file settings:

1. **Verify the file is properly mounted**:
   ```bash
   docker exec -it mysql-api cat /app/.env
   ```
   This should display the contents of your `.env` file.

2. **Check the format of your .env file**:
   - Each line should be in the format `KEY=value`
   - No spaces around the equals sign
   - No quotes around values unless they're part of the value itself

3. **Check which environment variables are actually set in the container**:
   ```bash
   docker exec -it mysql-api env | grep MYSQL
   ```

4. **Try with explicit environment variables**:
   If mounting the `.env` file doesn't work, try using the `-e` flags as shown in the Basic Run Command section.

## Using Docker Compose

For an even simpler setup, create a `docker-compose.yml` file:

```yaml
version: '3.8'

services:
  api:
    build: .
    ports:
      - "5000:5000"
    environment:
      - MYSQL_HOST=db
      - MYSQL_PORT=3306
      - MYSQL_USER=root
      - MYSQL_PASSWORD=my-secret-pw
      - MYSQL_DB=mydatabase
    depends_on:
      - db

  db:
    image: mysql:8.0
    command: --default-authentication-plugin=mysql_native_password
    restart: always
    environment:
      - MYSQL_ROOT_PASSWORD=my-secret-pw
      - MYSQL_DATABASE=mydatabase
    ports:
      - "3306:3306"
    volumes:
      - mysql-data:/var/lib/mysql

volumes:
  mysql-data:
```

Then run:

```bash
docker-compose up -d
```

This will create both the API container and a MySQL container for your database.

## Connecting to an External MySQL Database

### Host Machine Database

If your MySQL database is running on your host machine, use the special Docker DNS name `host.docker.internal` (for Docker Desktop on Windows/Mac) or your host's actual IP address:

```bash
docker run -d --name mysql-api \
  -p 5000:5000 \
  -e MYSQL_HOST=host.docker.internal \
  -e MYSQL_USER=yourusername \
  -e MYSQL_PASSWORD=yourpassword \
  -e MYSQL_DB=yourdatabase \
  mysql-api-tool
```

### Remote MySQL Server

For a remote MySQL server, simply provide its host address:

```bash
docker run -d --name mysql-api \
  -p 5000:5000 \
  -e MYSQL_HOST=your.mysql.server.com \
  -e MYSQL_USER=yourusername \
  -e MYSQL_PASSWORD=yourpassword \
  -e MYSQL_DB=yourdatabase \
  mysql-api-tool
```

## Testing the Deployment

After starting the container, test the API using curl:

```bash
# List all tables
curl http://localhost:5000/tables

# Get columns for a specific table
curl http://localhost:5000/tables/users/columns

# Get distinct values from a column
curl "http://localhost:5000/tables/products/columns/category/values?limit=10"

# Get row count for a table
curl http://localhost:5000/tables/customers/count

# Query data with filters
curl "http://localhost:5000/query/orders?field=status&value=pending&columns=id,customer_id,total"
```

If the API is working correctly, you should receive JSON responses with your database information.

## Troubleshooting

### Container Won't Start

If the container exits immediately after starting, check the logs:

```bash
docker logs mysql-api
```

Common issues include:

1. **Database Connection Problems**: Ensure the MySQL connection details are correct.
   
   Solution: Double-check your environment variables or .env file.

2. **Port Conflicts**: If port 5000 is already in use on your host.
   
   Solution: Map to a different port:
   ```bash
   docker run -p 8080:5000 ...
   ```

3. **Insufficient Permissions**: The container might lack necessary permissions.
   
   Solution: Ensure your MySQL user has appropriate permissions for the database.

### Connection Refused Errors

If you see "connection refused" errors when the container tries to connect to MySQL:

1. **Network Issues**: The container might not be able to reach the MySQL server.
   
   Solution for host database: Ensure your MySQL server is configured to accept remote connections:
   ```sql
   CREATE USER 'yourusername'@'%' IDENTIFIED BY 'yourpassword';
   GRANT ALL PRIVILEGES ON yourdatabase.* TO 'yourusername'@'%';
   FLUSH PRIVILEGES;
   ```

2. **Firewall Blocking**: Your firewall might be blocking the MySQL port.
   
   Solution: Configure your firewall to allow connections on port 3306.

3. **Docker Network Issues**: If using Docker Compose with multiple containers, ensure they are on the same network.
   
   Solution: Check your Docker Compose network configuration or create a custom network:
   ```bash
   docker network create my-network
   docker run --network=my-network ...
   ```

### SSL Certificate Issues

If you encounter SSL-related errors:

```
SSL connection error: error:00000001:lib(0):func(0):reason(1)
```

Try disabling SSL for the MySQL connection by modifying your Rust code or use a secure connection with proper certificates.

### Environment Variable Issues

If the container doesn't seem to be using the correct configuration:

1. **Check environment variables in container**:
   ```bash
   docker exec -it mysql-api env | grep MYSQL
   ```

2. **Verify your `.env` file**:
   ```bash
   cat .env
   docker exec -it mysql-api cat /app/.env
   ```

3. **Try running with explicit environment variables**:
   ```bash
   docker run -e MYSQL_HOST=your_host ...
   ```

### Slow API Responses

If the API is responding slowly:

1. **Connection Pool Size**: The default connection pool might be too small.
   
   Solution: Adjust the pool size in your Rust code.

2. **Resource Constraints**: The container might need more resources.
   
   Solution: Allocate more CPU/memory:
   ```bash
   docker run -d --name mysql-api \
     --cpus=2 --memory=512m \
     ...
   ```

## Performance Tuning

### Container Resource Allocation

For production use, consider setting resource limits explicitly:

```bash
docker run -d --name mysql-api \
  --cpus=2 \
  --memory=512m \
  -p 5000:5000 \
  ...
```

### MySQL Connection Pooling

The application uses connection pooling for better performance. In a production environment, consider adjusting these settings based on expected load.

## Security Considerations

### Environment Variables

Avoid hardcoding sensitive information like database passwords in your Dockerfile or docker-compose.yml. Instead:

1. Use a secret management solution like Docker Secrets or Kubernetes Secrets
2. Mount a .env file from a secure location
3. Pass sensitive information as environment variables during deployment

For Docker Swarm users, consider using secrets:

```yaml
version: '3.8'
services:
  api:
    image: mysql-api-tool
    secrets:
      - db_password
    environment:
      - MYSQL_PASSWORD_FILE=/run/secrets/db_password

secrets:
  db_password:
    external: true
```

### Network Security

In production environments:

1. Use Docker networks to isolate containers
2. Consider using a reverse proxy like Nginx in front of the API
3. Implement proper firewall rules to restrict access

### Updating Base Images

Regularly update the base images to get security patches:

```bash
docker pull rust:latest
docker pull debian:bullseye-slim
docker build -t mysql-api-tool .
```

## Production Deployment Tips

For production use:

1. Set up health checks for the container
2. Implement proper logging and monitoring
3. Use a container orchestration tool like Kubernetes or Docker Swarm
4. Configure automatic container restarts

Example Docker Compose with health checks:

```yaml
version: '3.8'

services:
  api:
    build: .
    ports:
      - "5000:5000"
    environment:
      - MYSQL_HOST=db
      - MYSQL_USER=root
      - MYSQL_PASSWORD=my-secret-pw
      - MYSQL_DB=mydatabase
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:5000/tables"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    restart: unless-stopped
```

By following this guide, you should be able to successfully deploy, run, and troubleshoot the MySQL Database API Tool using Docker in various environments.
