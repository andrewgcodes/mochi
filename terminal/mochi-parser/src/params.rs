//! Parameter parsing for CSI and other sequences.
//!
//! CSI sequences have parameters separated by semicolons.
//! Parameters can be:
//! - Empty (defaults to 0 or 1 depending on context)
//! - Single numbers
//! - Subparameters separated by colons (for SGR extended colors)

use std::fmt;

pub const MAX_PARAMS: usize = 32;
pub const MAX_SUBPARAMS: usize = 16;

#[derive(Clone)]
pub struct Params {
    params: [u16; MAX_PARAMS],
    subparams: [[u16; MAX_SUBPARAMS]; MAX_PARAMS],
    subparam_counts: [usize; MAX_PARAMS],
    len: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}

impl Params {
    pub fn new() -> Self {
        Params {
            params: [0; MAX_PARAMS],
            subparams: [[0; MAX_SUBPARAMS]; MAX_PARAMS],
            subparam_counts: [0; MAX_PARAMS],
            len: 0,
        }
    }

    pub fn push(&mut self, value: u16) {
        if self.len < MAX_PARAMS {
            self.params[self.len] = value;
            self.subparam_counts[self.len] = 0;
            self.len += 1;
        }
    }

    pub fn push_subparam(&mut self, value: u16) {
        if self.len == 0 {
            self.push(0);
        }
        let idx = self.len - 1;
        let sub_idx = self.subparam_counts[idx];
        if sub_idx < MAX_SUBPARAMS {
            self.subparams[idx][sub_idx] = value;
            self.subparam_counts[idx] += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> Option<u16> {
        if index < self.len {
            Some(self.params[index])
        } else {
            None
        }
    }

    pub fn get_or(&self, index: usize, default: u16) -> u16 {
        self.get(index).unwrap_or(default)
    }

    pub fn get_nonzero_or(&self, index: usize, default: u16) -> u16 {
        match self.get(index) {
            Some(0) | None => default,
            Some(v) => v,
        }
    }

    pub fn get_subparams(&self, index: usize) -> Option<&[u16]> {
        if index < self.len {
            let count = self.subparam_counts[index];
            if count > 0 {
                Some(&self.subparams[index][..count])
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.params[..self.len].iter().copied()
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl fmt::Debug for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for i in 0..self.len {
            if self.subparam_counts[i] > 0 {
                let mut v = vec![self.params[i]];
                v.extend_from_slice(&self.subparams[i][..self.subparam_counts[i]]);
                list.entry(&v);
            } else {
                list.entry(&self.params[i]);
            }
        }
        list.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_basic() {
        let mut params = Params::new();
        params.push(1);
        params.push(2);
        params.push(3);
        
        assert_eq!(params.len(), 3);
        assert_eq!(params.get(0), Some(1));
        assert_eq!(params.get(1), Some(2));
        assert_eq!(params.get(2), Some(3));
        assert_eq!(params.get(3), None);
    }

    #[test]
    fn test_params_defaults() {
        let params = Params::new();
        assert_eq!(params.get_or(0, 1), 1);
        assert_eq!(params.get_nonzero_or(0, 5), 5);
    }

    #[test]
    fn test_params_zero_default() {
        let mut params = Params::new();
        params.push(0);
        assert_eq!(params.get_nonzero_or(0, 5), 5);
    }

    #[test]
    fn test_subparams() {
        let mut params = Params::new();
        params.push(38);
        params.push_subparam(2);
        params.push_subparam(255);
        params.push_subparam(128);
        params.push_subparam(64);
        
        assert_eq!(params.get(0), Some(38));
        let subs = params.get_subparams(0).unwrap();
        assert_eq!(subs, &[2, 255, 128, 64]);
    }
}
