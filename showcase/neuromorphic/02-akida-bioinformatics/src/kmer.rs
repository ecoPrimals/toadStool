// SPDX-License-Identifier: AGPL-3.0-or-later
//! K-mer extraction and analysis

use anyhow::Result;

/// Extract all k-mers from a DNA sequence
pub fn extract_kmers(sequence: &[u8], k: usize) -> Vec<Vec<u8>> {
    if sequence.len() < k {
        return Vec::new();
    }
    
    (0..=sequence.len() - k)
        .map(|i| sequence[i..i + k].to_vec())
        .collect()
}

/// Calculate GC content of a sequence (percentage of G or C bases)
pub fn gc_content(sequence: &[u8]) -> f64 {
    if sequence.is_empty() {
        return 0.0;
    }
    
    let gc_count = sequence
        .iter()
        .filter(|&&b| b == b'G' || b == b'C' || b == b'g' || b == b'c')
        .count();
    
    gc_count as f64 / sequence.len() as f64
}

/// Check if sequence is low complexity (e.g., many repeated bases)
pub fn is_low_complexity(sequence: &[u8]) -> bool {
    if sequence.len() < 4 {
        return false;
    }
    
    // Count each base type
    let mut counts = [0u32; 4]; // A, C, G, T
    
    for &base in sequence {
        let index = match base {
            b'A' | b'a' => 0,
            b'C' | b'c' => 1,
            b'G' | b'g' => 2,
            b'T' | b't' => 3,
            _ => continue,
        };
        counts[index] += 1;
    }
    
    // If any single base dominates (>70%), it's low complexity
    let max_count = *counts.iter().max().unwrap();
    let total = counts.iter().sum::<u32>();
    
    if total == 0 {
        return true;
    }
    
    max_count as f64 / total as f64 > 0.7
}

/// Check if sequence contains common adapter sequences
pub fn is_adapter(sequence: &[u8]) -> bool {
    // Common Illumina adapters (simplified check)
    let adapters = [
        b"AGATCGGAAGAGC",  // TruSeq
        b"CTGTCTCTTATA",   // Nextera
    ];
    
    for adapter in &adapters {
        if contains_subsequence(sequence, adapter) {
            return true;
        }
    }
    
    false
}

/// Check if haystack contains needle
fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Convert DNA sequence to one-hot encoding for neural network
///
/// Returns a flattened vector where each base is encoded as 4 values:
/// A = [1,0,0,0], C = [0,1,0,0], G = [0,0,1,0], T = [0,0,0,1]
pub fn to_one_hot(sequence: &[u8]) -> Vec<f32> {
    let mut encoding = Vec::with_capacity(sequence.len() * 4);
    
    for &base in sequence {
        let one_hot = match base {
            b'A' | b'a' => [1.0, 0.0, 0.0, 0.0],
            b'C' | b'c' => [0.0, 1.0, 0.0, 0.0],
            b'G' | b'g' => [0.0, 0.0, 1.0, 0.0],
            b'T' | b't' => [0.0, 0.0, 0.0, 1.0],
            _ => [0.0, 0.0, 0.0, 0.0], // Unknown base
        };
        encoding.extend_from_slice(&one_hot);
    }
    
    encoding
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_kmers() {
        let sequence = b"ACGTACGT";
        let kmers = extract_kmers(sequence, 4);
        
        assert_eq!(kmers.len(), 5);
        assert_eq!(kmers[0], b"ACGT");
        assert_eq!(kmers[1], b"CGTA");
        assert_eq!(kmers[4], b"ACGT");
    }
    
    #[test]
    fn test_gc_content() {
        assert!((gc_content(b"AAAA") - 0.0).abs() < 0.01);
        assert!((gc_content(b"GGGG") - 1.0).abs() < 0.01);
        assert!((gc_content(b"ACGT") - 0.5).abs() < 0.01);
        assert!((gc_content(b"ACGTACGT") - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_low_complexity() {
        assert!(is_low_complexity(b"AAAAAAAA"));
        assert!(is_low_complexity(b"TTTTTTTT"));
        assert!(!is_low_complexity(b"ACGTACGT"));
        assert!(!is_low_complexity(b"ACGACGACG"));
    }
    
    #[test]
    fn test_adapter_detection() {
        assert!(is_adapter(b"ACGTACGTACGTAGATCGGAAGAGCACGT"));
        assert!(is_adapter(b"CTGTCTCTTATAACGT"));
        assert!(!is_adapter(b"ACGTACGTACGT"));
    }
    
    #[test]
    fn test_one_hot_encoding() {
        let encoding = to_one_hot(b"ACGT");
        
        assert_eq!(encoding.len(), 16); // 4 bases * 4 values
        
        // Check A encoding
        assert_eq!(&encoding[0..4], &[1.0, 0.0, 0.0, 0.0]);
        // Check C encoding
        assert_eq!(&encoding[4..8], &[0.0, 1.0, 0.0, 0.0]);
        // Check G encoding
        assert_eq!(&encoding[8..12], &[0.0, 0.0, 1.0, 0.0]);
        // Check T encoding
        assert_eq!(&encoding[12..16], &[0.0, 0.0, 0.0, 1.0]);
    }
}

