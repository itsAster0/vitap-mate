package vitapmate.data

import android.webkit.CookieManager
import vitapmate.core.SharedCookies
import vitapmate.observability.Log

/**
 * The app's one cookie store for VTOP: the web view's [CookieManager], so Rust's requests and the
 * VTOP web view are one session and nothing is copied (`set_shared_cookies`). Rust reads the
 * Cookie header here before each hop and stores each hop's Set-Cookie here, so a redirect's
 * cookies land before the next hop. The app's web view only shows VTOP, so [clear] removes every
 * cookie. Called off the main thread; a failure must not reach Rust (it would abort), so it is
 * logged, never with a value.
 */
object VtopCookies : SharedCookies {
    private val jar: CookieManager by lazy { CookieManager.getInstance() }

    override fun cookie(url: String): String? = guard("read", null) { jar.getCookie(url) }

    override fun setCookies(url: String, setCookies: List<String>) = guard("store", Unit) { setCookies.forEach { jar.setCookie(url, it) } }

    // The synchronous removal: the next hop must not see the old cookies.
    @Suppress("DEPRECATION")
    override fun clear() = guard("clear", Unit) {
        jar.removeAllCookie()
        jar.flush()
    }

    override fun flush() = guard("flush", Unit) { jar.flush() }

    private inline fun <T> guard(what: String, fallback: T, work: () -> T): T = runCatching(work).getOrElse { e ->
        Log.warn("cookies", "$what failed: ${e::class.simpleName}")
        fallback
    }
}
