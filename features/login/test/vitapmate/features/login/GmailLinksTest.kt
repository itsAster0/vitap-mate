package vitapmate.features.login

import kotlin.test.Test
import kotlin.test.assertEquals

class GmailLinksTest {
    @Test
    fun urlsCarryTheAccountAndPage() {
        assertEquals("https://mail.google.com/mail/?authuser=a.22bce0001%40vitapstudent.ac.in#settings/fwdandpop", gmailUrl(GmailPage.Forwarding, "a.22bce0001@vitapstudent.ac.in"))
        assertEquals("https://mail.google.com/mail/u/0/#settings/filters", gmailUrl(GmailPage.Filters, null))
    }

    @Test
    fun withoutTheCollegeMailGoogleAsksWhichAccount() {
        assertEquals(gmailUrl(GmailPage.Forwarding, "a@vitapstudent.ac.in"), collegeGmailUrl(GmailPage.Forwarding, "a@vitapstudent.ac.in"))
        assertEquals(
            "https://accounts.google.com/AccountChooser?service=mail&continue=https%3A%2F%2Fmail.google.com%2Fmail%2F%23settings%2Ffwdandpop",
            collegeGmailUrl(GmailPage.Forwarding, null),
        )
    }

    @Test
    fun linksNeverGoToTheGmailApp() {
        assertEquals(
            listOf("org.mozilla.firefox", "com.brave.browser"),
            browsersFor("org.mozilla.firefox", listOf("com.google.android.gm", "org.mozilla.firefox", "com.brave.browser")),
        )
        assertEquals(
            listOf("org.mozilla.firefox"),
            browsersFor("com.google.android.gm", listOf("com.google.android.gm", "com.android.chrome", "org.mozilla.firefox")),
        )
        // No default set: Android answers with its chooser; Chrome was already tried first.
        assertEquals(
            listOf("com.brave.browser"),
            browsersFor("android", listOf("android", "com.android.chrome", "com.brave.browser")),
        )
        assertEquals(emptyList(), browsersFor("com.google.android.gm", listOf("com.google.android.gm")))
        assertEquals(emptyList(), browsersFor(null, emptyList()))
    }
}
