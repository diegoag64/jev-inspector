use std::ops::Range;

const CHECKPOINT_STRIDE: usize = 256;

/// Count every occurrence, keeping one byte offset per block rather than per match.
#[derive(Default)]
pub struct SearchIndex {
    query: String,
    count: usize,
    checkpoints: Vec<usize>,
}

impl SearchIndex {
    pub fn rebuild(&mut self, source: &str, query: &str) {
        self.clear();
        self.query.push_str(query);
        if query.is_empty() {
            return;
        }
        for (i, (offset, _)) in source.match_indices(query).enumerate() {
            if i % CHECKPOINT_STRIDE == 0 {
                self.checkpoints.push(offset);
            }
            self.count += 1;
        }
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.count = 0;
        self.checkpoints.clear();
    }

    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn range(&self, source: &str, index: usize) -> Option<Range<usize>> {
        if index >= self.count {
            return None;
        }
        let start = self.checkpoints[index / CHECKPOINT_STRIDE];
        source[start..]
            .match_indices(&self.query)
            .nth(index % CHECKPOINT_STRIDE)
            .map(|(offset, value)| start + offset..start + offset + value.len())
    }

    /// Keep non-overlapping search alignment even if the preview begins mid-match.
    pub fn visible<'a>(
        &'a self,
        source: &'a str,
        start: usize,
        end: usize,
    ) -> impl Iterator<Item = (usize, Range<usize>)> + 'a {
        let block = self
            .checkpoints
            .partition_point(|offset| *offset <= start)
            .saturating_sub(1);
        let offset = self.checkpoints.get(block).copied().unwrap_or(0);
        source[offset..]
            .match_indices(&self.query)
            .take(if self.is_empty() { 0 } else { usize::MAX })
            .enumerate()
            .map(move |(i, (at, value))| {
                (
                    block * CHECKPOINT_STRIDE + i,
                    offset + at..offset + at + value.len(),
                )
            })
            .skip_while(move |(_, range)| range.end <= start)
            .take_while(move |(_, range)| range.start < end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_matches_remain_addressable_across_checkpoint_and_old_limit_boundaries() {
        let source = format!("{}tail", "\u{754c}\u{1f680}billing ".repeat(2001));
        let expected: Vec<_> = source
            .match_indices("billing")
            .map(|(i, v)| i..i + v.len())
            .collect();
        let mut index = SearchIndex::default();
        index.rebuild(&source, "billing");
        assert_eq!(index.len(), 2001);
        assert_eq!(index.checkpoints.len(), 8);
        for i in [0, 255, 256, 999, 1000, 1001, 1999, 2000] {
            assert_eq!(index.range(&source, i), Some(expected[i].clone()));
        }
        assert_eq!(index.range(&source, 2001), None);
        index.rebuild(&source, "tail");
        assert_eq!(index.len(), 1);
        index.rebuild(&source, "absent");
        assert!(index.is_empty());
        assert_eq!(index.visible(&source, 0, source.len()).count(), 0);
        index.rebuild(&source, "");
        assert!(index.is_empty());
    }

    #[test]
    fn preview_highlights_preserve_global_alignment_for_overlapping_patterns() {
        let source = "a".repeat(12_000);
        let mut index = SearchIndex::default();
        index.rebuild(&source, "aaa");
        let all: Vec<_> = source
            .match_indices("aaa")
            .enumerate()
            .map(|(n, (i, v))| (n, i..i + v.len()))
            .collect();
        for start in (0..source.len()).step_by(97) {
            let end = (start + 115).min(source.len());
            let expected: Vec<_> = all
                .iter()
                .filter(|(_, r)| r.end > start && r.start < end)
                .cloned()
                .collect();
            assert_eq!(
                index.visible(&source, start, end).collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn dense_search_keeps_sparse_offsets_and_supports_the_final_match() {
        let source = "x".repeat(2_000_001);
        let mut index = SearchIndex::default();
        index.rebuild(&source, "x");
        assert_eq!(index.len(), source.len());
        assert_eq!(
            index.checkpoints.len(),
            source.len().div_ceil(CHECKPOINT_STRIDE)
        );
        assert!(index.checkpoints.len() * size_of::<usize>() < 64_000);
        assert_eq!(
            index.range(&source, source.len() - 1),
            Some(source.len() - 1..source.len())
        );
    }
}
