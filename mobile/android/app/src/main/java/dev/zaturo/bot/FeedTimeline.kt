package dev.zaturo.bot

import org.json.JSONObject

/** Bounded newest-first cache. The messenger renders [items] in reverse order. */
internal class FeedTimeline(private val maxItems: Int = 200) {
    init {
        require(maxItems in 1..200)
    }

    private var cached: List<JSONObject> = emptyList()
    val items: List<JSONObject> get() = cached
    var hasMore: Boolean = false
        private set
    var nextOffset: Int = 0
        private set

    fun clear() {
        cached = emptyList()
        hasMore = false
        nextOffset = 0
    }

    fun merge(page: List<JSONObject>, more: Boolean, append: Boolean) {
        if (!append && page.isEmpty()) {
            clear()
            return
        }
        val incoming = safeRows(page)
        val previousIds = cached.mapTo(HashSet()) { it.getString("id") }
        val incomingIds = incoming.keys

        if (append) {
            cached = ordered(safeRows(cached).apply { putAll(incoming) }.values)
            nextOffset = addOffset(nextOffset, page.size)
            hasMore = more
            return
        }

        if (cached.isEmpty() || incomingIds.none { it in previousIds }) {
            // A burst with no overlap leaves an unknown gap; replace rather than imply continuity.
            cached = ordered(incoming.values)
            nextOffset = page.size
            hasMore = more
            return
        }

        val previousSize = cached.size
        cached = ordered(safeRows(cached).apply { putAll(incoming) }.values)
        nextOffset = addOffset(nextOffset, incomingIds.count { it !in previousIds })
        // A refresh cannot speak for the oldest page already loaded beyond its own range.
        if (previousSize <= page.size && incomingIds.containsAll(previousIds)) {
            hasMore = more
        }
    }

    private fun safeRows(rows: List<JSONObject>): LinkedHashMap<String, JSONObject> {
        val result = LinkedHashMap<String, JSONObject>()
        for (row in rows) {
            val id = row.opt("id") as? String ?: continue
            if (!row.optBoolean("sensitive", false) && id.isNotEmpty()) {
                result[id] = row
            }
        }
        return result
    }

    private fun ordered(rows: Collection<JSONObject>): List<JSONObject> = rows.sortedWith(
        compareByDescending<JSONObject> { it.optLong("created_at") }
            .thenByDescending { it.getString("id") }
    ).take(maxItems)

    private fun addOffset(previous: Int, count: Int): Int =
        (previous.toLong() + count).coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
}
