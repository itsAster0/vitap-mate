package vitapmate.features.vtop

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

/**
 * Recently opened VTOP pages, newest first, for one account (the Flutter app's
 * `VtopRecentPages`: same key, same JSON, 6 kept); another account's list reads as empty. Sign-out
 * removes the key.
 */
internal object VtopRecent {
    const val KEY = "settings_vtop_recent_pages"
    private const val MAX = 6

    fun read(context: Context, user: String?): List<VtopPage> = runCatching {
        if (user == null) return emptyList()
        val stored = JSONObject(prefs(context).getString(KEY, null) ?: return emptyList())
        if (stored.optString("user") != user.uppercase()) return emptyList()
        val pages = stored.getJSONArray("pages")
        (0 until pages.length()).map { pages.getJSONObject(it) }
            .map { VtopPage(it.optString("url"), it.optString("title"), it.optString("section")) }
            .filter { it.url.isNotEmpty() && it.title.isNotEmpty() }
    }.getOrDefault(emptyList())

    /** Puts [page] first; returns the new list. */
    fun add(context: Context, user: String?, page: VtopPage): List<VtopPage> {
        if (user == null || page.url.isEmpty()) return read(context, user)
        val pages = (listOf(page) + read(context, user).filter { it.url != page.url }).take(MAX)
        val json = JSONArray(pages.map { JSONObject().put("url", it.url).put("title", it.title).put("section", it.section) })
        prefs(context).edit().putString(KEY, JSONObject().put("user", user.uppercase()).put("pages", json).toString()).apply()
        return pages
    }

    private fun prefs(context: Context) = context.getSharedPreferences("settings", Context.MODE_PRIVATE)
}
