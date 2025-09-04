use std::collections::HashSet;
use url::Url;

/// Comprehensive HTML sanitization for preventing XSS attacks
pub struct HtmlSanitizer {
    allowed_tags: HashSet<&'static str>,
    #[allow(dead_code)]
    allowed_attributes: HashSet<&'static str>,
}

impl Default for HtmlSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlSanitizer {
    /// Create a new HTML sanitizer with safe defaults
    pub fn new() -> Self {
        let mut allowed_tags = HashSet::new();
        // Only allow very basic formatting tags
        allowed_tags.insert("b");
        allowed_tags.insert("i");
        allowed_tags.insert("em");
        allowed_tags.insert("strong");
        allowed_tags.insert("br");
        allowed_tags.insert("p");

        let allowed_attributes = HashSet::new();
        // No attributes allowed by default for maximum security

        Self {
            allowed_tags,
            allowed_attributes,
        }
    }

    /// Create a sanitizer that strips all HTML (safest option)
    pub fn strip_all() -> Self {
        Self {
            allowed_tags: HashSet::new(),
            allowed_attributes: HashSet::new(),
        }
    }

    /// Sanitize HTML input by removing dangerous content
    pub fn sanitize(&self, input: &str) -> String {
        if self.allowed_tags.is_empty() {
            // Strip all HTML
            self.strip_html_tags(input)
        } else {
            // Allow only specific tags
            self.sanitize_with_allowed_tags(input)
        }
    }

    /// Strip all HTML tags and return plain text
    fn strip_html_tags(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut in_tag = false;
        let mut in_dangerous_tag = false;
        let mut current_tag = String::new();
        let mut chars = input.chars();

        while let Some(ch) = chars.next() {
            match ch {
                '<' => {
                    in_tag = true;
                    current_tag.clear();
                }
                '>' if in_tag => {
                    in_tag = false;
                    // Check if this was a dangerous tag
                    let tag_lower = current_tag.to_lowercase();
                    if tag_lower.starts_with("script")
                        || tag_lower.starts_with("style")
                        || tag_lower.starts_with("iframe")
                    {
                        in_dangerous_tag = true;
                    } else if tag_lower.starts_with("/script")
                        || tag_lower.starts_with("/style")
                        || tag_lower.starts_with("/iframe")
                    {
                        in_dangerous_tag = false;
                    }
                    current_tag.clear();
                }
                ch if in_tag => {
                    current_tag.push(ch);
                }
                ch if !in_tag && !in_dangerous_tag => {
                    result.push(ch);
                }
                _ => {} // Skip characters inside dangerous tags
            }
        }

        result
    }

    /// Sanitize HTML while allowing specific tags
    fn sanitize_with_allowed_tags(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '<' {
                // Check if this is a valid tag
                let _tag_start = result.len();
                let mut tag_content = String::new();
                let mut found_end = false;

                // Collect the full tag
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '>' {
                        chars.next(); // consume the '>'
                        found_end = true;
                        break;
                    }
                    // Safe to unwrap since we just checked chars.peek() returned Some
                    tag_content.push(
                        chars
                            .next()
                            .expect("chars.next() should succeed after peek()"),
                    );
                }

                if found_end {
                    if let Some(sanitized_tag) = self.sanitize_tag(&tag_content) {
                        result.push('<');
                        result.push_str(&sanitized_tag);
                        result.push('>');
                    }
                    // If tag is not allowed, it's simply omitted
                } else {
                    // Malformed tag, escape the '<'
                    result.push_str("&lt;");
                    // Put back the characters we consumed
                    for ch in tag_content.chars() {
                        result.push_str(&self.escape_html_char(ch));
                    }
                }
            } else {
                result.push_str(&self.escape_html_char(ch));
            }
        }

        result
    }

    /// Sanitize a single tag and its attributes
    fn sanitize_tag(&self, tag_content: &str) -> Option<String> {
        let tag_content = tag_content.trim();
        if tag_content.is_empty() {
            return None;
        }

        // Check if it's a closing tag
        let is_closing = tag_content.starts_with('/');
        let tag_name = if is_closing {
            tag_content[1..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase()
        } else {
            tag_content
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase()
        };

        // Check if tag is allowed
        if !self.allowed_tags.contains(tag_name.as_str()) {
            return None;
        }

        if is_closing {
            // Closing tags don't have attributes
            return Some(format!("/{}", tag_name));
        }

        // For opening tags, sanitize attributes
        let parts: Vec<&str> = tag_content.splitn(2, ' ').collect();
        if parts.len() == 1 {
            // No attributes
            Some(tag_name)
        } else {
            // Has attributes - for now, we'll strip them all for security
            // In the future, this could be extended to allow specific attributes
            Some(tag_name)
        }
    }

    /// Escape a single character for HTML safety
    fn escape_html_char(&self, ch: char) -> String {
        match ch {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#x27;".to_string(),
            '&' => "&amp;".to_string(),
            '/' => "&#x2F;".to_string(),
            _ => ch.to_string(),
        }
    }
}

/// Escape HTML entities to prevent XSS
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;") // Must be first to avoid double-escaping
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;") // Forward slash for extra safety
}

/// Escape HTML attributes (more restrictive than general HTML escaping)
pub fn escape_html_attribute(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
        .replace('=', "&#x3D;")
        .replace('`', "&#x60;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
        .replace('\t', "&#9;")
}

/// Sanitize user input for safe display in HTML
pub fn sanitize_user_input(input: &str) -> String {
    let sanitizer = HtmlSanitizer::strip_all();
    sanitizer.sanitize(input)
}

/// Sanitize user input allowing basic formatting
pub fn sanitize_user_input_with_formatting(input: &str) -> String {
    let sanitizer = HtmlSanitizer::new();
    sanitizer.sanitize(input)
}

/// Validate and sanitize URLs to prevent javascript: and data: schemes
pub fn sanitize_url(url: &str) -> Option<String> {
    let url = url.trim();

    let parsed = Url::parse(url).ok()?;

    match parsed.scheme() {
        "http" | "https" | "ftp" | "mailto" => Some(url.to_string()),
        _ => None,
    }
}

/// Content Security Policy header value for additional XSS protection
pub const CSP_NONCE: &str = "eqrngNonce";

pub fn get_csp_header() -> &'static str {
    "default-src 'self'; script-src 'self' 'nonce-eqrngNonce'; style-src 'self' 'nonce-eqrngNonce' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'"
}

pub fn get_csp_nonce() -> &'static str {
    CSP_NONCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("Hello & goodbye"), "Hello &amp; goodbye");
        assert_eq!(escape_html("'\""), "&#x27;&quot;");
    }

    #[test]
    fn test_sanitize_strip_all() {
        let sanitizer = HtmlSanitizer::strip_all();
        assert_eq!(sanitizer.sanitize("<script>alert('xss')</script>"), "");
        assert_eq!(sanitizer.sanitize("<b>Bold</b> text"), "Bold text");
        assert_eq!(sanitizer.sanitize("Normal text"), "Normal text");
    }

    #[test]
    fn test_sanitize_with_allowed_tags() {
        let sanitizer = HtmlSanitizer::new();
        assert_eq!(sanitizer.sanitize("<b>Bold</b>"), "<b>Bold</b>");
        assert_eq!(sanitizer.sanitize("<script>evil</script>"), "evil");
        assert_eq!(
            sanitizer.sanitize("<b onclick='alert()'>Bold</b>"),
            "<b>Bold</b>"
        );
    }

    #[test]
    fn test_escape_html_characters_outside_tags() {
        let sanitizer = HtmlSanitizer::new();
        assert_eq!(
            sanitizer.sanitize("<b>Fish & Chips</b> > ' \" / < &"),
            "<b>Fish &amp; Chips</b> &gt; &#x27; &quot; &#x2F; &lt; &amp;"
        );
    }

    #[test]
    fn test_sanitize_url() {
        assert_eq!(
            sanitize_url("https://example.com"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            sanitize_url("ftp://example.com/file"),
            Some("ftp://example.com/file".to_string())
        );
        assert_eq!(
            sanitize_url("mailto:someone@example.com"),
            Some("mailto:someone@example.com".to_string())
        );
        assert_eq!(sanitize_url("javascript:alert()"), None);
        assert_eq!(sanitize_url("data:text/html,<script>"), None);
        assert_eq!(sanitize_url("/relative/path"), None);
    }
}
