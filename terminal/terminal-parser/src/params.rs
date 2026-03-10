//! CSI parameter parsing
//!
//! Handles parsing of semicolon-separated numeric parameters in CSI sequences.

/// Maximum number of parameters we'll track
const MAX_PARAMS: usize = 32;

/// CSI parameters
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// Parameter values (0 means default/unspecified)
    values: Vec<u16>,
    /// Subparameters (for colon-separated values like SGR)
    subparams: Vec<Vec<u16>>,
}

impl Params {
    /// Create empty params
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            subparams: Vec::new(),
        }
    }

    /// Create params from a slice
    pub fn from_slice(values: &[u16]) -> Self {
        Self {
            values: values.to_vec(),
            subparams: Vec::new(),
        }
    }

    /// Parse parameters from bytes
    ///
    /// Handles both semicolon-separated parameters (`1;2;3`) and colon-separated
    /// subparameters (`4:3`, `38:2:255:128:64`). For colon-separated values like
    /// `4:3`, the first value (`4`) becomes the main parameter and subsequent values
    /// (`3`) become subparameters. This matches the VT/xterm convention where
    /// `SGR 4:3` means "underline (4) with style curly (3)".
    pub fn parse(bytes: &[u8]) -> Self {
        let mut params = Self::new();
        let mut current: u16 = 0;
        let mut has_value = false;
        let mut current_subparams: Vec<u16> = Vec::new();
        // Track the main param value when we're inside a colon-separated group.
        // The first value before/after the first colon is the main parameter.
        let mut colon_main: Option<u16> = None;

        for &byte in bytes {
            match byte {
                b'0'..=b'9' => {
                    has_value = true;
                    current = current
                        .saturating_mul(10)
                        .saturating_add((byte - b'0') as u16);
                }
                b';' => {
                    if params.values.len() < MAX_PARAMS {
                        if let Some(main_val) = colon_main.take() {
                            // We were in a colon-separated group.
                            // main_val is the first value (the main param).
                            // current is the last colon-separated value - add it to subparams.
                            if has_value {
                                current_subparams.push(current);
                            }
                            params.values.push(main_val);
                            params.subparams.push(current_subparams.clone());
                            current_subparams.clear();
                        } else {
                            params.values.push(if has_value { current } else { 0 });
                            params.subparams.push(Vec::new());
                        }
                    }
                    current = 0;
                    has_value = false;
                }
                b':' => {
                    // Subparameter separator (used in SGR for underline styles, etc.)
                    // The first value becomes the main parameter, subsequent values
                    // become subparameters.
                    if colon_main.is_none() {
                        // First colon: current value is the main parameter
                        colon_main = Some(if has_value { current } else { 0 });
                    } else {
                        // Subsequent colons: current value is a subparameter
                        current_subparams.push(if has_value { current } else { 0 });
                    }
                    current = 0;
                    has_value = false;
                }
                _ => {
                    // Ignore other bytes (intermediates are handled separately)
                }
            }
        }

        // Don't forget the last parameter
        if (has_value || !params.values.is_empty() || colon_main.is_some())
            && params.values.len() < MAX_PARAMS
        {
            if let Some(main_val) = colon_main.take() {
                // We were in a colon-separated group at the end.
                if has_value {
                    current_subparams.push(current);
                }
                params.values.push(main_val);
                params.subparams.push(current_subparams);
            } else {
                params.values.push(if has_value { current } else { 0 });
                params.subparams.push(Vec::new());
            }
        }

        params
    }

    /// Get parameter at index, returning None if not present
    pub fn get(&self, index: usize) -> Option<u16> {
        self.values.get(index).copied().filter(|&v| v != 0)
    }

    /// Get parameter at index with default value
    pub fn get_or(&self, index: usize, default: u16) -> u16 {
        self.get(index).unwrap_or(default)
    }

    /// Get raw value at index (0 if not present)
    pub fn raw(&self, index: usize) -> u16 {
        self.values.get(index).copied().unwrap_or(0)
    }

    /// Get number of parameters
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Check if any parameter has subparameters (colon-separated values)
    pub fn has_subparams(&self) -> bool {
        self.subparams.iter().any(|s| !s.is_empty())
    }

    /// Get subparameters for a parameter
    pub fn subparams(&self, index: usize) -> Option<&[u16]> {
        self.subparams.get(index).map(|v| v.as_slice())
    }

    /// Iterate over parameters
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.values.iter().copied()
    }

    /// Iterate over parameters with subparameters
    pub fn iter_with_subparams(&self) -> impl Iterator<Item = (u16, &[u16])> + '_ {
        self.values.iter().enumerate().map(move |(i, &v)| {
            let subparams = self.subparams.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
            (v, subparams)
        })
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_empty() {
        let params = Params::parse(b"");
        assert!(params.is_empty());
    }

    #[test]
    fn test_params_single() {
        let params = Params::parse(b"42");
        assert_eq!(params.len(), 1);
        assert_eq!(params.get(0), Some(42));
    }

    #[test]
    fn test_params_multiple() {
        let params = Params::parse(b"1;2;3");
        assert_eq!(params.len(), 3);
        assert_eq!(params.get(0), Some(1));
        assert_eq!(params.get(1), Some(2));
        assert_eq!(params.get(2), Some(3));
    }

    #[test]
    fn test_params_default() {
        let params = Params::parse(b";5;");
        assert_eq!(params.len(), 3);
        assert_eq!(params.get(0), None); // Default (0)
        assert_eq!(params.get(1), Some(5));
        assert_eq!(params.get(2), None); // Default (0)
        assert_eq!(params.get_or(0, 1), 1);
    }

    #[test]
    fn test_params_large_value() {
        let params = Params::parse(b"65535");
        assert_eq!(params.get(0), Some(65535));
    }

    #[test]
    fn test_params_overflow() {
        // Should saturate instead of overflow
        let params = Params::parse(b"99999");
        // Value should be saturated to u16::MAX
        assert_eq!(params.get(0), Some(65535));
    }

    #[test]
    fn test_params_subparams() {
        // SGR with subparameters: 38:2:255:128:64 (RGB color)
        let params = Params::parse(b"38:2:255:128:64");
        assert_eq!(params.len(), 1);
        // Main value should be 38 (the first colon-separated value)
        assert_eq!(params.raw(0), 38);
        // Subparams should be [2, 255, 128, 64]
        let subparams = params.subparams(0).unwrap();
        assert_eq!(subparams, &[2, 255, 128, 64]);
    }

    #[test]
    fn test_params_subparams_underline_style() {
        // SGR 4:3 = curly underline
        let params = Params::parse(b"4:3");
        assert_eq!(params.len(), 1);
        assert_eq!(params.raw(0), 4); // Main param is 4 (underline)
        let subparams = params.subparams(0).unwrap();
        assert_eq!(subparams, &[3]); // Subparam is 3 (curly)
    }

    #[test]
    fn test_params_subparams_mixed() {
        // Mixed: 4:3;1 = curly underline + bold
        let params = Params::parse(b"4:3;1");
        assert_eq!(params.len(), 2);
        assert_eq!(params.raw(0), 4); // underline
        assert_eq!(params.raw(1), 1); // bold
        let subparams = params.subparams(0).unwrap();
        assert_eq!(subparams, &[3]); // curly
        let subparams1 = params.subparams(1).unwrap();
        assert!(subparams1.is_empty()); // no subparams for bold
    }

    #[test]
    fn test_params_subparams_color_then_param() {
        // 38:2:255:0:0;1 = RGB red foreground + bold
        let params = Params::parse(b"38:2:255:0:0;1");
        assert_eq!(params.len(), 2);
        assert_eq!(params.raw(0), 38);
        assert_eq!(params.raw(1), 1);
        let subparams = params.subparams(0).unwrap();
        assert_eq!(subparams, &[2, 255, 0, 0]);
    }

    #[test]
    fn test_params_iter() {
        let params = Params::parse(b"1;2;3");
        let values: Vec<_> = params.iter().collect();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_params_iter_with_subparams() {
        let params = Params::parse(b"4:3;38:2:255:0:0");
        let items: Vec<_> = params.iter_with_subparams().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, 4); // underline
        assert_eq!(items[0].1, &[3]); // curly
        assert_eq!(items[1].0, 38); // fg color
        assert_eq!(items[1].1, &[2, 255, 0, 0]); // RGB
    }
}
