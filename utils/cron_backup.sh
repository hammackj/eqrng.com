#!/bin/bash

# Cron Backup Script for eq_rng.com Database
# This script is designed to be run via cron for automated backups

# Exit on any error
set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BACKUP_SCRIPT="$SCRIPT_DIR/backup_database.sh"
LOG_FILE="$PROJECT_DIR/backups/backup.log"

# Backup settings
KEEP_BACKUPS=30          # Keep 30 most recent backups
USE_COMPRESSION=true     # Compress backups to save space
USE_DOCKER=true          # Set to false for local filesystem backups

# Email notification settings (optional)
SEND_EMAIL_ON_ERROR=false
ADMIN_EMAIL="admin@example.com"

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Email notification function
send_error_email() {
    local error_msg="$1"

    if [[ "$SEND_EMAIL_ON_ERROR" == "true" ]] && command -v mail &> /dev/null; then
        echo "Database backup failed on $(hostname) at $(date)" | \
        mail -s "EQ RNG Database Backup Failed" "$ADMIN_EMAIL"
    fi
}

# Main backup function
run_backup() {
    local backup_args=""

    # Build backup command arguments
    if [[ "$USE_DOCKER" == "true" ]]; then
        backup_args="$backup_args --docker"
    fi

    if [[ "$USE_COMPRESSION" == "true" ]]; then
        backup_args="$backup_args --compress"
    fi

    if [[ -n "$KEEP_BACKUPS" ]]; then
        backup_args="$backup_args --keep $KEEP_BACKUPS"
    fi

    # Change to project directory
    cd "$PROJECT_DIR"

    # Run backup script
    log "Starting automated backup process..."
    log "Command: $BACKUP_SCRIPT $backup_args"

    if "$BACKUP_SCRIPT" $backup_args >> "$LOG_FILE" 2>&1; then
        log "Backup completed successfully"
        return 0
    else
        log "ERROR: Backup failed with exit code $?"
        return 1
    fi
}

# Create log directory if it doesn't exist
mkdir -p "$(dirname "$LOG_FILE")"

# Check if backup script exists
if [[ ! -f "$BACKUP_SCRIPT" ]]; then
    log "ERROR: Backup script not found: $BACKUP_SCRIPT"
    send_error_email "Backup script not found: $BACKUP_SCRIPT"
    exit 1
fi

# Check if backup script is executable
if [[ ! -x "$BACKUP_SCRIPT" ]]; then
    log "ERROR: Backup script is not executable: $BACKUP_SCRIPT"
    send_error_email "Backup script is not executable: $BACKUP_SCRIPT"
    exit 1
fi

# Run the backup
if run_backup; then
    log "Automated backup process completed successfully"
    exit 0
else
    log "Automated backup process failed"
    send_error_email "Database backup failed"
    exit 1
fi
