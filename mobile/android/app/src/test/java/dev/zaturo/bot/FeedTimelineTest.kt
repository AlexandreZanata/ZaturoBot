package dev.zaturo.bot

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class FeedTimelineTest {
    private fun row(id: String, time: Long, text: String = id, sensitive: Boolean = false) =
        JSONObject().put("id", id).put("created_at", time).put("text", text)
            .put("sensitive", sensitive)

    private fun ids(timeline: FeedTimeline) = timeline.items.map { it.getString("id") }

    @Test fun orderingMatchesServerIncludingEqualTimestampIdTies() {
        val timeline = FeedTimeline()
        timeline.merge(listOf(row("a", 2), row("old", 1), row("z", 2)), false, false)
        assertEquals(listOf("z", "a", "old"), ids(timeline))
        assertEquals(listOf("old", "a", "z"), timeline.items.asReversed().map { it.getString("id") })
    }

    @Test fun sensitiveRowsAreExcludedAndDuplicateIdsUseTheFreshOccurrence() {
        val timeline = FeedTimeline()
        timeline.merge(listOf(
            row("public", 1, "first"), row("private", 3, "synthetic private data", true),
            row("public", 1, "updated"), row("", 4, "missing identity"),
            JSONObject().put("id", 123).put("created_at", 4)
        ), true, false)
        assertEquals(listOf("public"), ids(timeline))
        assertEquals("updated", timeline.items.single().getString("text"))
        assertEquals(5, timeline.nextOffset)
        assertTrue(timeline.hasMore)
    }

    @Test fun olderPageKeepsNewerRowsAndAdvancesByRawServerCountDespiteDuplicates() {
        val timeline = FeedTimeline()
        timeline.merge(listOf(row("d", 4), row("c", 3)), true, false)
        timeline.merge(listOf(
            row("c", 3), row("b", 2), row("b", 2, "updated older row"),
            row("private", 1, sensitive = true)
        ), false, true)
        assertEquals(listOf("d", "c", "b"), ids(timeline))
        assertEquals(6, timeline.nextOffset)
        assertFalse(timeline.hasMore)
        assertEquals("updated older row", timeline.items.last().getString("text"))
    }

    @Test fun overlappingPollPreservesLoadedHistoryAndItsOldestPageFlag() {
        val timeline = FeedTimeline()
        timeline.merge(listOf(row("d", 4), row("c", 3)), true, false)
        timeline.merge(listOf(row("b", 2), row("a", 1)), false, true)
        timeline.merge(listOf(row("e", 5), row("d", 4, "refreshed")), true, false)
        assertEquals(listOf("e", "d", "c", "b", "a"), ids(timeline))
        assertEquals(5, timeline.nextOffset)
        assertFalse(timeline.hasMore)
        assertEquals("refreshed", timeline.items[1].getString("text"))
        timeline.merge(listOf(row("e", 5), row("d", 4, "refreshed")), true, false)
        assertEquals(5, timeline.nextOffset)
        assertEquals(5, timeline.items.size)
    }

    @Test fun pollCoveringTheWholeCacheCanUpdateTheServerHasMoreFlag() {
        val timeline = FeedTimeline()
        timeline.merge(listOf(row("b", 2), row("a", 1)), true, false)
        timeline.merge(listOf(row("c", 3), row("b", 2), row("a", 1)), false, false)
        assertEquals(listOf("c", "b", "a"), ids(timeline))
        assertEquals(3, timeline.nextOffset)
        assertFalse(timeline.hasMore)
    }

    @Test fun nonoverlappingBurstResetsCacheRatherThanHidingAnUnknownGap() {
        val timeline = FeedTimeline()
        timeline.merge(listOf(row("b", 2), row("a", 1)), false, false)
        timeline.merge(listOf(row("z", 26), row("y", 25)), true, false)
        assertEquals(listOf("z", "y"), ids(timeline))
        assertEquals(2, timeline.nextOffset)
        assertTrue(timeline.hasMore)
    }

    @Test fun partialOverlapPreservesOlderHistoryEvenWhenTheFreshPageIsLarger() {
        val timeline = FeedTimeline()
        timeline.merge(listOf(row("c", 3), row("b", 2), row("a", 1)), false, false)
        timeline.merge(listOf(row("f", 6), row("e", 5), row("d", 4), row("c", 3)), true, false)
        assertEquals(listOf("f", "e", "d", "c", "b", "a"), ids(timeline))
        assertEquals(6, timeline.nextOffset)
        assertFalse(timeline.hasMore)
    }

    @Test fun capKeepsNewestRowsAndEmptyFirstPageOrClearResetState() {
        val timeline = FeedTimeline(3)
        timeline.merge((1..5).map { row("row-$it", it.toLong()) }, true, false)
        assertEquals(listOf("row-5", "row-4", "row-3"), ids(timeline))
        assertEquals(5, timeline.nextOffset)
        assertFalse(timeline.hasMore && timeline.items.size < 3)
        timeline.merge(listOf(row("row-2", 2), row("row-1", 1)), false, true)
        assertEquals(listOf("row-5", "row-4", "row-3"), ids(timeline))
        assertEquals(7, timeline.nextOffset)
        timeline.merge(emptyList(), false, false)
        assertTrue(timeline.items.isEmpty())
        assertEquals(0, timeline.nextOffset)
        assertFalse(timeline.hasMore)
        timeline.merge(listOf(row("new", 8)), true, false)
        timeline.clear()
        assertTrue(timeline.items.isEmpty())
        assertEquals(0, timeline.nextOffset)
        assertFalse(timeline.hasMore)
    }

    @Test fun emptyOlderPageEndsPaginationWithoutErasingVisibleHistory() {
        val timeline = FeedTimeline()
        timeline.merge(listOf(row("visible", 1)), true, false)
        timeline.merge(emptyList(), false, true)
        assertEquals(listOf("visible"), ids(timeline))
        assertEquals(1, timeline.nextOffset)
        assertFalse(timeline.hasMore)
    }
}
