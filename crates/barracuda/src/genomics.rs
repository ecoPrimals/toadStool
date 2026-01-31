//! High-level Bioinformatics and Genomics API
//!
//! This module provides production-ready interfaces for DNA/RNA sequence analysis
//! and genomics workflows. It wraps low-level operations (pattern_match, gc_content,
//! complexity_filter) into an ergonomic API for scientific computing.
//!
//! # Bioinformatics Capabilities
//!
//! - **Sequence Analysis**: Composition, GC content, complexity
//! - **Pattern Matching**: Motif discovery, pattern search
//! - **Quality Control**: Low-complexity filtering, validation
//! - **Batch Processing**: High-throughput genomics pipelines
//!
//! # Example
//!
//! ```no_run
//! use barracuda::genomics::{SequenceAnalyzer, SequenceConfig};
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! // Create analyzer
//! let analyzer = SequenceAnalyzer::new(&device, SequenceConfig::default()).await?;
//!
//! // Analyze sequence
//! let sequence = b"ATCGATCGATCG";
//! let report = analyzer.analyze_composition(sequence).await?;
//!
//! println!("GC Content: {:.1}%", report.gc_content * 100.0);
//! println!("Length: {}", report.length);
//! println!("Low-complexity regions: {}", report.low_complexity_regions.len());
//! # Ok(())
//! # }
//! ```

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result as BarracudaResult};
use crate::ops::pattern_match::pattern_match;
use crate::ops::gc_content::gc_content;
use crate::ops::complexity_filter::complexity_filter;

/// Configuration for sequence analysis
#[derive(Debug, Clone)]
pub struct SequenceConfig {
    /// Window size for complexity analysis
    pub complexity_window: u32,
    
    /// Minimum unique bases for complexity threshold
    pub min_unique_bases: u32,
    
    /// Enable parallel batch processing
    pub parallel_batch: bool,
}

impl Default for SequenceConfig {
    fn default() -> Self {
        Self {
            complexity_window: 10,
            min_unique_bases: 2,
            parallel_batch: true,
        }
    }
}

/// Region of interest in a sequence
#[derive(Debug, Clone)]
pub struct Region {
    /// Start position (0-indexed)
    pub start: usize,
    
    /// End position (exclusive)
    pub end: usize,
    
    /// Region type/annotation
    pub annotation: String,
}

/// Nucleotide composition counts
#[derive(Debug, Clone, Default)]
pub struct NucleotideCounts {
    pub a: usize,
    pub t: usize,
    pub g: usize,
    pub c: usize,
    pub n: usize, // Unknown/N bases
}

/// Composition analysis report
#[derive(Debug, Clone)]
pub struct CompositionReport {
    /// GC content as fraction (0.0-1.0)
    pub gc_content: f32,
    
    /// Sequence length
    pub length: usize,
    
    /// Low-complexity regions
    pub low_complexity_regions: Vec<Region>,
    
    /// Nucleotide counts
    pub nucleotide_counts: NucleotideCounts,
}

/// Motif match result
#[derive(Debug, Clone)]
pub struct MotifMatch {
    /// Pattern that matched
    pub pattern: Vec<u8>,
    
    /// Positions where pattern was found
    pub positions: Vec<usize>,
    
    /// Match count
    pub count: usize,
}

/// Quality report for sequence validation
#[derive(Debug, Clone)]
pub struct QualityReport {
    /// Whether sequence passes quality filters
    pub passes: bool,
    
    /// Fraction of low-complexity sequence
    pub low_complexity_fraction: f32,
    
    /// GC content (for bias detection)
    pub gc_content: f32,
    
    /// Number of unknown (N) bases
    pub n_count: usize,
    
    /// Quality issues found
    pub issues: Vec<String>,
}

/// High-level sequence analyzer for bioinformatics
pub struct SequenceAnalyzer {
    device: WgpuDevice,
    config: SequenceConfig,
}

impl SequenceAnalyzer {
    /// Create a new sequence analyzer
    ///
    /// # Arguments
    ///
    /// * `device` - WGPU device for GPU computation
    /// * `config` - Analysis configuration
    pub async fn new(device: &WgpuDevice, config: SequenceConfig) -> BarracudaResult<Self> {
        Ok(Self {
            device: device.clone(),
            config,
        })
    }
    
    /// Analyze sequence composition
    ///
    /// # Arguments
    ///
    /// * `sequence` - DNA/RNA sequence (ASCII: A, T/U, G, C, N)
    ///
    /// # Returns
    ///
    /// Comprehensive composition report
    pub async fn analyze_composition(&self, sequence: &[u8]) -> BarracudaResult<CompositionReport> {
        if sequence.is_empty() {
            return Err(BarracudaError::InvalidInput {
                message: "Sequence cannot be empty".to_string(),
            });
        }
        
        // Calculate GC content on GPU
        let gc = gc_content(&self.device.device, &self.device.queue, sequence).await?;
        
        // Find low-complexity regions on GPU
        let complexity_flags = complexity_filter(
            &self.device.device,
            &self.device.queue,
            sequence,
            self.config.complexity_window,
            self.config.min_unique_bases,
        ).await?;
        
        // Convert complexity flags to regions
        let mut low_complexity_regions = Vec::new();
        let mut in_region = false;
        let mut region_start = 0;
        
        for (i, &flag) in complexity_flags.iter().enumerate() {
            if flag > 0.5 && !in_region {
                // Start of low-complexity region
                in_region = true;
                region_start = i;
            } else if flag < 0.5 && in_region {
                // End of low-complexity region
                in_region = false;
                low_complexity_regions.push(Region {
                    start: region_start,
                    end: i,
                    annotation: "low_complexity".to_string(),
                });
            }
        }
        
        // Close last region if still open
        if in_region {
            low_complexity_regions.push(Region {
                start: region_start,
                end: sequence.len(),
                annotation: "low_complexity".to_string(),
            });
        }
        
        // Count nucleotides on CPU (small operation)
        let mut counts = NucleotideCounts::default();
        for &base in sequence {
            match base.to_ascii_uppercase() {
                b'A' => counts.a += 1,
                b'T' | b'U' => counts.t += 1,
                b'G' => counts.g += 1,
                b'C' => counts.c += 1,
                b'N' => counts.n += 1,
                _ => {} // Ignore invalid bases
            }
        }
        
        Ok(CompositionReport {
            gc_content: gc,
            length: sequence.len(),
            low_complexity_regions,
            nucleotide_counts: counts,
        })
    }
    
    /// Find motifs/patterns in sequence
    ///
    /// # Arguments
    ///
    /// * `sequence` - DNA/RNA sequence to search
    /// * `patterns` - Patterns to find
    ///
    /// # Returns
    ///
    /// List of motif matches with positions
    pub async fn find_motifs(&self, sequence: &[u8], patterns: &[&[u8]]) -> BarracudaResult<Vec<MotifMatch>> {
        if sequence.is_empty() {
            return Err(BarracudaError::InvalidInput {
                message: "Sequence cannot be empty".to_string(),
            });
        }
        
        if patterns.is_empty() {
            return Err(BarracudaError::InvalidInput {
                message: "At least one pattern required".to_string(),
            });
        }
        
        let mut matches = Vec::new();
        
        // Search for each pattern on GPU
        for pattern in patterns {
            if pattern.is_empty() {
                continue;
            }
            
            let match_flags = pattern_match(
                &self.device.device,
                &self.device.queue,
                sequence,
                pattern,
            ).await?;
            
            // Extract match positions
            let positions: Vec<usize> = match_flags.iter()
                .enumerate()
                .filter(|(_, &flag)| flag > 0.5)
                .map(|(i, _)| i)
                .collect();
            
            matches.push(MotifMatch {
                pattern: pattern.to_vec(),
                positions: positions.clone(),
                count: positions.len(),
            });
        }
        
        Ok(matches)
    }
    
    /// Perform quality control on sequence
    ///
    /// # Arguments
    ///
    /// * `sequence` - Sequence to validate
    ///
    /// # Returns
    ///
    /// Quality report with pass/fail and issues
    pub async fn quality_filter(&self, sequence: &[u8]) -> BarracudaResult<QualityReport> {
        if sequence.is_empty() {
            return Ok(QualityReport {
                passes: false,
                low_complexity_fraction: 0.0,
                gc_content: 0.0,
                n_count: 0,
                issues: vec!["Empty sequence".to_string()],
            });
        }
        
        // Analyze composition
        let composition = self.analyze_composition(sequence).await?;
        
        // Calculate low-complexity fraction
        let low_complexity_bases: usize = composition.low_complexity_regions.iter()
            .map(|r| r.end - r.start)
            .sum();
        let low_complexity_fraction = low_complexity_bases as f32 / sequence.len() as f32;
        
        // Check for quality issues
        let mut issues = Vec::new();
        let mut passes = true;
        
        // Too short
        if sequence.len() < 50 {
            issues.push("Sequence too short (< 50 bp)".to_string());
            passes = false;
        }
        
        // Too much low complexity
        if low_complexity_fraction > 0.5 {
            issues.push(format!("High low-complexity content: {:.1}%", low_complexity_fraction * 100.0));
            passes = false;
        }
        
        // GC bias
        if composition.gc_content < 0.2 || composition.gc_content > 0.8 {
            issues.push(format!("GC bias: {:.1}%", composition.gc_content * 100.0));
            // Warning, not failure
        }
        
        // Too many N bases
        let n_fraction = composition.nucleotide_counts.n as f32 / sequence.len() as f32;
        if n_fraction > 0.1 {
            issues.push(format!("High N content: {:.1}%", n_fraction * 100.0));
            passes = false;
        }
        
        Ok(QualityReport {
            passes,
            low_complexity_fraction,
            gc_content: composition.gc_content,
            n_count: composition.nucleotide_counts.n,
            issues,
        })
    }
    
    /// Process multiple sequences in batch
    ///
    /// # Arguments
    ///
    /// * `sequences` - Batch of sequences to analyze
    ///
    /// # Returns
    ///
    /// Composition reports for each sequence
    pub async fn process_batch(&self, sequences: &[Vec<u8>]) -> BarracudaResult<Vec<CompositionReport>> {
        if sequences.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut reports = Vec::with_capacity(sequences.len());
        
        // TODO: Implement parallel batch processing when config.parallel_batch is true
        // For now, process sequentially
        for sequence in sequences {
            reports.push(self.analyze_composition(sequence).await?);
        }
        
        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_analyzer_creation() {
        let device = WgpuDevice::new().await.unwrap();
        let analyzer = SequenceAnalyzer::new(&device, SequenceConfig::default()).await.unwrap();
        assert_eq!(analyzer.config.complexity_window, 10);
    }
    
    #[tokio::test]
    async fn test_composition_analysis() {
        let device = WgpuDevice::new().await.unwrap();
        let analyzer = SequenceAnalyzer::new(&device, SequenceConfig::default()).await.unwrap();
        
        let sequence = b"ATCGATCGATCG";
        let report = analyzer.analyze_composition(sequence).await.unwrap();
        
        assert_eq!(report.length, 12);
        assert!((report.gc_content - 0.5).abs() < 0.1); // 6 GC out of 12 = 50%
        assert_eq!(report.nucleotide_counts.a, 3);
        assert_eq!(report.nucleotide_counts.t, 3);
        assert_eq!(report.nucleotide_counts.g, 3);
        assert_eq!(report.nucleotide_counts.c, 3);
    }
    
    #[tokio::test]
    async fn test_motif_finding() {
        let device = WgpuDevice::new().await.unwrap();
        let analyzer = SequenceAnalyzer::new(&device, SequenceConfig::default()).await.unwrap();
        
        let sequence = b"ATCGATCGATCG";
        let patterns = vec![b"ATC".as_ref(), b"TCG".as_ref()];
        
        let matches = analyzer.find_motifs(sequence, &patterns).await.unwrap();
        
        assert_eq!(matches.len(), 2);
        assert!(matches[0].count > 0);
        assert!(matches[1].count > 0);
    }
    
    #[tokio::test]
    async fn test_quality_filter() {
        let device = WgpuDevice::new().await.unwrap();
        let analyzer = SequenceAnalyzer::new(&device, SequenceConfig::default()).await.unwrap();
        
        // Good sequence (longer than window size)
        let good_seq = b"ATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCG";
        let report = analyzer.quality_filter(good_seq).await.unwrap();
        assert!(report.passes);
        
        // Too short
        let short_seq = b"ATCG"; // 4 bp < window size (10)
        let report = analyzer.quality_filter(short_seq).await;
        // Should handle error gracefully since window > sequence length
        assert!(report.is_err() || !report.unwrap().passes);
    }
    
    #[tokio::test]
    async fn test_batch_processing() {
        let device = WgpuDevice::new().await.unwrap();
        let analyzer = SequenceAnalyzer::new(&device, SequenceConfig::default()).await.unwrap();
        
        let sequences = vec![
            b"ATCGATCGATCGATCGATCGATCG".to_vec(),
            b"GCGCGCGCGCGCGCGCGCGCGCGC".to_vec(),
            b"ATATATATATATATATATATAT".to_vec(),
        ];
        
        let reports = analyzer.process_batch(&sequences).await.unwrap();
        assert_eq!(reports.len(), 3);
        assert!(reports.iter().all(|r| r.length > 0));
    }
}
