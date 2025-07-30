# Database Backup and Restore Utilities

This directory contains utilities for backing up and restoring the SQLite database used by eq_rng.com.

## Files

- `backup_database.sh` - Creates backups of the database
- `restore_database.sh` - Restores database from backup files
- `README_BACKUP.md` - This documentation file

## Quick Start

### Creating a Backup

```bash
# Basic local backup
./utils/backup_database.sh

# Docker container backup
./utils/backup_database.sh --docker

# Compressed backup with cleanup
./utils/backup_database.sh --compress --keep 10
```

### Restoring from Backup

```bash
# List available backups
./utils/restore_database.sh --list

# Restore latest backup
./utils/restore_database.sh --latest

# Restore specific backup
./utils/restore_database.sh backups/database/zones_backup_20231215_143022.db
```

## Backup Script (`backup_database.sh`)

### Features

- **Dual Environment Support**: Works with both local development and Docker containerized deployments
- **SQLite Consistency**: Uses `sqlite3 .backup` command when available for consistent backups
- **Compression**: Optional gzip compression to save disk space
- **Automatic Cleanup**: Keep only the most recent N backups
- **Integrity Verification**: Verifies backup file integrity after creation
- **Comprehensive Logging**: Color-coded output with detailed status information

### Usage

```bash
./utils/backup_database.sh [OPTIONS]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-h, --help` | Show help message | - |
| `-d, --docker` | Backup from Docker container | Local filesystem |
| `-l, --local` | Backup from local filesystem | Default mode |
| `-o, --output DIR` | Specify backup directory | `backups/database` |
| `-n, --name NAME` | Specify backup filename | `zones_backup_YYYYMMDD_HHMMSS.db` |
| `-c, --container NAME` | Docker container name | `eq_rng-app-1` |
| `-k, --keep NUM` | Keep only last NUM backups | No cleanup |
| `--compress` | Compress backup with gzip | No compression |

#### Examples

```bash
# Create a basic local backup
./utils/backup_database.sh

# Create a backup from Docker container
./utils/backup_database.sh --docker

# Create compressed backup and keep only last 5
./utils/backup_database.sh --compress --keep 5

# Custom backup location and name
./utils/backup_database.sh --output /path/to/backups --name my_backup.db

# Docker backup with custom container name
./utils/backup_database.sh --docker --container my_eq_rng_container
```

### Backup Process

1. **Dependency Check**: Verifies required tools are available
2. **Directory Creation**: Creates backup directory if it doesn't exist
3. **Database Backup**: Uses SQLite's `.backup` command or file copy as fallback
4. **Compression** (optional): Compresses the backup with gzip
5. **Verification**: Checks backup integrity
6. **Cleanup** (optional): Removes old backups if `--keep` is specified

## Restore Script (`restore_database.sh`)

### Features

- **Safe Restoration**: Creates safety backup before restoring
- **Backup Listing**: List all available backup files with metadata
- **Latest Backup**: Automatically restore from the most recent backup
- **Compressed Support**: Handles both regular and gzip-compressed backups
- **Integrity Verification**: Verifies backup before restoration
- **Confirmation Prompts**: Requires user confirmation unless forced

### Usage

```bash
./utils/restore_database.sh [OPTIONS] [BACKUP_FILE]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-h, --help` | Show help message | - |
| `-d, --docker` | Restore to Docker container | Local filesystem |
| `-l, --local` | Restore to local filesystem | Default mode |
| `-c, --container NAME` | Docker container name | `eq_rng-app-1` |
| `-f, --force` | Force restore without confirmation | Prompt for confirmation |
| `--list` | List available backup files | - |
| `--latest` | Restore from most recent backup | - |

#### Examples

```bash
# List all available backups
./utils/restore_database.sh --list

# Restore from latest backup with confirmation
./utils/restore_database.sh --latest

# Restore specific backup file
./utils/restore_database.sh backups/database/zones_backup_20231215_143022.db

# Force restore to Docker container without confirmation
./utils/restore_database.sh --docker --force --latest

# Restore compressed backup
./utils/restore_database.sh backups/database/zones_backup_20231215_143022.db.gz
```

### Restore Process

1. **Backup Verification**: Verifies integrity of the backup file
2. **Safety Backup**: Creates a backup of current database before restore
3. **User Confirmation**: Prompts for confirmation (unless `--force` is used)
4. **Decompression** (if needed): Extracts compressed backup files
5. **Database Restoration**: Replaces current database with backup
6. **Cleanup**: Removes temporary files

## Directory Structure

```
eq_rng.com/
├── data/
│   └── zones.db              # Main database file
├── backups/
│   └── database/             # Backup storage directory
│       ├── zones_backup_20231215_143022.db
│       ├── zones_backup_20231215_143022.db.gz
│       └── zones_backup_before_restore_20231215_144500.db
└── utils/
    ├── backup_database.sh    # Backup utility
    ├── restore_database.sh   # Restore utility
    └── README_BACKUP.md      # This file
```

## Backup Naming Convention

Backup files follow this naming pattern:
- Regular backups: `zones_backup_YYYYMMDD_HHMMSS.db`
- Compressed backups: `zones_backup_YYYYMMDD_HHMMSS.db.gz`
- Safety backups: `zones_backup_before_restore_YYYYMMDD_HHMMSS.db`

Examples:
- `zones_backup_20231215_143022.db` - Created on Dec 15, 2023 at 14:30:22
- `zones_backup_20231215_143022.db.gz` - Compressed version
- `zones_backup_before_restore_20231215_144500.db` - Safety backup before restore

## Docker Integration

The scripts are designed to work seamlessly with the Docker Compose setup:

- **Container Detection**: Automatically detects if the specified container is running
- **Volume Mapping**: Leverages the existing volume mapping (`./data:/opt/eq_rng/data`)
- **SQLite Commands**: Attempts to use SQLite tools within the container when available
- **Fallback Methods**: Uses `docker cp` as fallback for file operations

### Docker Container Requirements

The scripts expect:
- Container name: `eq_rng-app-1` (default, configurable)
- Database path in container: `/opt/eq_rng/data/zones.db`
- Volume mapping for data directory

## Best Practices

### Backup Frequency

- **Development**: Create backups before major changes or migrations
- **Production**: Set up automated daily backups with retention policy
- **Before Updates**: Always backup before application updates

### Retention Policy

Use the `--keep` option to maintain a reasonable number of backups:
```bash
# Keep last 7 daily backups
./utils/backup_database.sh --keep 7

# Keep last 30 backups with compression
./utils/backup_database.sh --compress --keep 30
```

### Storage Considerations

- **Compression**: Use `--compress` for long-term storage
- **Location**: Consider storing backups on separate storage volumes
- **Monitoring**: Monitor backup directory size and set up alerts

### Security

- **Permissions**: Ensure backup files have appropriate permissions
- **Access Control**: Restrict access to backup directory
- **Encryption**: Consider encrypting backups for sensitive data

## Automation

### Cron Job Example

Add to crontab for automated daily backups:

```bash
# Daily backup at 2 AM with compression, keep 30 days
0 2 * * * /path/to/eq_rng.com/utils/backup_database.sh --compress --keep 30

# Docker backup every 6 hours, keep 28 backups (7 days)
0 */6 * * * /path/to/eq_rng.com/utils/backup_database.sh --docker --compress --keep 28
```

### Systemd Timer

Create a systemd service and timer for more advanced scheduling:

```ini
# /etc/systemd/system/eq-rng-backup.service
[Unit]
Description=EQ RNG Database Backup
After=network.target

[Service]
Type=oneshot
ExecStart=/path/to/eq_rng.com/utils/backup_database.sh --docker --compress --keep 30
User=your-user
WorkingDirectory=/path/to/eq_rng.com
```

```ini
# /etc/systemd/system/eq-rng-backup.timer
[Unit]
Description=Run EQ RNG Database Backup Daily
Requires=eq-rng-backup.service

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
```

## Troubleshooting

### Common Issues

#### "sqlite3 command not found"
- **Solution**: Install SQLite3 or the script will fallback to file copy
- **Note**: File copy may be inconsistent if database is actively being written to

#### "Docker container not running"
- **Solution**: Start the container with `docker-compose up -d`
- **Check**: Verify container name matches the expected name

#### "Permission denied"
- **Solution**: Ensure scripts are executable: `chmod +x utils/*.sh`
- **Check**: Verify user has read/write access to data and backup directories

#### "Backup integrity check failed"
- **Cause**: Corrupted backup or database was being written during backup
- **Solution**: Try creating a new backup when database is not actively being used

### Debugging

Enable verbose output by modifying the scripts to add:
```bash
set -x  # Enable debug mode
```

Or run with bash debugging:
```bash
bash -x utils/backup_database.sh
```

## Dependencies

### Required
- `bash` - Shell interpreter
- `find` - File finding utility
- `date` - Date/time utility
- `mkdir` - Directory creation
- `cp` - File copying

### Optional
- `sqlite3` - For consistent database backups and integrity checking
- `docker` - For Docker container operations
- `gzip` - For backup compression
- `gunzip` - For backup decompression

### Installation on Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install sqlite3 gzip
```

### Installation on macOS
```bash
brew install sqlite3 gzip
```

## Support

If you encounter issues:

1. **Check the logs**: Scripts provide detailed colored output
2. **Verify dependencies**: Ensure all required tools are installed
3. **Test manually**: Try SQLite commands manually to isolate issues
4. **Check permissions**: Verify file and directory permissions
5. **Container status**: For Docker mode, ensure container is running

For additional help, refer to the main project documentation or create an issue in the project repository.