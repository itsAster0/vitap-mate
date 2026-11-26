package vitapmate.features.login

import org.json.JSONObject

/** A note in a [tone] ("lab", "warning", ...) with an [icon] ("mail-check", ...), as the edge names them. */
internal data class GuideNote(val icon: String, val tone: String, val text: String)

/** One guide step: what to do, maybe a [note], then Gmail's screen for it (the student's address blurred). */
internal data class GuidePage(
    val step: Int?,
    val title: String,
    val body: String,
    val note: GuideNote?,
    val shot: String?,
    val caption: String?,
)

/** Pages tagged for [step], in guide order; untagged blocks from older guides are skipped. */
internal fun pagesFor(step: Int, pages: List<GuidePage>): List<GuidePage> =
    pages.filter { it.step == step }

/** [pagesFor], or every untagged page when none is tagged [step]: a cached guide from before steps. */
internal fun pagesOrAll(step: Int, pages: List<GuidePage>): List<GuidePage> =
    pagesFor(step, pages).ifEmpty { pages.filter { it.step == null } }

private fun JSONObject.note() = GuideNote(getString("icon"), getString("tone"), getString("text"))

private fun JSONObject.string(key: String) = optString(key).takeIf { has(key) && !isNull(key) }

private fun JSONObject.int(key: String) = (opt(key) as? Number)?.toInt()

/** Parses `guides/forwarding.json` step blocks and substitutes `{sender}` with VTOP's OTP sender. */
internal fun guidePages(json: JSONObject, sender: String): List<GuidePage> {
    val blocks = json.getJSONArray("blocks")
    // Block types this app doesn't know (and non-step callouts) are skipped.
    return (0 until blocks.length()).map(blocks::getJSONObject).mapNotNull { b ->
        if (b.getString("type") != "step") return@mapNotNull null
        GuidePage(
            step = b.int("step"),
            title = b.getString("title"),
            body = b.getString("body").replace("{sender}", sender),
            note = b.optJSONObject("callout")?.note(),
            shot = b.string("image"),
            caption = b.string("caption"),
        )
    }
}
