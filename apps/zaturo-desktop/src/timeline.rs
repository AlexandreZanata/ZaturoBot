//! Bounded newest-first feed state. Viewport intent stays in the presentation layer.
use crate::api::FeedItem;
use std::collections::{BTreeMap, HashSet};

pub const MAX_ITEMS: usize = 200;
pub const MAX_OFFSET: u32 = 1_000_000;

#[derive(Default)]
pub struct FeedTimeline {
    items: Vec<FeedItem>,
    next_offset: u32,
    has_more: bool,
}

impl FeedTimeline {
    pub fn clear(&mut self) {
        self.items.clear();
        self.next_offset = 0;
        self.has_more = false;
    }

    pub fn items(&self) -> &[FeedItem] {
        &self.items
    }

    pub fn next_offset(&self) -> u32 {
        self.next_offset
    }

    pub fn has_more(&self) -> bool {
        self.has_more
    }

    /// Merge a raw server page; return whether visible rows or the older-page flag changed.
    pub fn merge(&mut self, page: Vec<FeedItem>, more: bool, append: bool) -> bool {
        let received_count = count(page.len());
        self.merge_counted(page, received_count, more, append)
    }

    /// Preserve raw offsets when the HTTP adapter has already removed sensitive rows.
    pub fn merge_counted(
        &mut self,
        page: Vec<FeedItem>,
        received_count: u32,
        more: bool,
        append: bool,
    ) -> bool {
        let received_count = received_count.max(count(page.len())).min(MAX_OFFSET);
        if !append && received_count == 0 {
            let changed = !self.items.is_empty() || self.has_more;
            self.clear();
            return changed;
        }
        let incoming = safe_rows(page);
        let previous_ids = self
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<HashSet<_>>();
        let overlaps = incoming.keys().any(|id| previous_ids.contains(id.as_str()));
        let new_count = count(
            incoming
                .keys()
                .filter(|id| !previous_ids.contains(id.as_str()))
                .count(),
        );
        let covers_cache = self.items.len() <= received_count as usize
            && previous_ids.iter().all(|id| incoming.contains_key(*id));
        let preserve_cache = append || (!self.items.is_empty() && overlaps);
        let next_offset = if append {
            self.next_offset.saturating_add(received_count)
        } else if preserve_cache {
            self.next_offset.saturating_add(new_count)
        } else {
            received_count
        }
        .min(MAX_OFFSET);
        let has_more = if append || !preserve_cache || covers_cache {
            more
        } else {
            // The new leading page cannot describe the oldest page already loaded.
            self.has_more
        };
        let mut combined = if preserve_cache {
            safe_rows(self.items.iter().cloned())
        } else {
            BTreeMap::new()
        };
        combined.extend(incoming);
        let mut items = combined.into_values().collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        items.truncate(MAX_ITEMS);
        let changed = self.items != items || self.has_more != has_more;
        self.items = items;
        self.next_offset = next_offset;
        self.has_more = has_more;
        changed
    }
}

fn safe_rows(rows: impl IntoIterator<Item = FeedItem>) -> BTreeMap<String, FeedItem> {
    rows.into_iter()
        .filter(|item| !item.sensitive && !item.id.is_empty())
        .map(|item| (item.id.clone(), item))
        .collect()
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(MAX_OFFSET).min(MAX_OFFSET)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, created_at: i64) -> FeedItem {
        FeedItem {
            id: id.into(),
            created_at,
            kind: "chat".into(),
            role: "assistant".into(),
            text: format!("Synthetic message {id}"),
            task_id: None,
            artifact_id: None,
            sensitive: false,
        }
    }

    fn ids(timeline: &FeedTimeline) -> Vec<&str> {
        timeline.items().iter().map(|row| row.id.as_str()).collect()
    }

    #[test]
    fn canonical_order_matches_timestamp_and_id_descending() {
        let mut timeline = FeedTimeline::default();
        assert!(timeline.merge(
            vec![item("a", 2), item("old", 1), item("z", 2)],
            false,
            false
        ));
        assert_eq!(ids(&timeline), ["z", "a", "old"]);
        assert_eq!(
            timeline
                .items()
                .iter()
                .rev()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["old", "a", "z"]
        );
    }

    #[test]
    fn raw_pages_hide_sensitive_rows_and_keep_latest_payload_per_id() {
        let mut timeline = FeedTimeline::default();
        let mut private = item("private", 10);
        private.sensitive = true;
        let mut updated = item("public", 2);
        updated.text = "Updated public metadata".into();
        timeline.merge(
            vec![item("public", 2), private, updated, item("", 1)],
            true,
            false,
        );
        assert_eq!(ids(&timeline), ["public"]);
        assert_eq!(timeline.items()[0].text, "Updated public metadata");
        assert_eq!(timeline.next_offset(), 4);
    }

    #[test]
    fn already_filtered_http_pages_still_advance_by_received_count() {
        let mut timeline = FeedTimeline::default();
        timeline.merge_counted(vec![item("visible", 2)], 20, true, false);
        assert_eq!(timeline.next_offset(), 20);
        assert!(!timeline.merge_counted(Vec::new(), 20, true, true));
        assert_eq!(ids(&timeline), ["visible"]);
        assert_eq!(timeline.next_offset(), 40);
        assert!(timeline.has_more());
        let mut private_only = FeedTimeline::default();
        assert!(private_only.merge_counted(Vec::new(), 20, true, false));
        assert!(private_only.items().is_empty());
        assert_eq!(private_only.next_offset(), 20);
        assert!(private_only.has_more());
        assert!(private_only.merge_counted(Vec::new(), 0, false, false));
        assert_eq!(private_only.next_offset(), 0);
        assert!(!private_only.has_more());
    }

    #[test]
    fn older_duplicate_pages_keep_newer_rows_and_use_raw_offset() {
        let mut timeline = FeedTimeline::default();
        timeline.merge(vec![item("d", 4), item("c", 3)], true, false);
        timeline.merge(vec![item("c", 3), item("b", 2), item("b", 2)], false, true);
        assert_eq!(ids(&timeline), ["d", "c", "b"]);
        assert_eq!(timeline.next_offset(), 5);
        assert!(!timeline.has_more());
    }

    #[test]
    fn overlapping_poll_preserves_loaded_history_and_oldest_page_state() {
        let mut timeline = FeedTimeline::default();
        timeline.merge(vec![item("d", 4), item("c", 3)], true, false);
        timeline.merge(vec![item("b", 2), item("a", 1)], false, true);
        let newest = vec![item("e", 5), item("d", 4)];
        assert!(timeline.merge(newest.clone(), true, false));
        assert_eq!(ids(&timeline), ["e", "d", "c", "b", "a"]);
        assert_eq!(timeline.next_offset(), 5);
        assert!(!timeline.has_more());
        assert!(!timeline.merge(newest, true, false));
        assert_eq!(timeline.next_offset(), 5);
    }

    #[test]
    fn a_larger_partial_overlap_cannot_replace_the_oldest_page_flag() {
        let mut timeline = FeedTimeline::default();
        timeline.merge(vec![item("c", 3), item("b", 2), item("a", 1)], false, false);
        timeline.merge(
            vec![item("f", 6), item("e", 5), item("d", 4), item("c", 3)],
            true,
            false,
        );
        assert_eq!(ids(&timeline), ["f", "e", "d", "c", "b", "a"]);
        assert_eq!(timeline.next_offset(), 6);
        assert!(!timeline.has_more());
    }

    #[test]
    fn a_fresh_page_covering_the_whole_cache_updates_has_more() {
        let mut timeline = FeedTimeline::default();
        timeline.merge(vec![item("b", 2), item("a", 1)], true, false);
        timeline.merge(vec![item("c", 3), item("b", 2), item("a", 1)], false, false);
        assert_eq!(ids(&timeline), ["c", "b", "a"]);
        assert_eq!(timeline.next_offset(), 3);
        assert!(!timeline.has_more());
    }

    #[test]
    fn a_nonoverlapping_burst_resets_instead_of_implying_a_continuous_history() {
        let mut timeline = FeedTimeline::default();
        timeline.merge(vec![item("b", 2), item("a", 1)], false, false);
        timeline.merge(vec![item("z", 26), item("y", 25)], true, false);
        assert_eq!(ids(&timeline), ["z", "y"]);
        assert_eq!(timeline.next_offset(), 2);
        assert!(timeline.has_more());
    }

    #[test]
    fn the_cache_caps_at_two_hundred_newest_rows_and_offsets_stay_bounded() {
        let mut timeline = FeedTimeline::default();
        timeline.merge(
            (0..205).map(|n| item(&format!("item-{n}"), n)).collect(),
            true,
            false,
        );
        assert_eq!(timeline.items().len(), MAX_ITEMS);
        assert_eq!(timeline.items().first().unwrap().created_at, 204);
        assert_eq!(timeline.items().last().unwrap().created_at, 5);
        assert_eq!(timeline.next_offset(), 205);
        timeline.merge_counted(Vec::new(), u32::MAX, true, true);
        assert_eq!(timeline.next_offset(), MAX_OFFSET);
    }

    #[test]
    fn empty_older_page_preserves_rows_but_empty_first_page_or_clear_resets() {
        let mut timeline = FeedTimeline::default();
        timeline.merge(vec![item("visible", 1)], true, false);
        assert!(timeline.merge(Vec::new(), false, true));
        assert_eq!(ids(&timeline), ["visible"]);
        assert_eq!(timeline.next_offset(), 1);
        assert!(!timeline.has_more());
        assert!(timeline.merge(Vec::new(), false, false));
        assert!(timeline.items().is_empty());
        assert_eq!(timeline.next_offset(), 0);
        assert!(!timeline.has_more());
        timeline.merge(vec![item("other", 2)], true, false);
        timeline.clear();
        assert!(timeline.items().is_empty());
        assert_eq!(timeline.next_offset(), 0);
        assert!(!timeline.has_more());
    }
}
