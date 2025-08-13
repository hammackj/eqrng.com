#!/usr/bin/env bash
#
# resize_assets.sh
#
# Resize and optimize class/race images (backup originals).
#
# - Backs up original images into "backups/images/YYYYMMDD_HHMMSS/"
# - Resizes raster images to the target display size (default: 192px)
#   using ImageMagick (convert/mogrify). Resizing is only applied when
#   the source is larger than the target (ImageMagick ">" operator).
# - Optimizes PNGs via pngquant (if available) and JPEGs via jpegoptim (if available).
# - Processes both:
#     eq_rng.com/public/assets/images/{classes,races}
#     eq_rng.com/frontend/public/assets/images/{classes,races}
#
# Usage:
#   ./utils/resize_assets.sh [--size N] [--backup-dir DIR] [--dry-run] [--skip-opt]
#
# Examples:
#   # shrink to 192 pixels (default) with backup, then optimize
#   ./utils/resize_assets.sh
#
#   # dry-run (no changes)
#   ./utils/resize_assets.sh --dry-run
#
#   # change size to 256px and skip optimization
#   ./utils/resize_assets.sh --size 256 --skip-opt
#
set -euo pipefail

# Defaults
TARGET_SIZE=192
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
BACKUP_PARENT_DIR="$REPO_ROOT/backups/images"
DRY_RUN=false
SKIP_OPT=false
PARALLEL_JOBS=4

# Directories to process (relative to repo root)
DIRS_TO_PROCESS=(
  "public/assets/images/classes"
  "public/assets/images/races"
  "frontend/public/assets/images/classes"
  "frontend/public/assets/images/races"
)

# Tools
CMD_CONVERT=""
CMD_MOGRIFY=""
CMD_PNGQUANT=""
CMD_JPEGOPTIM=""
CMD_PARALLEL=""

print_usage() {
  cat <<EOF
resize_assets.sh - Resize and optimize class/race images

Usage:
  $0 [--size N] [--backup-dir DIR] [--dry-run] [--skip-opt] [--jobs N]

Options:
  --size N         Target max dimension for images (default: ${TARGET_SIZE})
  --backup-dir DIR Backup parent directory (default: ${BACKUP_PARENT_DIR})
  --dry-run        Do not modify files; only print actions
  --skip-opt       Skip pngquant/jpegoptim optimization step
  --jobs N         Parallel jobs for processing (default: ${PARALLEL_JOBS})
  -h, --help       Show this help and exit

Notes:
  - The script only resizes raster images (PNG/JPEG). SVG files are skipped.
  - Originals are backed up before any in-place replacement.
  - Requires ImageMagick (convert or mogrify). Optional optimizers: pngquant and jpegoptim.
EOF
}

# Parse args
while (( "$#" )); do
  case "$1" in
    --size)
      TARGET_SIZE="$2"; shift 2;;
    --backup-dir)
      BACKUP_PARENT_DIR="$2"; shift 2;;
    --dry-run)
      DRY_RUN=true; shift;;
    --skip-opt)
      SKIP_OPT=true; shift;;
    --jobs)
      PARALLEL_JOBS="$2"; shift 2;;
    -h|--help)
      print_usage; exit 0;;
    *)
      echo "Unknown argument: $1"; print_usage; exit 2;;
  esac
done

# Detect commands
if command -v mogrify >/dev/null 2>&1; then
  CMD_MOGRIFY="mogrify"
fi

if command -v convert >/dev/null 2>&1; then
  CMD_CONVERT="convert"
fi

if command -v pngquant >/dev/null 2>&1; then
  CMD_PNGQUANT="pngquant"
fi

if command -v jpegoptim >/dev/null 2>&1; then
  CMD_JPEGOPTIM="jpegoptim"
fi

if command -v parallel >/dev/null 2>&1; then
  CMD_PARALLEL="parallel"
fi

if [[ -z "$CMD_MOGRIFY" && -z "$CMD_CONVERT" ]]; then
  echo "Error: ImageMagick 'convert' or 'mogrify' is required but not found in PATH."
  echo "Install ImageMagick (apt: imagemagick) and try again."
  exit 1
fi

if [[ "$DRY_RUN" = true ]]; then
  echo "[DRY-RUN] Running in dry-run mode; no files will be modified."
fi

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
BACKUP_DIR="${BACKUP_PARENT_DIR%/}/$TIMESTAMP"

# Ensure backup dir if not dry-run
if [[ "$DRY_RUN" = false ]]; then
  mkdir -p "$BACKUP_DIR"
  echo "Backups will be stored under: $BACKUP_DIR"
else
  echo "Would create backups under: $BACKUP_DIR"
fi

# Helper: absolute path helper
abs_path() {
  local p="$1"
  if [[ -d "$p" || -f "$p" ]]; then
    (cd "$(dirname "$p")" && printf "%s/%s" "$(pwd -P)" "$(basename "$p")")
  else
    echo "$REPO_ROOT/$p"
  fi
}

# Process single file
# Arguments: full_path_to_file
process_file() {
  local f="$1"
  # guard: must exist
  if [[ ! -f "$f" ]]; then
    echo "[SKIP] Not a file: $f"
    return 0
  fi

  local rel="${f#$REPO_ROOT/}"
  local ext="${f##*.}"
  # lower-case extension using tr (portable)
  local lc_ext
  lc_ext="$(printf '%s' "$ext" | tr '[:upper:]' '[:lower:]')"

  # skip SVG
  if [[ "$lc_ext" = "svg" ]]; then
    echo "[SKIP] SVG (vector) skipped: $rel"
    return 0
  fi

  # Determine backup path
  local backup_path="$BACKUP_DIR/$rel"
  local backup_dir
  backup_dir="$(dirname "$backup_path")"

  # Make backup dir and copy
  if [[ "$DRY_RUN" = false ]]; then
    mkdir -p "$backup_dir"
    if [[ ! -f "$backup_path" ]]; then
      cp -p "$f" "$backup_path"
    fi
  else
    echo "[DRY-RUN] Would backup $rel -> $backup_path"
  fi

  # Create portable temp file and append extension
  local tmp
  # Try GNU-style mktemp (no args), fall back to BSD-style (-t), else use /tmp with PID
  if tmp="$(mktemp 2>/dev/null)"; then
    :
  elif tmp="$(mktemp -t resize_assets 2>/dev/null)"; then
    :
  else
    tmp="/tmp/resize_assets.$$"
    touch "$tmp"
  fi
  # Ensure the temporary filename has the original extension so tools behave consistently
  tmp="${tmp}.${lc_ext}"

  # Resize only if larger than target: ImageMagick '>' operator
  if [[ -n "$CMD_MOGRIFY" && -z "$CMD_CONVERT" ]]; then
    cp "$f" "$tmp"
    if [[ "$DRY_RUN" = false ]]; then
      mogrify -resize "${TARGET_SIZE}x${TARGET_SIZE}>" "$tmp"
    else
      echo "[DRY-RUN] Would run: mogrify -resize ${TARGET_SIZE}x${TARGET_SIZE}\\> $tmp"
    fi
  else
    if [[ "$DRY_RUN" = false ]]; then
      convert "$f" -resize "${TARGET_SIZE}x${TARGET_SIZE}>" -strip "$tmp"
    else
      echo "[DRY-RUN] Would run: convert $f -resize ${TARGET_SIZE}x${TARGET_SIZE}\\> -strip $tmp"
    fi
  fi

  # Optimize and move into place
  if [[ "$DRY_RUN" = false ]]; then
    case "$lc_ext" in
      png)
        if [[ "$SKIP_OPT" = false && -n "$CMD_PNGQUANT" ]]; then
          # pngquant writes to a file; use a temp output then move
          # pngquant supports reading/writing same file, but we'll use tmp as input and output
          "$CMD_PNGQUANT" --quality=65-85 --speed 1 --force --output "$tmp" -- "$tmp" >/dev/null 2>&1 || true
        fi
        mv -f "$tmp" "$f"
        ;;
      jpg|jpeg)
        mv -f "$tmp" "$f"
        if [[ "$SKIP_OPT" = false && -n "$CMD_JPEGOPTIM" ]]; then
          "$CMD_JPEGOPTIM" --max=85 --strip-all --all-progressive "$f" >/dev/null 2>&1 || true
        fi
        ;;
      *)
        mv -f "$tmp" "$f"
        ;;
    esac
    echo "[OK] Resized and optimized: $rel"
  else
    rm -f "$tmp" || true
    echo "[DRY-RUN] Would resize (and optionally optimize): $rel"
  fi
}

# Gather files
echo "Scanning directories and resizing images to max dimension ${TARGET_SIZE}px..."
files_to_process=()
for d in "${DIRS_TO_PROCESS[@]}"; do
  full="$REPO_ROOT/$d"
  if [[ -d "$full" ]]; then
    # find png/jpg/jpeg - print0 safe list
    while IFS= read -r -d '' file; do
      # skip svg by extension (portable)
      ext="${file##*.}"
      ext_lc="$(printf '%s' "$ext" | tr '[:upper:]' '[:lower:]')"
      if [[ "$ext_lc" = "svg" ]]; then
        continue
      fi
      files_to_process+=("$file")
    done < <(find "$full" -type f \( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' \) -print0)
  else
    echo "Notice: directory not found, skipping: $full"
  fi
done

if [[ "${#files_to_process[@]}" -eq 0 ]]; then
  echo "No raster images found to process. Exiting."
  exit 0
fi

echo "Found ${#files_to_process[@]} images to process."

# Export necessary things for subshells
export REPO_ROOT TARGET_SIZE BACKUP_DIR DRY_RUN SKIP_OPT CMD_PNGQUANT CMD_JPEGOPTIM CMD_MOGRIFY CMD_CONVERT
export -f process_file abs_path

# Process files in parallel using GNU parallel if present, else xargs
if [[ -n "$CMD_PARALLEL" ]]; then
  printf "%s\0" "${files_to_process[@]}" | parallel -0 -P "$PARALLEL_JOBS" --will-cite bash -c 'process_file "$@"' _ {}
else
  # xargs: use bash -c with '_' as $0 and the file as $1
  printf "%s\0" "${files_to_process[@]}" | xargs -0 -n1 -P "$PARALLEL_JOBS" bash -c 'process_file "$1"' _
fi

echo "Done. Backups (if not dry-run) are stored under: ${BACKUP_DIR}"
echo "Recommendation: verify images visually and commit the resized assets if OK."

exit 0
