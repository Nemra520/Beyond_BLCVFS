use super::bytes_parser::BytesParser;

/// SparkBytes .bytes file parser
pub struct SparkBytesParser;

impl SparkBytesParser {
    /// Parse .bytes file data and return formatted JSON string
    pub fn parse_to_json(data: &[u8]) -> String {
        let value = BytesParser::parse_bytes_file(data);
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    }
}
