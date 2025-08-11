#[cfg(feature = "admin")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "admin")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub id: Option<i32>,
    pub name: String,
    pub level_ranges: String, // JSON string
    pub expansion: String,
    pub continent: String,
    pub zone_type: String,
    pub connections: String, // JSON string
    pub image_url: String,
    pub map_url: String,
    pub rating: i32,
    pub verified: bool,
    pub notes: Vec<crate::zones::ZoneNote>,
    pub flags: Vec<crate::zones::ZoneFlag>,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct ZoneForm {
    pub name: String,
    pub level_ranges: String,
    pub expansion: String,
    pub continent: String,
    pub zone_type: String,
    pub connections: String,
    pub image_url: String,
    pub map_url: String,
    pub rating: i32,
    pub verified: Option<String>, // HTML forms send "on" or nothing
    pub _method: Option<String>,  // For method override
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct InstanceForm {
    pub name: String,
    pub level_ranges: String,
    pub expansion: String,
    pub continent: String,
    pub zone_type: String,
    pub connections: String,
    pub image_url: String,
    pub map_url: String,
    pub rating: i32,
    pub hot_zone: Option<String>, // HTML forms send "on" or nothing
    pub verified: Option<String>, // HTML forms send "on" or nothing
    pub _method: Option<String>,  // For method override
}

#[cfg(feature = "admin")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Instance {
    pub id: Option<i32>,
    pub name: String,
    pub level_ranges: String, // JSON string
    pub expansion: String,
    pub continent: String,
    pub zone_type: String,
    pub connections: String, // JSON string
    pub image_url: String,
    pub map_url: String,
    pub rating: i32,
    pub hot_zone: bool,
    pub verified: bool,
    pub notes: Vec<crate::instances::InstanceNote>,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
    pub search: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub verified: Option<String>,
    pub zone_type: Option<String>,
    pub expansion: Option<String>,
    pub flags: Option<String>,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct ZoneNoteForm {
    pub note_type_id: i64,
    pub content: String,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct ZoneFlagForm {
    pub flag_type_id: i64,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct InstanceNoteForm {
    pub note_type_id: i64,
    pub content: String,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct NoteTypeForm {
    pub name: String,
    pub display_name: String,
    pub color_class: String,
}

#[cfg(feature = "admin")]
#[derive(Debug, Deserialize)]
pub struct FlagTypeForm {
    pub name: String,
    pub display_name: String,
    pub color_class: String,
    pub filterable: Option<String>, // HTML forms send "on" or nothing
}

#[cfg(feature = "admin")]
#[derive(Deserialize)]
pub struct LinkForm {
    pub name: String,
    pub url: String,
    pub category: String,
    pub description: Option<String>,
    pub _method: Option<String>,
}
