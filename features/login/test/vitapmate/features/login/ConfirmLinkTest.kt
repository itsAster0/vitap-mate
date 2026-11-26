package vitapmate.features.login

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import vitapmate.core.ConfirmBy
import vitapmate.core.ConfirmFrom
import vitapmate.core.GmailConfirmation

class ConfirmLinkTest {
    private fun mail(by: ConfirmBy) = GmailConfirmation(by, "a@vitapstudent.ac.in", 5L, ConfirmFrom.COLLEGE_INBOX)

    @Test
    fun theLinkStaysToOpenAgainWhileTheMailIsKept() {
        val link = "https://mail-settings.google.com/mail/vf-x"
        assertEquals(link, mail(ConfirmBy.Link(link)).link())
        assertEquals(link, mail(ConfirmBy.CodeAndLink("1", link)).link())
        assertNull(mail(ConfirmBy.Code("1")).link())
        assertNull(mail(ConfirmBy.Neither).link())
        assertNull((null as GmailConfirmation?)?.link())
    }
}
