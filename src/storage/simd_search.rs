//! SIMD-accelerated search operations for blazing fast trace matching.
//!
//! Uses CPU vector instructions for parallel comparisons and pattern matching.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// SIMD-accelerated batch comparison of u128 trace IDs
///
/// Compares a needle against a haystack using AVX2 instructions for
/// parallel processing of multiple values simultaneously.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn find_trace_id_simd_internal(needle: u128, haystack: &[u128]) -> Option<usize> {
    if !is_x86_feature_detected!("avx2") {
        return find_trace_id_scalar(needle, haystack);
    }

    let len = haystack.len();
    if len < 4 {
        return find_trace_id_scalar(needle, haystack);
    }

    // Process 4 u128s at a time using AVX2 (256-bit registers)
    let needle_low = needle as u64;
    let needle_high = (needle >> 64) as u64;

    // Broadcast needle to all lanes
    let needle_low_vec = _mm256_set1_epi64x(needle_low as i64);
    let needle_high_vec = _mm256_set1_epi64x(needle_high as i64);

    let mut i = 0;
    while i + 4 <= len {
        // Load 4 u128 values (as 8 u64 values)
        let ptr = haystack.as_ptr().add(i) as *const u64;

        // Load low and high parts
        let data_low = _mm256_loadu_si256(ptr as *const __m256i);
        let data_high = _mm256_loadu_si256(ptr.add(4) as *const __m256i);

        // Compare low parts
        let cmp_low = _mm256_cmpeq_epi64(data_low, needle_low_vec);
        // Compare high parts
        let cmp_high = _mm256_cmpeq_epi64(data_high, needle_high_vec);

        // Combine comparisons
        let combined = _mm256_and_si256(cmp_low, cmp_high);
        let mask = _mm256_movemask_epi8(combined);

        if mask != 0 {
            // Found a match, determine which lane
            for j in 0..4 {
                if haystack[i + j] == needle {
                    return Some(i + j);
                }
            }
        }

        i += 4;
    }

    // Handle remaining elements
    while i < len {
        if haystack[i] == needle {
            return Some(i);
        }
        i += 1;
    }

    None
}

/// Public API for trace ID search with SIMD acceleration
#[inline]
pub fn find_trace_id_simd(needle: u128, haystack: &[u128]) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { find_trace_id_simd_internal(needle, haystack) }
        } else {
            find_trace_id_scalar(needle, haystack)
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        find_trace_id_scalar(needle, haystack)
    }
}

/// Scalar fallback for non-SIMD systems
#[inline]
fn find_trace_id_scalar(needle: u128, haystack: &[u128]) -> Option<usize> {
    haystack.iter().position(|&x| x == needle)
}

/// SIMD-accelerated string pattern matching
///
/// Uses SIMD instructions to search for pattern occurrences in text.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn find_pattern_simd(pattern: &[u8], text: &[u8]) -> Vec<usize> {
    if !is_x86_feature_detected!("avx2") {
        return find_pattern_scalar(pattern, text);
    }

    let mut matches = Vec::new();
    let pattern_len = pattern.len();
    let text_len = text.len();

    if pattern_len == 0 || pattern_len > text_len {
        return matches;
    }

    // Use first byte for initial SIMD scan
    let first_byte = pattern[0];
    let first_byte_vec = _mm256_set1_epi8(first_byte as i8);

    let mut i = 0;
    while i + 32 <= text_len - pattern_len + 1 {
        // Load 32 bytes of text
        let text_chunk = _mm256_loadu_si256(text.as_ptr().add(i) as *const __m256i);

        // Compare with first byte of pattern
        let cmp = _mm256_cmpeq_epi8(text_chunk, first_byte_vec);
        let mask = _mm256_movemask_epi8(cmp) as u32;

        if mask != 0 {
            // Check each potential match
            for j in 0..32 {
                if (mask & (1 << j)) != 0 {
                    let pos = i + j;
                    if pos + pattern_len <= text_len {
                        // Verify full pattern match
                        if &text[pos..pos + pattern_len] == pattern {
                            matches.push(pos);
                        }
                    }
                }
            }
        }

        i += 32;
    }

    // Handle remaining bytes
    while i <= text_len - pattern_len {
        if &text[i..i + pattern_len] == pattern {
            matches.push(i);
        }
        i += 1;
    }

    matches
}

/// Scalar fallback for pattern matching
#[inline]
fn find_pattern_scalar(pattern: &[u8], text: &[u8]) -> Vec<usize> {
    let mut matches = Vec::new();
    let pattern_len = pattern.len();
    let text_len = text.len();

    if pattern_len == 0 || pattern_len > text_len {
        return matches;
    }

    for i in 0..=text_len - pattern_len {
        if &text[i..i + pattern_len] == pattern {
            matches.push(i);
        }
    }

    matches
}

/// SIMD-accelerated batch scoring for search results
///
/// Computes relevance scores for multiple items in parallel.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn compute_scores_simd(lengths: &[u32], weights: &[f32]) -> Vec<f32> {
    if !is_x86_feature_detected!("avx2") {
        return compute_scores_scalar(lengths, weights);
    }

    let len = lengths.len().min(weights.len());
    let mut scores = vec![0.0f32; len];

    let mut i = 0;
    while i + 8 <= len {
        // Load 8 lengths and weights
        let lengths_vec = _mm256_loadu_ps(lengths.as_ptr().add(i) as *const f32);
        let weights_vec = _mm256_loadu_ps(weights.as_ptr().add(i));

        // Multiply lengths by weights
        let scores_vec = _mm256_mul_ps(lengths_vec, weights_vec);

        // Store results
        _mm256_storeu_ps(scores.as_mut_ptr().add(i), scores_vec);

        i += 8;
    }

    // Handle remaining elements
    while i < len {
        scores[i] = lengths[i] as f32 * weights[i];
        i += 1;
    }

    scores
}

/// Scalar fallback for score computation
#[inline]
fn compute_scores_scalar(lengths: &[u32], weights: &[f32]) -> Vec<f32> {
    lengths
        .iter()
        .zip(weights.iter())
        .map(|(&len, &weight)| len as f32 * weight)
        .collect()
}

/// Batch check if any values in the array match the target using SIMD
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn contains_u64_simd(haystack: &[u64], needle: u64) -> bool {
    unsafe {
        if is_x86_feature_detected!("avx2") {
            contains_u64_avx2(haystack, needle)
        } else {
            haystack.contains(&needle)
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn contains_u64_avx2(haystack: &[u64], needle: u64) -> bool {
    let len = haystack.len();
    if len < 4 {
        return haystack.contains(&needle);
    }

    let needle_vec = _mm256_set1_epi64x(needle as i64);

    let mut i = 0;
    while i + 4 <= len {
        let data = _mm256_loadu_si256(haystack.as_ptr().add(i) as *const __m256i);
        let cmp = _mm256_cmpeq_epi64(data, needle_vec);
        let mask = _mm256_movemask_epi8(cmp);

        if mask != 0 {
            return true;
        }

        i += 4;
    }

    // Check remaining elements
    while i < len {
        if haystack[i] == needle {
            return true;
        }
        i += 1;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_trace_id_simd() {
        let haystack: Vec<u128> = (0..1000).map(|i| i as u128 * 12345).collect();
        let needle = 500 * 12345;

        unsafe {
            let result = find_trace_id_simd(needle, &haystack);
            assert_eq!(result, Some(500));

            let not_found = find_trace_id_simd(u128::MAX, &haystack);
            assert_eq!(not_found, None);
        }
    }

    #[test]
    fn test_pattern_matching_simd() {
        let text = b"hello world hello universe hello cosmos";
        let pattern = b"hello";

        unsafe {
            let matches = find_pattern_simd(pattern, text);
            assert_eq!(matches, vec![0, 12, 27]);
        }
    }

    #[test]
    fn test_batch_scoring() {
        let lengths = vec![10, 20, 30, 40, 50, 60, 70, 80];
        let weights = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];

        unsafe {
            let scores = compute_scores_simd(&lengths, &weights);
            assert_eq!(scores, vec![5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0]);
        }
    }

    #[test]
    fn test_contains_u64() {
        let haystack: Vec<u64> = (0..1000).collect();

        assert!(contains_u64_simd(&haystack, 500));
        assert!(!contains_u64_simd(&haystack, 1500));
    }
}
