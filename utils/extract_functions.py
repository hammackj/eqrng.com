#!/usr/bin/env python3
"""
Helper script to extract function implementations from the original admin.rs backup
and organize them into the new modular structure.

This script helps automate the process of moving functions from the monolithic
admin.rs file into the appropriate modules.
"""

import re
import os
from typing import Dict, List, Tuple

# Define which functions belong to which modules
FUNCTION_MAPPING = {
    'zones.rs': [
        'list_zones',
        'new_zone_form',
        'edit_zone_form',
        'create_zone',
        'update_zone',
        'delete_zone',
        'handle_zone_update_or_delete',
        'move_zone_to_instances',
        'zone_ratings',
        'zone_notes',
        'create_zone_note',
        'delete_zone_note',
        'create_zone_flag',
        'delete_zone_flag',
        'delete_zone_flag_simple',
        'get_zone_form_header',
        'get_zone_form_body',
        'get_zone_form_body_with_notes',
        'get_zone_form_body_with_notes_and_flags'
    ],
    'instances.rs': [
        'list_instances',
        'edit_instance_form',
        'handle_instance_update_or_delete',
        'update_instance',
        'delete_instance',
        'instance_notes',
        'create_instance_note',
        'delete_instance_note',
        'get_instance_form_header',
        'get_instance_form_body'
    ],
    'notes.rs': [
        'list_note_types',
        'create_note_type',
        'delete_note_type'
    ],
    'flags.rs': [
        'list_flag_types',
        'create_flag_type',
        'edit_flag_type_form',
        'update_flag_type',
        'delete_flag_type'
    ],
    'ratings.rs': [
        'list_all_ratings',
        'delete_rating_admin',
        'handle_rating_delete'
    ],
    'links.rs': [
        'list_links',
        'new_link_form',
        'edit_link_form',
        'create_link_admin',
        'handle_link_update_or_delete',
        'update_link_admin',
        'delete_link_admin'
    ]
}

def extract_function(content: str, function_name: str) -> str:
    """Extract a function definition from the source content."""
    # Pattern to match function definitions with attributes and body
    pattern = rf'((?:#\[cfg\(feature = "admin"\)\]\s*)?(?:pub\s+)?async\s+fn\s+{function_name}\s*\([^{{]*\)\s*(?:->\s*[^{{]*?)?\s*\{{)'

    match = re.search(pattern, content, re.MULTILINE | re.DOTALL)
    if not match:
        # Try without async
        pattern = rf'((?:#\[cfg\(feature = "admin"\)\]\s*)?(?:pub\s+)?fn\s+{function_name}\s*\([^{{]*\)\s*(?:->\s*[^{{]*?)?\s*\{{)'
        match = re.search(pattern, content, re.MULTILINE | re.DOTALL)

    if not match:
        return f"// Function {function_name} not found\n"

    start_pos = match.start()
    brace_count = 0
    pos = match.end() - 1  # Start at the opening brace

    # Find the matching closing brace
    while pos < len(content):
        if content[pos] == '{':
            brace_count += 1
        elif content[pos] == '}':
            brace_count -= 1
            if brace_count == 0:
                break
        pos += 1

    if brace_count != 0:
        return f"// Could not find complete function body for {function_name}\n"

    function_code = content[start_pos:pos + 1]
    return function_code + "\n\n"

def generate_module_content(backup_content: str, module_name: str, functions: List[str]) -> str:
    """Generate the complete content for a module."""

    # Module header based on the module type
    if module_name == 'zones.rs':
        header = '''// Zone management functionality
#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};

#[cfg(feature = "admin")]
use sqlx::Row;
#[cfg(feature = "admin")]
use urlencoding;

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::types::*;
#[cfg(feature = "admin")]
use crate::admin::dashboard::{generate_sortable_header, get_distinct_zone_types, get_distinct_expansions, generate_expansion_options, generate_zone_type_options};

'''
    elif module_name == 'instances.rs':
        header = '''// Instance management functionality
#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};

#[cfg(feature = "admin")]
use sqlx::Row;
#[cfg(feature = "admin")]
use urlencoding;

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::types::*;
#[cfg(feature = "admin")]
use crate::admin::dashboard::generate_sortable_header;

'''
    elif module_name == 'notes.rs':
        header = '''// Note types management functionality
#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Redirect},
};

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::types::*;

'''
    elif module_name == 'flags.rs':
        header = '''// Flag types management functionality
#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Redirect},
};

#[cfg(feature = "admin")]
use sqlx::Row;

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::types::*;

'''
    elif module_name == 'ratings.rs':
        header = '''// Rating management functionality
#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};

#[cfg(feature = "admin")]
use std::collections::HashMap;
#[cfg(feature = "admin")]
use sqlx::Row;
#[cfg(feature = "admin")]
use urlencoding;

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::types::*;
#[cfg(feature = "admin")]
use crate::admin::dashboard::generate_sortable_header;

'''
    elif module_name == 'links.rs':
        header = '''// Link management functionality
#[cfg(feature = "admin")]
use axum::{
    Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};

#[cfg(feature = "admin")]
use sqlx::Row;
#[cfg(feature = "admin")]
use urlencoding;

#[cfg(feature = "admin")]
use crate::AppState;
#[cfg(feature = "admin")]
use crate::admin::types::*;
#[cfg(feature = "admin")]
use crate::admin::dashboard::generate_sortable_header;

'''
    else:
        header = f"// {module_name} functionality\n\n"

    content = header

    # Extract each function
    for func_name in functions:
        func_content = extract_function(backup_content, func_name)
        content += func_content

    return content

def main():
    """Main function to orchestrate the extraction process."""

    # Check if backup file exists
    backup_file = "admin_backup.rs"
    if not os.path.exists(backup_file):
        print(f"Error: {backup_file} not found!")
        print("Please create a backup of the original admin.rs file first.")
        return

    # Read the backup content
    with open(backup_file, 'r') as f:
        backup_content = f.read()

    # Create output directory if it doesn't exist
    output_dir = "src/admin_extracted"
    os.makedirs(output_dir, exist_ok=True)

    # Process each module
    for module_name, functions in FUNCTION_MAPPING.items():
        print(f"Processing {module_name}...")

        module_content = generate_module_content(backup_content, module_name, functions)

        output_path = os.path.join(output_dir, module_name)
        with open(output_path, 'w') as f:
            f.write(module_content)

        print(f"  -> {output_path} created with {len(functions)} functions")

    print("\nExtraction complete!")
    print(f"Check the {output_dir} directory for the extracted modules.")
    print("\nNext steps:")
    print("1. Review the extracted functions")
    print("2. Replace placeholder functions in src/admin/ with the extracted ones")
    print("3. Test each module individually")
    print("4. Remove any duplicate imports or unused code")

if __name__ == "__main__":
    main()
