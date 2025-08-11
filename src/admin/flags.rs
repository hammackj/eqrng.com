// Flag types-related admin functionality
// This file contains flag type management features for the admin interface

#[cfg(feature = "admin")]
use crate::security::{escape_html, sanitize_user_input};

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

#[cfg(feature = "admin")]
pub async fn list_flag_types(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get all flag types
    let flag_type_rows = sqlx::query("SELECT * FROM flag_types ORDER BY name")
        .fetch_all(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Manage Flag Types - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-section {{ background: #f8f9fa; padding: 20px; margin-bottom: 20px; border-radius: 5px; }}
        .form-group {{ margin-bottom: 15px; display: inline-block; margin-right: 15px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input, select {{ padding: 8px; border: 1px solid #ddd; border-radius: 4px; }}
        .btn {{ background: #007bff; color: white; padding: 8px 15px; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn:hover {{ background: #0056b3; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-danger:hover {{ background: #c82333; }}
        .btn-small {{ padding: 4px 8px; font-size: 0.8em; }}
        table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; }}
        th, td {{ padding: 8px; border: 1px solid #ddd; text-align: left; }}
        th {{ background: #f8f9fa; border-bottom: 2px solid #dee2e6; }}
        .color-preview {{ display: inline-block; width: 20px; height: 20px; border-radius: 3px; margin-right: 10px; vertical-align: middle; }}
        .flag-type-preview {{ display: inline-block; padding: 4px 8px; border-radius: 12px; color: white; font-size: 0.8em; font-weight: bold; }}

        /* Color classes for flag type previews */
        .bg-blue-500 {{ background-color: #3b82f6; }}
        .bg-green-500 {{ background-color: #22c55e; }}
        .bg-yellow-500 {{ background-color: #eab308; color: #000; }}
        .bg-red-500 {{ background-color: #ef4444; }}
        .bg-purple-500 {{ background-color: #a855f7; }}
        .bg-indigo-500 {{ background-color: #6366f1; }}
        .bg-pink-500 {{ background-color: #ec4899; }}
        .bg-gray-500 {{ background-color: #6b7280; }}
        .bg-orange-500 {{ background-color: #f97316; }}
        .bg-teal-500 {{ background-color: #14b8a6; }}
        .bg-cyan-500 {{ background-color: #06b6d4; }}
        .bg-emerald-500 {{ background-color: #10b981; }}
        .bg-lime-500 {{ background-color: #84cc16; color: #000; }}
        .bg-amber-500 {{ background-color: #f59e0b; color: #000; }}
        .bg-rose-500 {{ background-color: #f43f5e; }}
        .bg-fuchsia-500 {{ background-color: #d946ef; }}
        .bg-violet-500 {{ background-color: #8b5cf6; }}
        .bg-slate-500 {{ background-color: #64748b; }}
        .bg-zinc-500 {{ background-color: #71717a; }}
        .bg-neutral-500 {{ background-color: #737373; }}
        .bg-stone-500 {{ background-color: #78716c; }}
        .bg-sky-500 {{ background-color: #0ea5e9; }}
    </style>
    <script>
        function deleteFlagType(id, name) {{
            if (confirm('Are you sure you want to delete "' + name + '"? This will affect all existing flags of this type.')) {{
                document.getElementById('delete-form-' + id).submit();
            }}
        }}

        function updatePreview() {{
            const displayName = document.getElementById('display_name').value || 'Preview';
            const colorClass = document.getElementById('color_class').value;
            const preview = document.getElementById('preview');

            preview.textContent = displayName;
            preview.className = 'flag-type-preview ' + colorClass;
        }}

        document.addEventListener('DOMContentLoaded', function() {{
            document.getElementById('display_name').addEventListener('input', updatePreview);
            document.getElementById('color_class').addEventListener('change', updatePreview);
            updatePreview();
        }});
    </script>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/instances">Manage Instances</a>
        <a href="/admin/ratings">Manage Ratings</a>
        <a href="/admin/links">Manage Links</a>
        <a href="/admin/note-types">Note Types</a>
        <a href="/admin/flag-types">Flag Types</a>
    </div>

    <h1>Manage Flag Types</h1>

    <div class="form-section">
        <h2>Add New Flag Type</h2>
        <form method="post" action="/admin/flag-types" style="display: flex; align-items: end; gap: 15px; flex-wrap: wrap;">
            <div class="form-group">
                <label for="name">Internal Name:</label>
                <input type="text" id="name" name="name" placeholder="e.g., requires_key" required />
            </div>

            <div class="form-group">
                <label for="display_name">Display Name:</label>
                <input type="text" id="display_name" name="display_name" placeholder="e.g., Requires Key" required />
            </div>

            <div class="form-group">
                <label for="color_class">Color Class:</label>
                <select id="color_class" name="color_class" required>
                    <option value="bg-blue-500">Blue</option>
                    <option value="bg-green-500">Green</option>
                    <option value="bg-yellow-500">Yellow</option>
                    <option value="bg-red-500">Red</option>
                    <option value="bg-purple-500">Purple</option>
                    <option value="bg-indigo-500">Indigo</option>
                    <option value="bg-pink-500">Pink</option>
                    <option value="bg-orange-500">Orange</option>
                    <option value="bg-teal-500">Teal</option>
                    <option value="bg-cyan-500">Cyan</option>
                    <option value="bg-emerald-500">Emerald</option>
                    <option value="bg-lime-500">Lime</option>
                    <option value="bg-amber-500">Amber</option>
                    <option value="bg-rose-500">Rose</option>
                    <option value="bg-fuchsia-500">Fuchsia</option>
                    <option value="bg-violet-500">Violet</option>
                    <option value="bg-sky-500">Sky</option>
                    <option value="bg-slate-500">Slate</option>
                    <option value="bg-zinc-500">Zinc</option>
                    <option value="bg-neutral-500">Neutral</option>
                    <option value="bg-stone-500">Stone</option>
                    <option value="bg-gray-500">Gray</option>
                </select>
            </div>

            <div class="form-group">
                <label>Preview:</label>
                <div>
                    <span id="preview" class="flag-type-preview bg-blue-500">Preview</span>
                </div>
            </div>

            <div class="form-group">
                <input type="checkbox" id="filterable" name="filterable" value="true" checked />
                <label for="filterable" style="display: inline; margin-left: 5px;">Available as filter on main page</label>
            </div>

            <div class="form-group">
                <button type="submit" class="btn">Add Flag Type</button>
            </div>
        </form>
    </div>

    <h2>Existing Flag Types</h2>

    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>Display Name</th>
                <th>Color</th>
                <th>Preview</th>
                <th>Filterable</th>
                <th>Created</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
"#
    );

    // Add flag type rows
    for row in flag_type_rows {
        let id: i64 = row.get("id");
        let name: String = row.get("name");
        let display_name: String = row.get("display_name");
        let color_class: String = row.get("color_class");
        let filterable: bool = row.get("filterable");
        let created_at: String = row.get("created_at");

        // Extract color name from class
        let color_name = color_class
            .replace("bg-", "")
            .replace("-500", "")
            .split('-')
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        html.push_str(&format!(
            r#"
            <tr>
                <td><code>{}</code></td>
                <td>{}</td>
                <td>{}</td>
                <td><span class="flag-type-preview {}">{}</span></td>
                <td>{}</td>
                <td>{}</td>
                <td>
                    <a href="/admin/flag-types/{}" class="btn btn-small">Edit</a>
                    <button onclick="deleteFlagType({}, '{}')" class="btn btn-danger btn-small">Delete</button>
                    <form id="delete-form-{}" method="post" action="/admin/flag-types/{}/delete" style="display: none;">
                        <input type="hidden" name="_method" value="DELETE" />
                    </form>
                </td>
            </tr>
            "#,
            name,
            display_name,
            color_name,
            color_class,
            display_name,
            if filterable { "✓ Yes" } else { "✗ No" },
            created_at.split('T').next().unwrap_or(&created_at),
            id,
            id,
            name.replace("'", "\\'"),
            id,
            id
        ));
    }

    html.push_str(
        r#"
        </tbody>
    </table>

    <div style="margin-top: 30px; padding: 20px; background: #e9ecef; border-radius: 5px;">
        <h3>Flag Type Usage</h3>
        <p>Flag types are used to categorize flags attached to zones. They help mark special characteristics or requirements:</p>
        <ul>
            <li><strong>Internal Name:</strong> Used in the database and code (lowercase, underscores)</li>
            <li><strong>Display Name:</strong> What users see in the interface</li>
            <li><strong>Color Class:</strong> Visual styling for the flag type badge</li>
            <li><strong>Filterable:</strong> Controls whether this flag appears as a filter option on the main zone generator page</li>
        </ul>
        <p><strong>Examples:</strong> "requires_key" → "Requires Key", "raid_zone" → "Raid Zone", "instanced" → "Instanced"</p>
        <p><strong>Filterable Usage:</strong> Checked flags will appear as filter options on the main page. Unchecked flags are informational only and will only appear as badges on zones. This allows you to create detailed administrative flags without cluttering the user interface.</p>
        <p><strong>Warning:</strong> Deleting a flag type will affect all existing flags of that type. Use with caution.</p>
    </div>
</body>
</html>
"#,
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn create_flag_type(
    State(state): State<AppState>,
    Form(form): Form<FlagTypeForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    let filterable = form.filterable.is_some();

    // Insert the new flag type
    sqlx::query(
        r#"
        INSERT INTO flag_types (name, display_name, color_class, filterable)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&form.name)
    .bind(&form.display_name)
    .bind(&form.color_class)
    .bind(filterable)
    .execute(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/admin/flag-types"))
}

#[cfg(feature = "admin")]
pub async fn edit_flag_type_form(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Html<String>, StatusCode> {
    let pool = &state.zone_state.pool;

    // Get the flag type
    let flag_type_row = sqlx::query("SELECT * FROM flag_types WHERE id = ?")
        .bind(id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let name: String = flag_type_row.get("name");
    let display_name: String = flag_type_row.get("display_name");
    let color_class: String = flag_type_row.get("color_class");
    let filterable: bool = flag_type_row.get("filterable");

    let colors = [
        ("bg-blue-500", "Blue"),
        ("bg-green-500", "Green"),
        ("bg-yellow-500", "Yellow"),
        ("bg-red-500", "Red"),
        ("bg-purple-500", "Purple"),
        ("bg-indigo-500", "Indigo"),
        ("bg-pink-500", "Pink"),
        ("bg-orange-500", "Orange"),
        ("bg-teal-500", "Teal"),
        ("bg-cyan-500", "Cyan"),
        ("bg-emerald-500", "Emerald"),
        ("bg-lime-500", "Lime"),
        ("bg-amber-500", "Amber"),
        ("bg-rose-500", "Rose"),
        ("bg-fuchsia-500", "Fuchsia"),
        ("bg-violet-500", "Violet"),
        ("bg-sky-500", "Sky"),
        ("bg-slate-500", "Slate"),
        ("bg-zinc-500", "Zinc"),
        ("bg-neutral-500", "Neutral"),
        ("bg-stone-500", "Stone"),
        ("bg-gray-500", "Gray"),
    ];
    let color_options = colors
        .iter()
        .map(|(class, name_str)| {
            if *class == color_class {
                format!(
                    r#"<option value="{}" selected>{}</option>"#,
                    class, name_str
                )
            } else {
                format!(r#"<option value="{}">{}</option>"#, class, name_str)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Edit Flag Type - EQ RNG Admin</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .nav {{ background: #f5f5f5; padding: 15px; margin-bottom: 20px; border-radius: 5px; }}
        .nav a {{ margin-right: 15px; text-decoration: none; color: #333; font-weight: bold; }}
        .nav a:hover {{ color: #007bff; }}
        .form-group {{ margin-bottom: 20px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
        input, select {{ width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }}
        .btn {{ background: #007bff; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; border: none; cursor: pointer; }}
        .btn:hover {{ background: #0056b3; }}
        .btn-secondary {{ background: #6c757d; }}
        .btn-secondary:hover {{ background: #545b62; }}
        .btn-danger {{ background: #dc3545; }}
        .btn-danger:hover {{ background: #c82333; }}
        .flag-type-preview {{ display: inline-block; padding: 4px 8px; border-radius: 12px; color: white; font-size: 0.8em; font-weight: bold; }}
        .preview-section {{ margin-top: 10px; }}

        /* Color classes for flag type previews */
        .bg-blue-500 {{ background-color: #3b82f6; }}
        .bg-green-500 {{ background-color: #22c55e; }}
        .bg-yellow-500 {{ background-color: #eab308; color: #000; }}
        .bg-red-500 {{ background-color: #ef4444; }}
        .bg-purple-500 {{ background-color: #a855f7; }}
        .bg-indigo-500 {{ background-color: #6366f1; }}
        .bg-pink-500 {{ background-color: #ec4899; }}
        .bg-gray-500 {{ background-color: #6b7280; }}
        .bg-orange-500 {{ background-color: #f97316; }}
        .bg-teal-500 {{ background-color: #14b8a6; }}
        .bg-cyan-500 {{ background-color: #06b6d4; }}
        .bg-emerald-500 {{ background-color: #10b981; }}
        .bg-lime-500 {{ background-color: #84cc16; color: #000; }}
        .bg-amber-500 {{ background-color: #f59e0b; color: #000; }}
        .bg-rose-500 {{ background-color: #f43f5e; }}
        .bg-fuchsia-500 {{ background-color: #d946ef; }}
        .bg-violet-500 {{ background-color: #8b5cf6; }}
        .bg-slate-500 {{ background-color: #64748b; }}
        .bg-zinc-500 {{ background-color: #71717a; }}
        .bg-neutral-500 {{ background-color: #737373; }}
        .bg-stone-500 {{ background-color: #78716c; }}
        .bg-sky-500 {{ background-color: #0ea5e9; }}
    </style>
    <script>
        function deleteFlagType() {{
            if (confirm('Are you sure you want to delete this flag type?')) {{
                document.getElementById('delete-form').submit();
            }}
        }}

        function updatePreview() {{
            const displayName = document.getElementById('display_name').value || 'Preview';
            const colorClass = document.getElementById('color_class').value;
            const preview = document.getElementById('preview');

            preview.textContent = displayName;
            preview.className = 'flag-type-preview ' + colorClass;
        }}

        document.addEventListener('DOMContentLoaded', function() {{
            document.getElementById('display_name').addEventListener('input', updatePreview);
            document.getElementById('color_class').addEventListener('change', updatePreview);
            updatePreview();
        }});
    </script>
</head>
<body>
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/zones">Manage Zones</a>
        <a href="/admin/instances">Manage Instances</a>
        <a href="/admin/ratings">Manage Ratings</a>
        <a href="/admin/links">Manage Links</a>
        <a href="/admin/note-types">Note Types</a>
        <a href="/admin/flag-types">Flag Types</a>
    </div>

    <h1>Edit Flag Type</h1>

    <form method="post" action="/admin/flag-types/{}">
        <input type="hidden" name="_method" value="PUT" />

        <div class="form-group">
            <label for="name">Internal Name:</label>
            <input type="text" id="name" name="name" value="{}" required />
        </div>

        <div class="form-group">
            <label for="display_name">Display Name:</label>
            <input type="text" id="display_name" name="display_name" value="{}" required />
        </div>

        <div class="form-group">
            <label for="color_class">Color Class:</label>
            <select id="color_class" name="color_class" required>
                {}
            </select>
        </div>

        <div class="form-group">
            <label>Preview:</label>
            <div class="preview-section">
                <span id="preview" class="flag-type-preview {}">Preview</span>
            </div>
        </div>

        <div class="form-group">
            <input type="checkbox" id="filterable" name="filterable" value="true" {} />
            <label for="filterable" style="display: inline; margin-left: 5px;">Available as filter on main page</label>
        </div>

        <div class="form-group">
            <button type="submit" class="btn">Update Flag Type</button>
            <a href="/admin/flag-types" class="btn btn-secondary">Cancel</a>
            <button type="button" onclick="deleteFlagType()" class="btn btn-danger">Delete</button>
        </div>
    </form>

    <form id="delete-form" method="post" action="/admin/flag-types/{}/delete" style="display: none;">
        <input type="hidden" name="_method" value="DELETE" />
    </form>
</body>
</html>
"#,
        id,
        escape_html(&name),
        escape_html(&display_name),
        color_options,
        color_class,
        if filterable { "checked" } else { "" },
        id
    );

    Ok(Html(html))
}

#[cfg(feature = "admin")]
pub async fn update_flag_type(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<FlagTypeForm>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    let filterable = form.filterable.is_some();

    // Update the flag type
    sqlx::query(
        r#"
        UPDATE flag_types
        SET name = ?, display_name = ?, color_class = ?, filterable = ?
        WHERE id = ?
        "#,
    )
    .bind(&form.name)
    .bind(&form.display_name)
    .bind(&form.color_class)
    .bind(filterable)
    .bind(id)
    .execute(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/admin/flag-types"))
}

#[cfg(feature = "admin")]
pub async fn delete_flag_type(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Redirect, StatusCode> {
    let pool = &state.zone_state.pool;

    // Delete the flag type (this will cascade to related flags due to foreign key constraints)
    sqlx::query("DELETE FROM flag_types WHERE id = ?")
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/admin/flag-types"))
}
