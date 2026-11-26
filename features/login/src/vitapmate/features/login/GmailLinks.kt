package vitapmate.features.login

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import java.net.URLEncoder

internal enum class GmailPage(val fragment: String) {
    Forwarding("settings/fwdandpop"),
    Filters("settings/filters"),
}

private fun encode(s: String): String = URLEncoder.encode(s, "UTF-8")

private fun gmailBase(account: String?): String =
    if (account.isNullOrBlank()) "https://mail.google.com/mail/u/0/"
    else "https://mail.google.com/mail/?authuser=${encode(account)}"

internal fun gmailUrl(page: GmailPage, account: String?): String =
    "${gmailBase(account)}#${page.fragment}"

/**
 * Opens [url] in Chrome (the Gmail app has no forwarding settings and would grab the link),
 * else any browser; true when one opened. With none, the link is copied for the student instead.
 */
internal fun openInChrome(context: Context, url: String): Boolean {
    val uri = Uri.parse(url)
    for (intent in listOf(Intent(Intent.ACTION_VIEW, uri).setPackage("com.android.chrome"), Intent(Intent.ACTION_VIEW, uri))) {
        try {
            context.startActivity(intent)
            return true
        } catch (_: ActivityNotFoundException) {
            // Try the next: Chrome may be off or missing.
        }
    }
    copy(context, "Link (no browser found)", url)
    return false
}
