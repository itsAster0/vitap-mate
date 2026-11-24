package vitapmate.data

import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import vitapmate.core.CookieAnswer
import vitapmate.core.CookieKind
import vitapmate.observability.Unattended

class CookieConsentTest {
    @AfterTest
    fun cleanUp() {
        Foreground.stopped()
    }

    @Test
    fun unattendedFlowIsNotAttended() {
        Foreground.started()
        assertTrue(CookieConsent.attended("f"))
        // As `flow(name, unattended = true)` marks its id while it runs (androidx.tracing needs a device).
        Unattended.ids += "f"
        try {
            assertFalse(CookieConsent.attended("f"))
        } finally {
            Unattended.ids -= "f"
        }
    }

    @Test
    fun notShownIsNotAttended() {
        Foreground.started()
        Foreground.stopped()
        assertFalse(CookieConsent.attended("f"))
    }

    @Test
    fun twoAsksOfOneActionShareOneDialog() = runBlocking {
        Foreground.started()
        val first = async { CookieConsent.ask("f1", CookieKind.ROUTE, "Fetch attendance", "vmate.aster0.dev") }
        val second = async { CookieConsent.ask("f2", CookieKind.ROUTE, "Fetch attendance", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        yield()
        assertEquals(1, CookieConsent.pending.value.size)
        val yes = CookieAnswer.Allow(remember = false)
        CookieConsent.answer(CookieConsent.pending.value.single(), yes)
        assertEquals(yes, first.await())
        assertEquals(yes, second.await())
        assertTrue(CookieConsent.pending.value.isEmpty())
    }

    @Test
    fun timeoutAnswersCancel() = runBlocking {
        val wait = CookieConsent.waitMs
        CookieConsent.waitMs = { 10L }
        try {
            val answer = CookieConsent.ask("f", CookieKind.ACCOUNT, "Load connected apps", "vmate.aster0.dev")
            assertEquals(CookieAnswer.Deny, answer)
            assertTrue(CookieConsent.pending.value.isEmpty())
        } finally {
            CookieConsent.waitMs = wait
        }
    }

    @Test
    fun leavingTheAppAnswersCancel() = runBlocking {
        Foreground.started()
        val asked = async { CookieConsent.ask("f", CookieKind.HANDOVER, "Give a connected app your VTOP session", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        Foreground.stopped()
        assertEquals(CookieAnswer.Deny, asked.await())
        assertTrue(CookieConsent.pending.value.isEmpty())
    }

    @Test
    fun anotherActionNeverRidesOnAShownDialog() = runBlocking {
        Foreground.started()
        val load = async { CookieConsent.ask("f1", CookieKind.ACCOUNT, "Load your connected apps", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        val delete = async { CookieConsent.ask("f2", CookieKind.ACCOUNT, "Delete your account on the server", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.size < 2) yield()
        val yes = CookieAnswer.Allow(remember = false)
        CookieConsent.answer(CookieConsent.pending.value.first(), yes)
        assertEquals(yes, load.await())
        assertEquals("Delete your account on the server", CookieConsent.pending.value.single().action)
        CookieConsent.answer(CookieConsent.pending.value.single(), CookieAnswer.Deny)
        assertEquals(CookieAnswer.Deny, delete.await())
    }

    @Test
    fun cancelledAskTakesItsDialogAway() = runBlocking {
        Foreground.started()
        val asked = launch { CookieConsent.ask("f", CookieKind.ACCOUNT, "Load your connected apps", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        asked.cancelAndJoin()
        assertTrue(CookieConsent.pending.value.isEmpty())
    }

    @Test
    fun cancelledAskKeepsADialogOthersWaitOn() = runBlocking {
        Foreground.started()
        val first = launch { CookieConsent.ask("f1", CookieKind.ROUTE, "Fetch attendance", "vmate.aster0.dev") }
        val second = async { CookieConsent.ask("f2", CookieKind.ROUTE, "Fetch attendance", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        yield()
        first.cancelAndJoin()
        val yes = CookieAnswer.Allow(remember = false)
        CookieConsent.answer(CookieConsent.pending.value.single(), yes)
        assertEquals(yes, second.await())
    }

    @Test
    fun askWhileNotShownAnswersCancelAtOnce() = runBlocking {
        val answer = CookieConsent.ask("f", CookieKind.ACCOUNT, "Load your connected apps", "vmate.aster0.dev")
        assertEquals(CookieAnswer.Deny, answer)
        assertTrue(CookieConsent.pending.value.isEmpty())
    }
}
