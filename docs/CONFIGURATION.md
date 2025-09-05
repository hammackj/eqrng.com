# Configuration Guide

EQ RNG now uses a centralized configuration system with environment-specific files and environment variable overrides.

## Configuration Files

### Default Configuration (`config/default.toml`)
Contains the base configuration that applies to all environments.

### Environment-Specific Configuration
- `config/development.toml` - Development environment settings
- `config/production.toml` - Production environment settings (create as needed)

## Configuration Structure

### Server Configuration
```toml
[server]
port = 3000
host = "0.0.0.0"
```

### Database Configuration
```toml
[database]
path = "./data/zones.db"
backup_dir = "./backups/database"
migrate_on_startup = true
```

### Security Configuration
```toml
[security]
min_ip_hash_key_length = 32
```

### Ratings Configuration
```toml
[ratings]
min_rating = 1
max_rating = 5
transaction_log_path = "./data/rating_transaction.log"
```

### Admin Configuration
```toml
[admin]
enabled = false  # Set to true for development
page_size = 20
min_page_size = 5
max_page_size = 100
default_sort_column = "name"
default_sort_order = "asc"
```

### CORS Configuration
```toml
[cors]
development_origins = [
    "http://localhost:3000",
    "http://localhost:5173"
]
production_origins = ["https://yourdomain.com"]
```

### Logging Configuration
```toml
[logging]
level = "info"  # debug, info, warn, error
format = "json"  # json or pretty
file_path = "./logs/eq_rng.log"
max_file_size = "10MB"
max_files = 5
```

## Environment Variables

You can override most configuration values using environment variables with the `EQ_RNG_` prefix. The `rating_ip_hash_key` is loaded exclusively from the `RATING_IP_HASH_KEY` environment variable:

```bash
# Override server port
export EQ_RNG_SERVER_PORT=8080

# Override database path
export EQ_RNG_DATABASE_PATH="/var/lib/eq_rng/zones.db"

# Set the required security key
export RATING_IP_HASH_KEY="your-production-key-here"

# Override logging level
export EQ_RNG_LOGGING_LEVEL="debug"
```

## Environment Detection

The application automatically detects the environment using the `EQ_RNG_ENV` environment variable:

```bash
# Development
export EQ_RNG_ENV=development

# Production
export EQ_RNG_ENV=production
```

If not set, it defaults to `development`.

## Configuration Validation

The configuration system automatically validates:
- Rating ranges (min < max)
- Admin pagination settings
- Security key lengths
- Required fields

## Logging

### Console Logging
- Development: Pretty format with colors
- Production: JSON format for log aggregation

### File Logging
- Automatic log rotation
- Configurable file sizes and retention
- JSON format for production

### Log Levels
- `debug` - Detailed debugging information
- `info` - General application information
- `warn` - Warning messages
- `error` - Error messages

## Example Environment Setup

### Development
```bash
export EQ_RNG_ENV=development
export EQ_RNG_LOGGING_LEVEL=debug
export EQ_RNG_ADMIN_ENABLED=true
```

### Production
```bash
export EQ_RNG_ENV=production
export EQ_RNG_LOGGING_LEVEL=info
export EQ_RNG_ADMIN_ENABLED=false
export RATING_IP_HASH_KEY="your-secure-production-key"
export EQ_RNG_SERVER_HOST="0.0.0.0"
export EQ_RNG_SERVER_PORT=3000
```

## Docker Configuration

When running in Docker, you can pass configuration via environment variables:

```yaml
# docker-compose.yml
environment:
  EQ_RNG_ENV: production
  EQ_RNG_LOGGING_LEVEL: info
  RATING_IP_HASH_KEY: "your-production-key"
```

## Migration from Old System

The old hardcoded constants have been replaced with configuration values:

- `DEFAULT_PAGE_SIZE` → `admin.page_size`
- `MIN_PAGE_SIZE` → `admin.min_page_size`
- `MAX_PAGE_SIZE` → `admin.max_page_size`
- `RATING_MIN` → `ratings.min_rating`
- `RATING_MAX` → `ratings.max_rating`

## Troubleshooting

### Configuration File Not Found
Ensure the `config/` directory exists and contains the required files.

### Invalid Configuration
Check the application logs for validation errors. Common issues:
- Invalid rating ranges
- Missing required fields
- Invalid file paths

### Permission Issues
Ensure the application has read access to configuration files and write access to log directories.
