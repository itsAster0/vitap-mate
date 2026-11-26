package vitapmate.features.login

import kotlin.test.Test
import kotlin.test.assertEquals

class GmailLinksTest {
    @Test
    fun urlsCarryTheAccountAndPage() {
        assertEquals("https://mail.google.com/mail/?authuser=a.22bce0001%40vitapstudent.ac.in#settings/fwdandpop", gmailUrl(GmailPage.Forwarding, "a.22bce0001@vitapstudent.ac.in"))
        assertEquals("https://mail.google.com/mail/u/0/#settings/filters", gmailUrl(GmailPage.Filters, null))
    }
}
