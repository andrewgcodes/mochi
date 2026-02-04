//! Parameter parsing for CSI sequences
//!
//! CSI parameters are semicolon-separated numbers with optional subparameters
//! separated by colons (for extended color sequences).

use serde::{Deserialize, Serialize};

/// Maximum number of parameters
pub const MAX_PARAMS: usize = 32;

/// Maximum parameter value
pub const MAX_PARAM_VALUE: u16 = 65535;

/// Parsed parameters from a CSI sequence
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Params {
    /// Parameter values
    values: Vec<u16>,
    /// Subparameter boundaries (for colon-separated values)
    /// Each entry is the start index of a new parameter group
    subparam_starts: Vec<usize>,
}

impl Params {
    pub fn new() -> Self {
        Params {
            values: Vec::with_capacity(8),
            subparam_starts: vec![0],
        }
    }

    /// Add a parameter value
    pub fn push(&mut self, value: u16) {
        if self.values.len() < MAX_PARAMS {
            self.values.push(value);
        }
    }

    /// Start a new parameter group (after semicolon)
    pub fn next_param(&mut self) {
        self.subparam_starts.push(self.values.len());
    }

    /// Get the number of parameter groups
    pub fn len(&self) -> usize {
        self.subparam_starts.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get parameter at index (first value of the group)
    pub fn get(&self, index: usize) -> Option<u16> {
        if index >= self.subparam_starts.len() {
            return None;
        }
        let start = self.subparam_starts[index];
        self.values.get(start).copied()
    }

    /// Get parameter at index, or default value
    pub fn get_or(&self, index: usize, default: u16) -> u16 {
        self.get(index).unwrap_or(default)
    }

    /// Get all subparameters for a parameter group
    pub fn get_subparams(&self, index: usize) -> Option<&[u16]> {
        if index >= self.subparam_starts.len() {
            return None;
        }
        let start = self.subparam_starts[index];
        let end = self
            .subparam_starts
            .get(index + 1)
            .copied()
            .unwrap_or(self.values.len());
        Some(&self.values[start..end])
    }

    /// Get all values as a flat slice
    pub fn as_slice(&self) -> &[u16] {
        &self.values
    }

    /// Convert to a simple Vec (first value of each group)
    pub fn to_vec(&self) -> Vec<u16> {
        (0..self.len())
            .filter_map(|i| self.get(i))
            .collect()
    }

    /// Clear all parameters
    pub fn clear(&mut self) {
        self.values.clear();
        self.subparam_starts.clear();
        self.subparam_starts.push(0);
    }

    /// Parse parameters from a byte slice
    /// Format: "1;2;3" or "38:2:255:128:0;48:5:196"
    pub fn parse(bytes: &[u8]) -> Self {
        let mut params = Params::new();
        let mut current_value: u32 = 0;
        let mut has_value = false;

        for &byte in bytes {
            match byte {
                b'0'..=b'9' => {
                    current_value = current_value.saturating_mul(10).saturating_add((byte - b'0') as u32);
                    has_value = true;
                }
                b';' => {
                    if has_value {
                        params.push(current_value.min(MAX_PARAM_VALUE as u32) as u16);
                    } else {
                        params.push(0); // Empty parameter defaults to 0
                    }
                    params.next_param();
                    current_value = 0;
                    has_value = false;
                }
                b':' => {
                    // Subparameter separator
                    if has_value {
                        params.push(current_value.min(MAX_PARAM_VALUE as u32) as u16);
                    } else {
                        params.push(0);
                    }
                    current_value = 0;
                    has_value = false;
                }
                _ => {
                    // Ignore invalid characters
                }
            }
        }

        // Push final value
        if has_value {
            params.push(current_value.min(MAX_PARAM_VALUE as u32) as u16);
        } else if !params.values.is_empty() || bytes.ends_with(b";") {
            // Trailing semicolon or empty final parameter
            params.push(0);
        }

        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_simple() {
        let params = Params::parse(b"1;2;3");
        assert_eq!(params.len(), 3);
        assert_eq!(params.get(0), Some(1));
        assert_eq!(params.get(1), Some(2));
        assert_eq!(params.get(2), Some(3));
    }

    #[test]
    fn test_params_empty() {
        let params = Params::parse(b"");
        assert_eq!(params.len(), 1);
        assert!(params.is_empty());
    }

    #[test]
    fn test_params_defaults() {
        let params = Params::parse(b";5;");
        assert_eq!(params.len(), 3);
        assert_eq!(params.get(0), Some(0));
        assert_eq!(params.get(1), Some(5));
        assert_eq!(params.get(2), Some(0));
    }

    #[test]
    fn test_params_subparams() {
        let params = Params::parse(b"38:2:255:128:0");
        assert_eq!(params.len(), 1);
        let subparams = params.get_subparams(0).unwrap();
        assert_eq!(subparams, &[38, 2, 255, 128, 0]);
    }

    #[test]
    fn test_params_mixed() {
        let params = Params::parse(b"38:2:255:0:0;1");
        assert_eq!(params.len(), 2);
        let subparams = params.get_subparams(0).unwrap();
        assert_eq!(subparams, &[38, 2, 255, 0, 0]);
        assert_eq!(params.get(1), Some(1));
    }

    #[test]
    fn test_params_large_value() {
        let params = Params::parse(b"99999999");
        assert_eq!(params.get(0), Some(65535)); // Clamped to max
    }

    #[test]
    fn test_params_to_vec() {
        let params = Params::parse(b"1;2;3");
        assert_eq!(params.to_vec(), vec![1, 2, 3]);
    }
}
