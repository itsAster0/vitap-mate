package vitapmate.features.login

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import java.net.URLEncoder

internal enum class GmailPage(val fragment: String) {
    Forwarding("settings/fwdandpop"),
    Filters("settings/filters"),
}

private const val CHROME = "com.android.chrome"

/** The Gmail app: it has no forwarding or filter settings, so links never go to it. */
private const val GMAIL_APP = "com.google.android.gm"

/** What Android answers for "the default browser" when there is none: its app chooser. */
private const val CHOOSER = "android"

private fun encode(s: String): String = URLEncoder.encode(s, "UTF-8")

private fun gmailBase(account: String?): String =
    if (account.isNullOrBlank()) "https://mail.google.com/mail/u/0/"
    else "https://mail.google.com/mail/?authuser=${encode(account)}"

internal fun gmailUrl(page: GmailPage, account: String?): String =
    "${gmailBase(account)}#${page.fragment}"

/**
 * [page] in the college Gmail: under [collegeMail] when known, else through Google's account
 * chooser, since `/u/0/` is usually the personal Gmail that was the wrong one.
 */
internal fun collegeGmailUrl(page: GmailPage, collegeMail: String?): String =
    if (!collegeMail.isNullOrBlank()) gmailUrl(page, collegeMail)
    else "https://accounts.google.com/AccountChooser?service=mail&continue=${encode("https://mail.google.com/mail/#${page.fragment}")}"

/**
 * Browsers to try after Chrome for Gmail's web pages, given Android's [default] for an https link
 * and every app that [handles] one: the default first, then other browsers, never Chrome (already
 * tried), the chooser, or the Gmail app.
 */
internal fun browsersFor(default: String?, handles: List<String>): List<String> =
    (listOfNotNull(default) + handles).filter { it != CHROME && it != CHOOSER && it != GMAIL_APP }.distinct()

/**
 * Opens [url] in Chrome, else the default browser (never the Gmail app: it has no forwarding
 * settings and would grab the link); true when one opened. With none, the link is copied for the
 * student instead.
 */
internal fun openInChrome(context: Context, url: String): Boolean {
    val uri = Uri.parse(url)
    fun open(pkg: String): Boolean = try {
        context.startActivity(Intent(Intent.ACTION_VIEW, uri).addCategory(Intent.CATEGORY_BROWSABLE).setPackage(pkg))
        true
    } catch (_: ActivityNotFoundException) {
        // Try the next: the browser may be off or missing.
        false
    } catch (_: SecurityException) {
        // A browser that doesn't export its view activity to us: try the next.
        false
    }
    if (open(CHROME)) return true
    val browsable = Intent(Intent.ACTION_VIEW, Uri.parse("https://example.com")).addCategory(Intent.CATEGORY_BROWSABLE)
    val pm = context.packageManager
    // Needs the https <queries> entry in the manifest.
    val default = pm.resolveActivity(browsable, PackageManager.MATCH_DEFAULT_ONLY)?.activityInfo?.packageName
    val handles = pm.queryIntentActivities(browsable, PackageManager.MATCH_ALL).mapNotNull { it.activityInfo?.packageName }
    if (browsersFor(default, handles).any(::open)) return true
    copy(context, "Link (no browser found)", url)
    return false
}
